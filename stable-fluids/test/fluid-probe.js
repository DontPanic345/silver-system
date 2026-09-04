// Headless probes for the stable-fluids solver (js/fluid.js).
//
//   node test/fluid-probe.js   (or: npm test)
//
// No browser, no rendering — this file probes the velocity solver: stability at
// large dt, the field staying near-divergence-free, no motion from nowhere, and
// mirror symmetry (the Jacobi solver should give this exactly). Scalar-transport
// conservation, determinism and per-step cost live in test/conservation.js.
//
// Velocities are quoted in grid-widths per unit time; the advection backtrace
// is dt·N·velocity, so values here are deliberately small (~0.05).

import {
  createFluid, step, splat,
  maxDivergence, rmsDivergence, hasNonFinite, IX,
} from '../js/fluid.js';

let failures = 0;
const check = (name, ok, detail = '') => {
  console.log(`  ${ok ? 'PASS' : 'FAIL'} ${name}${detail ? `  (${detail})` : ''}`);
  if (!ok) failures++;
};
const maxSpeed = (f) => {
  let m = 0;
  for (let i = 0; i < f.u.length; i++) m = Math.max(m, Math.hypot(f.u[i], f.v[i]));
  return m;
};

// --- stability: big dt, repeated shove, many steps ----------------------
{
  console.log('stability');
  const f = createFluid(96, { dt: 1.0, fade: 0 });
  for (let s = 0; s < 300; s++) {
    splat(f, 20, 48, 3, 1, 0.4, 0.05);
    step(f);
  }
  check('no NaN/Inf after 300 steps at dt=1.0', !hasNonFinite(f));
  check('velocity stays bounded', maxSpeed(f) < 10, `max speed ${maxSpeed(f).toFixed(2)}`);
}

// --- projection keeps the field near-incompressible -------------------
{
  console.log('incompressibility');
  const f = createFluid(96, { dt: 0.2, fade: 0 });
  for (let s = 0; s < 80; s++) {
    splat(f, 48, 20, 3, 1, 0, 0.06);
    step(f);
  }
  for (let s = 0; s < 20; s++) step(f); // let the solve settle
  const rms = rmsDivergence(f) / maxSpeed(f);
  const peak = maxDivergence(f) / maxSpeed(f); // a jet on a wall is the worst case
  check('RMS divergence < 3% of flow speed', rms < 0.03, `rms ${(rms * 100).toFixed(2)}%`);
  check('worst-cell divergence < 30% of flow speed', peak < 0.3, `peak ${(peak * 100).toFixed(1)}%`);
}

// --- a still fluid stays still ----------------------------------------
{
  console.log('quiescence');
  const f = createFluid(64, { dt: 0.2, fade: 0 });
  splat(f, 32, 32, 5, 1, 0, 0); // dye, no momentum
  for (let s = 0; s < 100; s++) step(f);
  check('no motion appears from nowhere', maxSpeed(f) < 1e-9, `max speed ${maxSpeed(f).toExponential(1)}`);
}

// --- symmetric setup stays exactly symmetric (Jacobi is order-free) ---
{
  console.log('symmetry');
  const N = 80;
  const f = createFluid(N, { dt: 0.15, fade: 0 });
  for (let s = 0; s < 80; s++) {
    splat(f, N / 2, N - 12, 3, 1, 0, -0.05);     // jet straddling the axis
    splat(f, N / 2 + 1, N - 12, 3, 1, 0, -0.05);
    step(f);
  }
  let worst = 0, peak = 0;
  for (let j = 1; j <= N; j++) {
    for (let i = 1; i <= N; i++) {
      const mi = N + 1 - i;
      peak = Math.max(peak, f.dens[IX(f.SIZE, i, j)]);
      worst = Math.max(worst, Math.abs(f.dens[IX(f.SIZE, i, j)] - f.dens[IX(f.SIZE, mi, j)]));
      worst = Math.max(worst, Math.abs(f.u[IX(f.SIZE, i, j)] + f.u[IX(f.SIZE, mi, j)]));
    }
  }
  check('dye + flow stay mirror-symmetric', worst < 1e-9, `worst ${worst.toExponential(1)} (peak ${peak.toFixed(1)})`);
}

console.log(failures === 0 ? '\nALL CHECKS PASSED' : `\n${failures} CHECK(S) FAILED`);
process.exit(failures === 0 ? 0 : 1);
