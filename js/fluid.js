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
    u: cell(), v: cell(), uPrev: cell(), vPrev: cell(),
    dens: cell(), densPrev: cell(),
    temp: cell(), tempPrev: cell(), // temperature field + in-place-swapped scratch
    tmp: cell(),  // grid-sized scratch: Jacobi sweeps, and the advected-scalar working buffer
    flux: cell(), // grid-sized scratch: per-face flux buffer for conservative scalar advection
  };
  // Seed the temperature interior. temp0 is a uniform Number or an (i, j) => value
  // callback over 1-indexed interior cells; the boundary ring stays zero and is
  // refreshed as a zero-gradient copy by setBnd on the first step.
  const temp0 = opts.temp0 ?? 0;
  const t0 = typeof temp0 === 'function' ? temp0 : () => temp0;
  for (let j = 1; j <= N; j++) {
    for (let i = 1; i <= N; i++) f.temp[ix(SIZE, i, j)] = t0(i, j);
  }
  return f;
};

const ix = (SIZE, i, j) => i + SIZE * j;

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
  const nxt = f.tmp;      // Jacobi scratch — free during densStep
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
const tempStep = (f) => {
  [f.temp, f.tempPrev] = [f.tempPrev, f.temp];
  advectScalar(f, f.temp, f.tempPrev, f.u, f.v);
  if (f.kappa > 0) {
    [f.temp, f.tempPrev] = [f.tempPrev, f.temp];
    diffuse(f, 0, f.temp, f.tempPrev, f.kappa);
  }
};

export const step = (f) => {
  velStep(f);
  densStep(f);
  tempStep(f);
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
  for (const arr of [f.u, f.v, f.dens, f.temp]) {
    for (let i = 0; i < arr.length; i++) if (!Number.isFinite(arr[i])) return true;
  }
  return false;
};

export const IX = ix;
