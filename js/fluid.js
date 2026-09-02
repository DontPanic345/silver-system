// Stable Fluids — Jos Stam, "Real-Time Fluid Dynamics for Games" (2003).
//
// A grid-based (Eulerian) solver. Velocity and a passive dye ("density") live
// on a grid; each step advects them along the flow and then projects the
// velocity field back to divergence-free with a Jacobi pressure solve. That
// projection is what makes it unconditionally stable — it never blows up
// regardless of timestep. Velocity is advected semi-Lagrangian; the passive
// scalar uses the conservative MUSCL flux scheme in `advectScalar`.
//
// Pure module: no DOM, no rendering. `js/main.js` drives it on screen;
// `test/fluid-probe.js` drives it headless.

const ix = (SIZE, i, j) => i + SIZE * j;

export const createFluid = (N, opts = {}) => {
  const SIZE = N + 2; // interior N×N plus a one-cell boundary ring
  const cell = () => new Float32Array(SIZE * SIZE);
  const f = {
    N,
    SIZE,
    dt: opts.dt ?? 0.12,
    iter: opts.iter ?? 24, // Jacobi sweeps per linear solve
    visc: opts.visc ?? 0,
    diff: opts.diff ?? 0,
    kappa: opts.kappa ?? 0, // thermal diffusivity for conduction (temp channel)
    fade: opts.fade ?? 0, // fraction of dye removed each step (0 = conserve)
    buoyancy: opts.buoyancy ?? 0, // thermal buoyancy coefficient (0 = off)
    u: cell(), v: cell(), uPrev: cell(), vPrev: cell(),
    dens: cell(), densPrev: cell(),
    temp: cell(), tempPrev: cell(), // temperature field + in-place-swapped scratch
    // Water mixture: three phase fractions per cell that sum to `capacity`. All
    // three ride the shared (u, v) flow with the same conservative MUSCL scheme
    // as dens/temp; `air` is explicit so vapour displaces it rather than
    // appearing from nowhere. There is ONE velocity field — no momentum or
    // settling fields this round.
    liquid: cell(), liquidPrev: cell(),
    vapour: cell(), vapourPrev: cell(),
    air: cell(), airPrev: cell(),
    tmp: cell(),  // grid-sized scratch: Jacobi sweeps, and the advected-scalar working buffer
    flux: cell(), // grid-sized scratch: per-face flux buffer for conservative scalar advection
  };
  f.capacity = opts.capacity ?? 1.0;
  f.phaseChange = opts.phaseChange ?? true;
  f.latentHeat = opts.latentHeat ?? 0;
  f.boilTemp = opts.boilTemp ?? Infinity;
  f.condenseTemp = opts.condenseTemp ?? -Infinity;
  const heatOpt = opts.heat ?? 0;
  f.heat = typeof heatOpt === 'function'
    ? heatOpt
    : (heatOpt === 0 ? null : () => heatOpt);
  // Displayable scalar field channels, in render order. This is the explicit
  // registry the page iterates (AC 16): adding a scalar field to the solver means
  // adding its name here and a renderer in js/main.js — nothing downstream guesses
  // from buffer shapes, so scratch/vector buffers can never leak into the view.
  f.channels = ['dens', 'temp', 'liquid', 'vapour'];
  // Seed the temperature interior. temp0 is a uniform Number or an (i, j) => value
  // callback over 1-indexed interior cells; the boundary ring stays zero and is
  // refreshed as a zero-gradient copy by setBnd on the first step.
  const temp0 = opts.temp0 ?? 0;
  const t0 = typeof temp0 === 'function' ? temp0 : () => temp0;
  for (let j = 1; j <= N; j++) {
    for (let i = 1; i <= N; i++) f.temp[ix(SIZE, i, j)] = t0(i, j);
  }
  // Seed the water mixture. water0 returns { liquid, vapour } for a 1-indexed
  // interior cell; air fills the rest of the capacity. Default: all air.
  const w0 = typeof opts.water0 === 'function' ? opts.water0 : null;
  for (let j = 1; j <= N; j++) {
    for (let i = 1; i <= N; i++) {
      const k = ix(SIZE, i, j);
      let L = 0, V = 0;
      if (w0) { const r = w0(i, j) || {}; L = r.liquid || 0; V = r.vapour || 0; }
      f.liquid[k] = L;
      f.vapour[k] = V;
      f.air[k] = f.capacity - L - V;
    }
  }
  setBnd(f, 0, f.liquid);
  setBnd(f, 0, f.vapour);
  setBnd(f, 0, f.air);
  return f;
};

