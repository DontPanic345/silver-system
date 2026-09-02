// Round 3 — Buoyancy (groundwork).
//
//   node test/buoyancy.js   (run via: npm test)
//
// Intent: the sim is heading for a closed box where heating the water makes
// warm vapour RISE, cool aloft, condense, and rain back down — a closed cycle
// driven by physics alone. Buoyancy is the body force that makes warm fluid
// rise. Both liquid and gas move by the single shared, solved velocity field;
// buoyancy is a force added into THAT field, not a second momentum field.
//
// This round only proves the groundwork:
//
//   * AC 6 — a warm blob released into an otherwise uniform domain has its
//     centre of mass move UP; a cold blob's centre of mass moves DOWN.
//   * AC 7 — with uniform temperature and density everywhere, NO bulk motion
//     develops from buoyancy alone (a flat field stays still, to floating point).
//
// Load-bearing here: the SIGN of the centre-of-mass movement (warm up, cold
// down) and the near-zero velocity bound for a flat field. NOT load-bearing:
// the exact buoyancy coefficient, the blob size/shape, or how far/fast the
// blob actually moves. Assertions below are on direction and near-zero bounds.
//
// ------------------------------------------------------------------------
// Grid orientation (how "up" was determined):
//
//   js/main.js render() writes interior row j (1..N) straight into image row
//   (j-1) via putImageData, and canvas y increases DOWNWARD. So a larger j is
//   lower on screen and a smaller j is higher. "Up" is the -j / -v direction.
//   A warm blob rising therefore moves its centroid toward SMALLER j.
//
// ------------------------------------------------------------------------
// Assumed API for the Green phase to implement to:
//
//   createFluid(N, { buoyancy, ... })
//     * opts.buoyancy : scalar coefficient for the thermal buoyancy body
//       force. Defaults to 0 (no buoyancy — current behaviour, all other
//       tests unaffected).
//
//   step(f), when buoyancy != 0, adds a vertical velocity contribution to the
//   shared field f.v proportional to the LOCAL temperature deviation from the
//   domain's mean interior temperature (equivalently a reference temp such
//   that a uniform field has zero deviation everywhere):
//
//       f.v[cell] += -buoyancy * (f.temp[cell] - meanInteriorTemp) * dt
//
//   with the sign such that WARMER-than-average fluid is pushed toward -v
//   (up / smaller j) and COLDER-than-average toward +v (down / larger j).
//   Because the force is a deviation, a spatially uniform temperature field
//   produces exactly zero force in every cell -> zero bulk motion (AC 7).
//
// These tests currently fail because no buoyancy force exists: opts.buoyancy
// is ignored, so the warm/cold blobs never move and AC 6 fails on a centroid
// that does not shift. (AC 7 passes trivially today and is a guard for Green.)
// ------------------------------------------------------------------------

import { createFluid, step, hasNonFinite, IX } from '../js/fluid.js';

let failures = 0;
const check = (name, ok, detail = '') => {
  console.log(`  ${ok ? 'PASS' : 'FAIL'} ${name}${detail ? `  (${detail})` : ''}`);
  if (!ok) failures++;
};

// Weighted centroid row (j) of the interior, weight = w(temp).
const centroidY = (f, w) => {
  const { N, SIZE } = f;
  let m = 0, my = 0;
  for (let j = 1; j <= N; j++) {
    for (let i = 1; i <= N; i++) {
      const wt = w(f.temp[IX(SIZE, i, j)]);
      m += wt; my += wt * j;
    }
  }
  return my / m;
};

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

const setBlob = (f, lo, hi, value) => {
  for (let j = lo; j <= hi; j++) {
    for (let i = lo; i <= hi; i++) f.temp[IX(f.SIZE, i, j)] = value;
  }
};

// --- AC 6: warm blob rises, cold blob sinks ------------------------------
{
  console.log('a warm blob\'s centre of mass moves up (AC 6)');
  const N = 64;
  const f = createFluid(N, { dt: 0.15, kappa: 0, buoyancy: 0.6, temp0: 0 });
  // Warm square on a cold (0) background, centred in the domain so it is free
  // to move either way; only buoyancy can break the up/down symmetry.
  setBlob(f, 25, 39, 1);
  const startY = centroidY(f, (t) => t); // warm-weighted
  for (let s = 0; s < 120; s++) step(f);
  const endY = centroidY(f, (t) => t);
  check('warm-weighted centroid moves up (smaller j)',
    Number.isFinite(endY) && startY - endY > 0.5,
    `centroid j ${startY.toFixed(2)} -> ${endY.toFixed(2)}`);
  check('field stays finite', !hasNonFinite(f));
}

{
  console.log('a cold blob\'s centre of mass moves down (AC 6)');
  const N = 64;
  const f = createFluid(N, { dt: 0.15, kappa: 0, buoyancy: 0.6, temp0: 1 });
  // Cold square on a warm (1) background.
  setBlob(f, 25, 39, 0);
  const coldWeight = (t) => 1 - t; // coldness-weighted centroid
  const startY = centroidY(f, coldWeight);
  for (let s = 0; s < 120; s++) step(f);
  const endY = centroidY(f, coldWeight);
  check('cold-weighted centroid moves down (larger j)',
    Number.isFinite(endY) && endY - startY > 0.5,
    `centroid j ${startY.toFixed(2)} -> ${endY.toFixed(2)}`);
  check('field stays finite', !hasNonFinite(f));
}

// --- AC 7: flat field -> no bulk motion from buoyancy --------------------
{
  console.log('uniform temperature and density -> no bulk motion (AC 7)');
  const N = 48;
  const f = createFluid(N, { dt: 0.15, kappa: 0, buoyancy: 0.6, temp0: 0.7 });
  // Uniform temperature everywhere, no density gradient, no kick. Buoyancy is
  // a deviation force, so every cell's force must be exactly zero.
  for (let s = 0; s < 200; s++) step(f);
  check('no velocity develops from a flat field',
    !hasNonFinite(f) && maxSpeed(f) < 1e-9,
    `max |u|+|v| after 200 steps = ${maxSpeed(f).toExponential(2)}`);
}

console.log(failures === 0 ? '\nALL CHECKS PASSED' : `\n${failures} CHECK(S) FAILED`);
process.exit(failures === 0 ? 0 : 1);
