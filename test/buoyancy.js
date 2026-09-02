// Round 3 — Buoyancy DIRECTION (corrective).
//
//   node test/buoyancy.js   (run via: npm test)
//
// Intent: the sim is heading for a closed box where heating the water makes
// warm vapour RISE, cool aloft, condense, and rain back down — a closed cycle
// driven by physics alone. Buoyancy is the body force that makes warm fluid
// rise. Both liquid and gas move by the single shared, solved velocity field;
// buoyancy is a force added into THAT field, not a second momentum field.
//
// Load-bearing here: warm rises, cold sinks, and a spatially uniform field
// develops no motion at all. NOT load-bearing: the exact buoyancy coefficient,
// the blob size/shape, the step count, or how far the blob travels. Assertions
// below are on direction/sign and near-zero bounds.
//
// ------------------------------------------------------------------------
// Grid orientation (how "up" was determined):
//
//   js/main.js render() writes interior row j (1..N) straight into image row
//   (j-1) via putImageData, and canvas y increases DOWNWARD. So a larger j is
//   lower on screen and a smaller j is higher. "Up" is the -j direction.
//
//   The solver's advect/advectScalar backtrace along +v toward LARGER j: a
//   positive v transports fluid DOWNWARD on screen. So for warm fluid to RISE,
//   the buoyancy force must push warm (temp > mean) cells toward NEGATIVE v.
//
// ------------------------------------------------------------------------
// Why this is measured in the early-time regime:
//
//   The domain is a closed box. Once the plume reaches a wall the flow is
//   dominated by the reflected return circulation, and the temperature-weighted
//   centroid can sit on either side of its start depending on where in that
//   sloshing cycle you sample — the net displacement after many steps is NOT a
//   clean probe of force direction. In the first handful of steps, before the
//   blob gets anywhere near a wall (verified: velocity in the rows adjacent to
//   the top/bottom walls stays ~1e-3 through step 8, versus ~0.17 inside the
//   blob), the only thing moving the blob is the buoyancy force, and its
//   direction is unambiguous.
//
// ------------------------------------------------------------------------
// Assumed API for the Green phase to implement to:
//
//   createFluid(N, { buoyancy, ... })
//     * opts.buoyancy : scalar coefficient for the thermal buoyancy body
//       force. Defaults to 0 (no buoyancy — all other tests unaffected).
//
//   step(f), when buoyancy != 0, adds a vertical velocity contribution to the
//   shared field f.v proportional to the LOCAL temperature deviation from the
//   domain's mean interior temperature:
//
//       f.v[cell] -= buoyancy * (f.temp[cell] - meanInteriorTemp) * dt
//
//   i.e. WARMER-than-average fluid is pushed toward -v (up / smaller j) and
//   COLDER-than-average toward +v (down / larger j). Because the force is a
//   deviation from the mean, a spatially uniform temperature field produces
//   exactly zero force in every cell -> zero bulk motion (AC 7).
//
// History: buoyancyStep once had this sign inverted (`v[k] += ...`), pushing warm
// fluid down. A net-displacement test after 120 steps missed it — it passed off a
// wall-bounce rebound. These early-time direction checks are the guard against a
// re-inversion; AC 7 is sign-independent and guards against the force leaking in
// on a flat field.
// ------------------------------------------------------------------------

import { createFluid, step, hasNonFinite, IX } from '../js/fluid.js';
import { weightedCentroid } from '../js/measure.js';

let failures = 0;
const check = (name, ok, detail = '') => {
  console.log(`  ${ok ? 'PASS' : 'FAIL'} ${name}${detail ? `  (${detail})` : ''}`);
  if (!ok) failures++;
};

// Temperature-weighted centroid row (j) of the interior, weight = w(temp).
const centroidY = (f, w) => weightedCentroid(f, f.temp, { axis: 'j', weight: w });

const maxSpeed = (f) => {
  const { N, SIZE } = f;
  let m = 0;
  for (let j = 1; j <= N; j++) {
    for (let i = 1; i <= N; i++) {
      const s = Math.abs(f.u[IX(SIZE, i, j)]) + Math.abs(f.v[IX(SIZE, i, j)]);
      if (s > m) m = s;
    }
  }
  return m;
};