// b: which field we're bounding — 1 = horizontal velocity (flips sign at the
// left/right walls), 2 = vertical velocity (flips at top/bottom), 0 = scalar.
const setBnd = (f, b, x) => {
  const { N, SIZE } = f;
  for (let i = 1; i <= N; i++) {
    x[ix(SIZE, 0, i)]     = b === 1 ? -x[ix(SIZE, 1, i)] : x[ix(SIZE, 1, i)];
    x[ix(SIZE, N + 1, i)] = b === 1 ? -x[ix(SIZE, N, i)] : x[ix(SIZE, N, i)];
    x[ix(SIZE, i, 0)]     = b === 2 ? -x[ix(SIZE, i, 1)] : x[ix(SIZE, i, 1)];
    x[ix(SIZE, i, N + 1)] = b === 2 ? -x[ix(SIZE, i, N)] : x[ix(SIZE, i, N)];
  }
  x[ix(SIZE, 0, 0)]         = 0.5 * (x[ix(SIZE, 1, 0)] + x[ix(SIZE, 0, 1)]);
  x[ix(SIZE, 0, N + 1)]     = 0.5 * (x[ix(SIZE, 1, N + 1)] + x[ix(SIZE, 0, N)]);
  x[ix(SIZE, N + 1, 0)]     = 0.5 * (x[ix(SIZE, N, 0)] + x[ix(SIZE, N + 1, 1)]);
  x[ix(SIZE, N + 1, N + 1)] = 0.5 * (x[ix(SIZE, N, N + 1)] + x[ix(SIZE, N + 1, N)]);
};

// Jacobi iteration (not Gauss-Seidel): every cell is updated from the previous
// sweep's values, held in `tmp`, then the whole interior is copied back. It
// converges more slowly per sweep than Gauss-Seidel, but it is completely
// order-independent — so the result is exactly reflection-symmetric, bit-for-bit
// deterministic, and each sweep is embarrassingly parallel, which is how a GPU
// port would want it.
const linSolve = (f, b, x, x0, a, c) => {
  const { N, SIZE, iter, tmp } = f;
  const invC = 1 / c;
  for (let k = 0; k < iter; k++) {
    for (let j = 1; j <= N; j++) {
      for (let i = 1; i <= N; i++) {
        tmp[ix(SIZE, i, j)] = (x0[ix(SIZE, i, j)] + a * (
          x[ix(SIZE, i - 1, j)] + x[ix(SIZE, i + 1, j)] +
          x[ix(SIZE, i, j - 1)] + x[ix(SIZE, i, j + 1)]
        )) * invC;
      }
    }
    for (let j = 1; j <= N; j++) {
      for (let i = 1; i <= N; i++) x[ix(SIZE, i, j)] = tmp[ix(SIZE, i, j)];
    }
    setBnd(f, b, x);
  }
};

const diffuse = (f, b, x, x0, amount) => {
  const a = f.dt * amount * f.N * f.N;
  linSolve(f, b, x, x0, a, 1 + 4 * a);
};

