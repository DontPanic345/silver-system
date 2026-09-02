// Stable Fluids — Jos Stam, "Real-Time Fluid Dynamics for Games" (2003).
//
// A grid-based (Eulerian) solver. Velocity and a passive dye ("density") live
// on a grid; each step advects them along the flow (semi-Lagrangian) and then
// projects the velocity field back to divergence-free with a Gauss-Seidel
// pressure solve. That projection is what makes it unconditionally stable —
// it never blows up regardless of timestep.
//
// Pure module: no DOM, no rendering. `js/main.js` drives it on screen;
// `test/fluid-probe.js` drives it headless.

export const createFluid = (N, opts = {}) => {
  const SIZE = N + 2; // interior N×N plus a one-cell boundary ring
  const cell = () => new Float32Array(SIZE * SIZE);
  return {
    N,
    SIZE,
    dt: opts.dt ?? 0.12,
    iter: opts.iter ?? 24, // Jacobi sweeps per linear solve
    visc: opts.visc ?? 0,
    diff: opts.diff ?? 0,
    fade: opts.fade ?? 0, // fraction of dye removed each step (0 = conserve)
    u: cell(), v: cell(), uPrev: cell(), vPrev: cell(),
    dens: cell(), densPrev: cell(),
    tmp: cell(), // scratch for the Jacobi solver
  };
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
  advect(f, 0, f.dens, f.densPrev, f.u, f.v);
  if (f.fade > 0) {
    const keep = 1 - f.fade;
    for (let i = 0; i < f.dens.length; i++) f.dens[i] *= keep;
  }
};

export const step = (f) => {
  velStep(f);
  densStep(f);
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

export const totalDensity = (f) => {
  let sum = 0;
  for (let i = 0; i < f.dens.length; i++) sum += f.dens[i];
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
  for (const arr of [f.u, f.v, f.dens]) {
    for (let i = 0; i < arr.length; i++) if (!Number.isFinite(arr[i])) return true;
  }
  return false;
};

export const IX = ix;