const LO = 25, HI = 39; // interior square blob on a 64-wide grid, centred (j = 32)

const setBlob = (f, value) => {
  for (let j = LO; j <= HI; j++) {
    for (let i = LO; i <= HI; i++) f.temp[IX(f.SIZE, i, j)] = value;
  }
};

// Mean vertical velocity over the blob's cells — the most direct probe of which
// way the buoyancy body force points.
const meanBlobV = (f) => {
  let s = 0, n = 0;
  for (let j = LO; j <= HI; j++) {
    for (let i = LO; i <= HI; i++) { s += f.v[IX(f.SIZE, i, j)]; n++; }
  }
  return s / n;
};

// Max |v| in the two rows adjacent to each of the top/bottom walls — a guard
// that we are still in the pre-reflection regime when we sample.
const wallV = (f) => {
  const { N, SIZE } = f;
  let m = 0;
  for (const j of [1, 2, N - 1, N]) {
    for (let i = 1; i <= N; i++) m = Math.max(m, Math.abs(f.v[IX(SIZE, i, j)]));
  }
  return m;
};

// One buoyancy run: warm blob (background 0) or cold blob (background 1),
// stepped STEPS times, returning start/end centroid and the blob velocity.
const STEPS = 5; // verified pre-wall-reflection for the LO..HI blob: wallV ~6e-3 here
const runBlob = ({ temp0, blob, weight }) => {
  const N = 64;
  const f = createFluid(N, { dt: 0.15, kappa: 0, buoyancy: 0.6, temp0 });
  setBlob(f, blob);
  const startY = centroidY(f, weight);
  for (let s = 0; s < STEPS; s++) step(f);
  return {
    f,
    startY,
    endY: centroidY(f, weight),
    blobV: meanBlobV(f),
    wallV: wallV(f),
    finite: !hasNonFinite(f),
  };
};

// --- AC 6: warm rises / cold sinks (early-time, pre-reflection) ---------
// One directional case. `dir` is the sign of "the way this blob should travel"
// in j (down = +1, up = -1): a warm blob in cold background should go up, a cold
// blob in warm background down. Both the centroid transport and the mean blob
// velocity must carry that sign.
const directionCase = ({ title, temp0, blob, weight, dir }) => {
  console.log(title);
  const r = runBlob({ temp0, blob, weight });
  check('still pre-wall-reflection at the sample point',
    r.wallV < 0.02, `max |v| next to a wall = ${r.wallV.toExponential(2)}`);
  check(`temperature-weighted centroid moved ${dir > 0 ? 'DOWN (larger j)' : 'UP (smaller j)'}`,
    r.finite && dir * (r.endY - r.startY) > 0.5,
    `centroid j ${r.startY.toFixed(2)} -> ${r.endY.toFixed(2)}`);
  check(`mean vertical velocity over the blob is ${dir > 0 ? 'positive (downward force)' : 'negative (upward force)'}`,
    r.finite && dir * r.blobV > 1e-3, `mean v over blob = ${r.blobV.toExponential(3)}`);
  check('field stays finite', r.finite);
};

directionCase({
  title: 'a warm blob rises before it reaches a wall (AC 6)',
  temp0: 0, blob: 1, weight: (t) => t, dir: -1,
});
directionCase({
  title: 'a cold blob sinks before it reaches a wall (AC 6)',
  temp0: 1, blob: 0, weight: (t) => 1 - t, dir: +1,
});

// --- AC 7: flat field -> no bulk motion from buoyancy ------------------
{
  console.log('uniform temperature and density -> no bulk motion (AC 7)');
  const N = 48;
  const f = createFluid(N, { dt: 0.15, kappa: 0, buoyancy: 0.6, temp0: 0.7 });
  // Uniform temperature everywhere, no density gradient, no kick. Buoyancy is a
  // deviation force, so every cell's force must be exactly zero.
  for (let s = 0; s < 200; s++) step(f);
  check('no velocity develops from a flat field',
    !hasNonFinite(f) && maxSpeed(f) < 1e-9,
    `max |u|+|v| after 200 steps = ${maxSpeed(f).toExponential(2)}`);
}

console.log(failures === 0 ? '\nALL CHECKS PASSED' : `\n${failures} CHECK(S) FAILED`);
process.exit(failures === 0 ? 0 : 1);