// Semi-Lagrangian advection: backtrace each cell along the flow and sample the
// old field there (bilinear). Used for the velocity components. The passive
// scalar uses the conservative flux scheme below instead.
const advect = (f, b, d, d0, velU, velV) => {
  const { N, SIZE } = f;
  const dt0 = f.dt * N;
  for (let j = 1; j <= N; j++) {
    for (let i = 1; i <= N; i++) {
      let x = i - dt0 * velU[ix(SIZE, i, j)];
      let y = j - dt0 * velV[ix(SIZE, i, j)];
      if (x < 0.5) x = 0.5; else if (x > N + 0.5) x = N + 0.5;
      if (y < 0.5) y = 0.5; else if (y > N + 0.5) y = N + 0.5;
      const i0 = x | 0, i1 = i0 + 1;
      const j0 = y | 0, j1 = j0 + 1;
      const s1 = x - i0, s0 = 1 - s1;
      const t1 = y - j0, t0 = 1 - t1;
      d[ix(SIZE, i, j)] =
        s0 * (t0 * d0[ix(SIZE, i0, j0)] + t1 * d0[ix(SIZE, i0, j1)]) +
        s1 * (t0 * d0[ix(SIZE, i1, j0)] + t1 * d0[ix(SIZE, i1, j1)]);
    }
  }
  setBnd(f, b, d);
};

// minmod slope limiter: zero if the neighbouring differences disagree in sign,
// otherwise the smaller in magnitude. This is what makes the flux scheme below
// monotone — it never reconstructs a face value outside the local data range.
const minmod = (a, c) => (a * c <= 0 ? 0 : (Math.abs(a) < Math.abs(c) ? a : c));

// One conservative 1D MUSCL sweep. `stride` steps between adjacent cells along
// the sweep axis (1 for x, SIZE for y). For each interior face we reconstruct a
// limited upwind face value, form the flux a·phi_face, and update the two cells
// it separates by ∓h·flux. Wall faces carry no flux (wall-normal velocity is
// zero), so the interior total is exactly preserved — whatever leaves one cell
// enters its neighbour.
const muscl1D = (f, out, cur, vel, stride, h) => {
  const { N, SIZE, flux: fbuf } = f;
  // Pass 1: the h-scaled flux through every interior face, stored at the index
  // of the cell on its low side. Wall faces (a = 0, a = N) stay zero.
  for (let p = 1; p <= N; p++) {
    const base = stride === 1 ? SIZE * p : p;
    fbuf[base] = 0;
    fbuf[base + N * stride] = 0;
    for (let a = 1; a <= N - 1; a++) {
      const k = base + a * stride;            // cell a
      const kR = k + stride;                  // cell a+1
      const vface = 0.5 * (vel[k] + vel[kR]);
      const cr = vface * h;                   // Courant number across this face
      let face;
      if (vface >= 0) {
        const sl = minmod(cur[k] - cur[k - stride], cur[kR] - cur[k]);
        face = cur[k] + 0.5 * (1 - cr) * sl;
      } else {
        const sl = minmod(cur[kR] - cur[k], cur[kR + stride] - cur[kR]);
        face = cur[kR] - 0.5 * (1 + cr) * sl;
      }
      fbuf[k] = h * vface * face;
    }
  }
  // Pass 2: each cell as one expression — cur minus its net outflux. Written as
  // a single subtraction so an x-mirror-symmetric input stays bit-symmetric.
  for (let p = 1; p <= N; p++) {
    const base = stride === 1 ? SIZE * p : p;
    for (let a = 1; a <= N; a++) {
      const k = base + a * stride;
      out[k] = cur[k] - (fbuf[k] - fbuf[k - stride]);
    }
  }
  setBnd(f, 0, out);
};

// Conservative flux-form advection for the passive scalar. Plain semi-Lagrangian
// advection is strongly dissipative — over a long closed run it bleeds a large
// fraction of the total scalar into numerical diffusion (7–26% over 500 steps).
// This scheme moves scalar as fluxes between cells instead, so a closed box
// conserves the interior total to machine precision, and the minmod limiter
// keeps it monotone (AC 2: no new maxima, nothing negative). Dimensionally
// split (x then y) and sub-cycled whenever the Courant number exceeds 1, which
// is what keeps it stable at the large dt the stability probe uses.
const advectScalar = (f, d, d0, velU, velV) => {
  const { N, SIZE } = f;
  const dt0 = f.dt * N;

  let maxC = 0;
  for (let j = 1; j <= N; j++) {
    for (let i = 1; i <= N; i++) {
      const k = ix(SIZE, i, j);
      const c = Math.abs(velU[k]) + Math.abs(velV[k]);
      if (c > maxC) maxC = c;
    }
  }
  const sub = Math.max(1, Math.ceil(maxC * dt0 + 1e-9));
  const h = dt0 / sub;

  const cur = d;          // running field (output buffer)
  const nxt = f.tmp;      // Jacobi scratch — no linear solve runs during a scalar advect
  cur.set(d0);
  for (let s = 0; s < sub; s++) {
    muscl1D(f, nxt, cur, velU, 1, h);
    cur.set(nxt);
    muscl1D(f, nxt, cur, velV, SIZE, h);
    cur.set(nxt);
  }

  // Godunov splitting can leave a sub-milli overshoot the 1D limiter alone
  // doesn't catch. Clamp the interior to the global range the field held before
  // the step (exactly AC 2's bound) and hand the clipped scalar back to the
  // field rather than dropping it, so the total is still conserved.
  let lo = Infinity, hi = -Infinity;
  for (let j = 1; j <= N; j++) {
    for (let i = 1; i <= N; i++) {
      const v = d0[ix(SIZE, i, j)];
      if (v < lo) lo = v;
      if (v > hi) hi = v;
    }
  }
  let clipped = 0;
  for (let j = 1; j <= N; j++) {
    for (let i = 1; i <= N; i++) {
      const k = ix(SIZE, i, j);
      const v = cur[k];
      if (v > hi) { clipped += v - hi; cur[k] = hi; }
      else if (v < lo) { clipped += v - lo; cur[k] = lo; }
    }
  }
  // Push the clipped scalar (positive = trimmed an overshoot, negative = filled
  // an undershoot) back over the whole interior, weighted by how much room each
  // cell has toward the bound we're moving away from, so the interior total is
  // unchanged and no cell is driven across a bound.
  if (clipped !== 0) {
    let capacity = 0;
    for (let j = 1; j <= N; j++) {
      for (let i = 1; i <= N; i++) {
        const v = cur[ix(SIZE, i, j)];
        capacity += clipped > 0 ? hi - v : v - lo;
      }
    }
    if (capacity > 0) {
      const scale = Math.min(1, clipped / capacity);
      for (let j = 1; j <= N; j++) {
        for (let i = 1; i <= N; i++) {
          const k = ix(SIZE, i, j);
          const v = cur[k];
          cur[k] = v + (clipped > 0 ? hi - v : v - lo) * scale;
        }
      }
    }
  }
  setBnd(f, 0, d);
};

const project = (f, velU, velV, p, divg) => {
  const { N, SIZE } = f;
  const h = 1 / N;
  for (let j = 1; j <= N; j++) {
    for (let i = 1; i <= N; i++) {
      divg[ix(SIZE, i, j)] = -0.5 * h * (
        velU[ix(SIZE, i + 1, j)] - velU[ix(SIZE, i - 1, j)] +
        velV[ix(SIZE, i, j + 1)] - velV[ix(SIZE, i, j - 1)]
      );
      p[ix(SIZE, i, j)] = 0;
    }
  }
  setBnd(f, 0, divg);
  setBnd(f, 0, p);
  linSolve(f, 0, p, divg, 1, 4);

  for (let j = 1; j <= N; j++) {
    for (let i = 1; i <= N; i++) {
      velU[ix(SIZE, i, j)] -= 0.5 * (p[ix(SIZE, i + 1, j)] - p[ix(SIZE, i - 1, j)]) / h;
      velV[ix(SIZE, i, j)] -= 0.5 * (p[ix(SIZE, i, j + 1)] - p[ix(SIZE, i, j - 1)]) / h;
    }
  }
  setBnd(f, 1, velU);
  setBnd(f, 2, velV);
};

const velStep = (f) => {
  [f.u, f.uPrev] = [f.uPrev, f.u];
  diffuse(f, 1, f.u, f.uPrev, f.visc);
  [f.v, f.vPrev] = [f.vPrev, f.v];
  diffuse(f, 2, f.v, f.vPrev, f.visc);
  project(f, f.u, f.v, f.uPrev, f.vPrev);
  [f.u, f.uPrev] = [f.uPrev, f.u];
  [f.v, f.vPrev] = [f.vPrev, f.v];
  advect(f, 1, f.u, f.uPrev, f.uPrev, f.vPrev);
  advect(f, 2, f.v, f.vPrev, f.uPrev, f.vPrev);
  project(f, f.u, f.v, f.uPrev, f.vPrev);
};

const densStep = (f) => {
  [f.dens, f.densPrev] = [f.densPrev, f.dens];
  diffuse(f, 0, f.dens, f.densPrev, f.diff);
  [f.dens, f.densPrev] = [f.densPrev, f.dens];
  advectScalar(f, f.dens, f.densPrev, f.u, f.v);
  if (f.fade > 0) {
    const keep = 1 - f.fade;
    for (let i = 0; i < f.dens.length; i++) f.dens[i] *= keep;
  }
};

// Temperature rides the flow with the same conservative MUSCL scheme as the dye
// (closed box conserves the interior sum to ~machine precision), then conducts
// between neighbours via the implicit Jacobi solve. Walls are insulating:
// setBnd with b = 0 is zero-gradient, so no heat flux crosses the boundary.
// Flux-form explicit conduction: each step is a convex combination of a cell and
// its four neighbours (sub-stepped so the mixing weight stays <= 1/4), with
// insulating zero-gradient walls carrying no flux. Conservative AND monotone BY
// CONSTRUCTION — the interior thermal-energy sum is preserved to machine
// precision at any iteration count, and no cell ever leaves the local data
// range. This replaces the truncated implicit Jacobi solve, which only
// conserved at convergence (~2% drift at the default iter) and forced scenarios
// to crank `iter`.
const conduct = (f) => {
  const { N, SIZE, kappa, dt, temp, tmp } = f;
  const a = kappa * dt * N * N;
  if (a <= 0) return;
  const sub = Math.max(1, Math.ceil(a / 0.25));
  const as = a / sub;
  for (let s = 0; s < sub; s++) {
    for (let j = 1; j <= N; j++) {
      for (let i = 1; i <= N; i++) {
        const k = ix(SIZE, i, j);
        tmp[k] = temp[k] + as * (
          temp[k - 1] + temp[k + 1] + temp[k - SIZE] + temp[k + SIZE] - 4 * temp[k]
        );
      }
    }
    for (let j = 1; j <= N; j++) {
      for (let i = 1; i <= N; i++) temp[ix(SIZE, i, j)] = tmp[ix(SIZE, i, j)];
    }
    setBnd(f, 0, temp);
  }
};

const tempStep = (f) => {
  [f.temp, f.tempPrev] = [f.tempPrev, f.temp];
  advectScalar(f, f.temp, f.tempPrev, f.u, f.v);
  conduct(f);
  if (f.heat) {
    const { N, SIZE } = f;
    for (let j = 1; j <= N; j++) {
      for (let i = 1; i <= N; i++) f.temp[ix(SIZE, i, j)] += f.heat(i, j);
    }
    setBnd(f, 0, f.temp);
  }
};

// Advect the three phase fractions through the shared flow, then renormalise
// each interior cell back to `capacity`. Skipped entirely when there is no water
// anywhere (keeps the dry-air fast path cheap). Air is advected like the others
// so a uniform mixture stays uniform; the renormalise only mops up the sub-milli
// splitting error.
const waterStep = (f) => {
  const { N, SIZE, capacity } = f;
  let anyWater = false;
  for (let j = 1; j <= N && !anyWater; j++) {
    for (let i = 1; i <= N; i++) {
      const k = ix(SIZE, i, j);
      if (f.liquid[k] !== 0 || f.vapour[k] !== 0) { anyWater = true; break; }
    }
  }
  if (!anyWater) return;

  [f.liquid, f.liquidPrev] = [f.liquidPrev, f.liquid];
  advectScalar(f, f.liquid, f.liquidPrev, f.u, f.v);
  [f.vapour, f.vapourPrev] = [f.vapourPrev, f.vapour];
  advectScalar(f, f.vapour, f.vapourPrev, f.u, f.v);
  [f.air, f.airPrev] = [f.airPrev, f.air];
  advectScalar(f, f.air, f.airPrev, f.u, f.v);

  // Renormalise each interior cell back to `capacity` by making AIR the slack
  // phase: air = capacity - (liquid + vapour). This keeps liquid + vapour — the
  // conserved water total — EXACTLY as the conservative advection produced it,
  // and l + v + air == capacity holds by construction. With one shared
  // incompressible velocity field and no boiling divergence source (round 7),
  // a cell can transiently pack in more water than fits during vigorous boiling;
  // air then goes slightly negative rather than water being invented or
  // destroyed. It is a tracked, unrendered field — the visible water is right.
  for (let j = 1; j <= N; j++) {
    for (let i = 1; i <= N; i++) {
      const k = ix(SIZE, i, j);
      f.air[k] = capacity - f.liquid[k] - f.vapour[k];
    }
  }
  setBnd(f, 0, f.liquid);
  setBnd(f, 0, f.vapour);
  setBnd(f, 0, f.air);
};

// Phase change: 1:1 by mass between liquid and vapour at cells past the
// threshold, bounded per step for stability. Latent heat leaves f.temp on
// boiling and returns to it on condensation — so sensible + latent energy is
// conserved by construction, and air is never touched (AC 22).
const phaseChangeStep = (f) => {
  if (!f.phaseChange) return;
  const { N, SIZE, capacity, latentHeat, boilTemp, condenseTemp } = f;
  const maxRate = 0.25 * capacity;
  for (let j = 1; j <= N; j++) {
    for (let i = 1; i <= N; i++) {
      const k = ix(SIZE, i, j);
      const T = f.temp[k];
      if (T >= boilTemp && f.liquid[k] > 0) {
        // Convert at most enough to bring the cell back to boilTemp — a boiling
        // plateau. Latent heat leaves f.temp with the vapour.
        const toThreshold = latentHeat > 0
          ? (T - boilTemp) * capacity / latentHeat
          : maxRate;
        const dm = Math.min(f.liquid[k], maxRate, Math.max(0, toThreshold));
        if (dm > 0) {
          f.liquid[k] -= dm;
          f.vapour[k] += dm;
          f.temp[k] = T - latentHeat * dm / capacity;
        }
      } else if (T <= condenseTemp && f.vapour[k] > 0) {
        // Symmetric: condense at most enough to warm the cell back to
        // condenseTemp, releasing latent heat into f.temp.
        const toThreshold = latentHeat > 0
          ? (condenseTemp - T) * capacity / latentHeat
          : maxRate;
        const dm = Math.min(f.vapour[k], maxRate, Math.max(0, toThreshold));
        if (dm > 0) {
          f.vapour[k] -= dm;
          f.liquid[k] += dm;
          f.temp[k] = T + latentHeat * dm / capacity;
        }
      }
    }
  }
  setBnd(f, 0, f.temp);
  setBnd(f, 0, f.liquid);
  setBnd(f, 0, f.vapour);
};

// Thermal buoyancy: a body force on the shared vertical velocity field. Warmer-
// than-average fluid is pushed toward -v (up / smaller j), colder toward +v.
// The force is proportional to the local deviation from the mean interior
// temperature, so a spatially uniform field produces exactly zero force in every
// cell. Injected before velStep so the pressure solve keeps the field
// divergence-free, exactly as the other forces are handled.
const buoyancyStep = (f) => {
  const { N, SIZE, buoyancy, dt, temp, v } = f;
  let sum = 0;
  for (let j = 1; j <= N; j++) {
    for (let i = 1; i <= N; i++) sum += temp[ix(SIZE, i, j)];
  }
  const mean = sum / (N * N);
  for (let j = 1; j <= N; j++) {
    for (let i = 1; i <= N; i++) {
      const k = ix(SIZE, i, j);
      // `-=`, not `+=`: advect() backtraces along +v toward LARGER j, and larger
      // j renders LOWER on screen (js/main.js maps interior row j to canvas row
      // j-1, canvas y down). So warm fluid (temp > mean) must be driven to
      // negative v to rise. Getting this sign backwards once passed the old
      // net-displacement test by a wall-bounce fluke — hence the direct
      // early-time direction checks in test/buoyancy.js.
      v[k] -= buoyancy * (temp[k] - mean) * dt;
    }
  }
  setBnd(f, 2, v);
};

export const step = (f) => {
  if (f.buoyancy !== 0) buoyancyStep(f);
  velStep(f);
  densStep(f);
  tempStep(f);
  waterStep(f);
  phaseChangeStep(f);
};

// Stamp dye and momentum into a square of half-width `r` around interior cell
// (ci, cj). 1-indexed; out-of-range cells are skipped.
export const splat = (f, ci, cj, r, amount, vx, vy) => {
  const { N, SIZE } = f;
  for (let b = -r; b <= r; b++) {
    for (let a = -r; a <= r; a++) {
      const i = ci + a, j = cj + b;
      if (i < 1 || i > N || j < 1 || j > N) continue;
      f.dens[ix(SIZE, i, j)] += amount;
      f.u[ix(SIZE, i, j)] += vx;
      f.v[ix(SIZE, i, j)] += vy;
    }
  }
};

// Total advected scalar over the interior. The boundary ring is excluded: it
// holds no fluid, its cells are just zero-gradient copies of the edge, and
// counting them would double-count scalar that piles up against a wall. The
// interior sum is the quantity the conservative advection actually preserves.
export const totalDensity = (f) => {
  const { N, SIZE } = f;
  let sum = 0;
  for (let j = 1; j <= N; j++) {
    for (let i = 1; i <= N; i++) sum += f.dens[ix(SIZE, i, j)];
  }
  return sum;
};

// Max |divergence| over the interior after a step — should sit near zero once
// projection has run.
export const maxDivergence = (f) => {
  const { N, SIZE, u, v } = f;
  let m = 0;
  for (let j = 1; j <= N; j++) {
    for (let i = 1; i <= N; i++) {
      const d = Math.abs(
        u[ix(SIZE, i + 1, j)] - u[ix(SIZE, i - 1, j)] +
        v[ix(SIZE, i, j + 1)] - v[ix(SIZE, i, j - 1)]
      );
      if (d > m) m = d;
    }
  }
  return m;
};

// RMS |divergence| over the interior — a whole-field "how incompressible is
// this" number, less alarmist than maxDivergence about lone boundary cells.
export const rmsDivergence = (f) => {
  const { N, SIZE, u, v } = f;
  let acc = 0;
  for (let j = 1; j <= N; j++) {
    for (let i = 1; i <= N; i++) {
      const d =
        u[ix(SIZE, i + 1, j)] - u[ix(SIZE, i - 1, j)] +
        v[ix(SIZE, i, j + 1)] - v[ix(SIZE, i, j - 1)];
      acc += d * d;
    }
  }
  return Math.sqrt(acc / (N * N));
};

export const hasNonFinite = (f) => {
  for (const arr of [f.u, f.v, f.dens, f.temp, f.liquid, f.vapour, f.air]) {
    for (let i = 0; i < arr.length; i++) if (!Number.isFinite(arr[i])) return true;
  }
  return false;
};

export const IX = ix;
