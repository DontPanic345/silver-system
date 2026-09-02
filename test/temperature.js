// Round 2 — Temperature field (groundwork).
//
//   node test/temperature.js   (run via: npm test)
//
// Intent: the sim is heading for a closed box holding air / liquid / vapour,
// each carrying a temperature. Heat drives the water cycle; energy must be
// conserved to a good approximation ("nothing appears or vanishes; drift does
// not grow without bound"). This round only lays the groundwork: a temperature
// field that RIDES the flow and CONDUCTS between neighbours, with the right
// relationships holding.
//
//   * AC 3 — every cell carries a temperature, advected by the flow and
//     diffused by conduction;
//   * AC 4 — closed domain, no heat sources: total thermal energy drifts
//     < 1% over 500 steps;
//   * AC 5 — a hot region beside a cold region equalises monotonically: the
//     hottest cell never gets hotter, the coldest never gets colder, and the
//     two converge.
//
// Conserved quantity ("thermal energy"): we model a uniform heat capacity, so
// sensible heat is proportional to the sum of cell temperatures over the
// interior. Every energy assertion below is on that interior sum.
//
// ------------------------------------------------------------------------
// Assumed API for the Green phase to implement to:
//
//   createFluid(N, { temp0, kappa, ... })
//     * opts.temp0 : initial temperature. Either a Number (uniform) or a
//       function (i, j) => value, called for interior cells with 1-indexed
//       (i, j). Defaults to 0 (uniform) if omitted.
//     * opts.kappa : thermal diffusivity for conduction (same role `diff`
//       plays for the dye channel). Defaults to 0 (no conduction).
//
//   f.temp      : Float32Array, SIZE*SIZE, the temperature field (interior
//                 1..N, plus the one-cell boundary ring like every other field).
//   f.tempPrev  : Float32Array scratch, swapped in place like densPrev.
//
//   step(f) advects f.temp along (f.u, f.v) with the SAME conservative scheme
//   the dye uses (so a closed box conserves the interior sum to ~machine
//   precision), then conducts it by kappa. Boundary is zero-gradient (b = 0):
//   no heat flux through the walls.
//
// The tests below currently fail because f.temp does not exist yet.
// ------------------------------------------------------------------------

import { createFluid, step, splat, hasNonFinite, IX } from '../js/fluid.js';

let failures = 0;
const check = (name, ok, detail = '') => {
  console.log(`  ${ok ? 'PASS' : 'FAIL'} ${name}${detail ? `  (${detail})` : ''}`);
  if (!ok) failures++;
};

const hasTemp = (f) => f.temp instanceof Float32Array && f.temp.length === f.SIZE * f.SIZE;

// --- interior reductions over f.temp --------------------------------------
const heat = (f) => {
  const { N, SIZE } = f;
  let sum = 0;
  for (let j = 1; j <= N; j++) {
    for (let i = 1; i <= N; i++) sum += f.temp[IX(SIZE, i, j)];
  }
  return sum;
};

const tempRange = (f) => {
  const { N, SIZE } = f;
  let lo = Infinity, hi = -Infinity;
  for (let j = 1; j <= N; j++) {
    for (let i = 1; i <= N; i++) {
      const v = f.temp[IX(SIZE, i, j)];
      if (v < lo) lo = v;
      if (v > hi) hi = v;
    }
  }
  return { lo, hi };
};

// Temperature-weighted centroid column of the interior (for "rides the flow").
const centroidX = (f) => {
  const { N, SIZE } = f;
  let m = 0, mx = 0;
  for (let j = 1; j <= N; j++) {
    for (let i = 1; i <= N; i++) {
      const w = f.temp[IX(SIZE, i, j)];
      m += w; mx += w * i;
    }
  }
  return mx / m;
};

// --- AC 3: temperature is carried by the flow -----------------------------
{
  console.log('temperature is advected by the flow (AC 3)');
  const f = createFluid(64, { dt: 0.15, kappa: 0, temp0: 0 });
  if (!hasTemp(f)) {
    check('f.temp is a grid-sized Float32Array', false, 'f.temp missing / wrong shape');
    check('hot blob centroid moves downstream with the flow', false, 'no temp field');
  } else {
    check('f.temp is a grid-sized Float32Array', true);
    // A compact hot blob on a cold (0) background, pushed right by a one-off
    // momentum kick. kappa = 0, so this isolates transport.
    for (let j = 28; j <= 36; j++) {
      for (let i = 12; i <= 20; i++) f.temp[IX(f.SIZE, i, j)] = 1;
    }
    splat(f, 16, 32, 6, 0, 0.06, 0); // momentum only, no dye, rightward
    const startX = centroidX(f);
    for (let s = 0; s < 80; s++) step(f);
    const endX = centroidX(f);
    check('hot blob centroid moves downstream with the flow',
      Number.isFinite(endX) && endX - startX > 1,
      `centroid x ${startX.toFixed(2)} -> ${endX.toFixed(2)}`);
    check('temperature field stays finite', !hasNonFinite(f) && Number.isFinite(heat(f)));
  }
}

// --- AC 3 / AC 4: conservative transport — pure advection ----------------
{
  console.log('sensible heat conserved under pure advection (AC 3)');
  const f = createFluid(64, { dt: 0.15, kappa: 0, temp0: (i) => (i < 32 ? 1 : 0.2) });
  if (!hasTemp(f)) {
    check('interior thermal energy drifts < 1% over 500 steps (advection only)', false, 'no temp field');
  } else {
    splat(f, 20, 24, 4, 0, 0.05, 0.03);
    splat(f, 44, 40, 4, 0, -0.04, 0.02);
    const start = heat(f);
    for (let s = 0; s < 500; s++) step(f);
    const end = heat(f);
    const drift = Math.abs(end - start) / start;
    check('interior thermal energy drifts < 1% over 500 steps (advection only)',
      Number.isFinite(end) && drift < 0.01,
      `drift ${(drift * 100).toFixed(2)}%  (start ${start.toFixed(3)} -> end ${end.toFixed(3)})`);
  }
}

// --- AC 4: closed domain, no sources, conduction on --------------------
{
  console.log('closed domain conserves thermal energy with conduction (AC 4)');
  const f = createFluid(64, {
    dt: 0.15,
    kappa: 0.00015,
    temp0: (i, j) => 0.5 + 0.5 * Math.sin(i * 0.3) * Math.cos(j * 0.25),
  });
  if (!hasTemp(f)) {
    check('interior thermal energy drifts < 1% over 500 steps (advection + conduction)', false, 'no temp field');
  } else {
    splat(f, 24, 24, 5, 0, 0.05, 0.02);
    const start = heat(f);
    let worst = 0;
    for (let s = 0; s < 500; s++) {
      step(f);
      worst = Math.max(worst, Math.abs(heat(f) - start) / Math.abs(start));
    }
    check('interior thermal energy drifts < 1% over 500 steps (advection + conduction)',
      !hasNonFinite(f) && worst < 0.01,
      `worst drift ${(worst * 100).toFixed(2)}%`);
  }
}

// --- AC 5: hot beside cold equalises monotonically and converges -------
{
  console.log('hot region beside cold region equalises monotonically (AC 5)');
  const N = 48;
  const f = createFluid(N, {
    dt: 0.15,
    kappa: 0.001,
    temp0: (i) => (i <= N / 2 ? 1 : 0), // left half hot, right half cold; no flow
  });
  if (!hasTemp(f)) {
    check('the hottest cell never gets hotter', false, 'no temp field');
    check('the coldest cell never gets colder', false, 'no temp field');
    check('the hot/cold gap never widens (monotone equalisation)', false, 'no temp field');
    check('the two regions converge substantially', false, 'no temp field');
    check('mean temperature is unchanged (no energy invented or lost)', false, 'no temp field');
  } else {
    const start = tempRange(f);
    const startMean = heat(f) / (N * N);
    let prevGap = start.hi - start.lo;
    let hottestRose = 0, coldestFell = 0, gapEverGrew = 0;
    for (let s = 0; s < 2400; s++) {
      step(f);
      const r = tempRange(f);
      hottestRose = Math.max(hottestRose, r.hi - start.hi);
      coldestFell = Math.max(coldestFell, start.lo - r.lo);
      const gap = r.hi - r.lo;
      gapEverGrew = Math.max(gapEverGrew, gap - prevGap);
      prevGap = gap;
    }
    const end = tempRange(f);
    const endMean = heat(f) / (N * N);
    const EPS = 1e-6;
    check('the hottest cell never gets hotter', hottestRose <= EPS,
      `rose by ${hottestRose.toExponential(2)}`);
    check('the coldest cell never gets colder', coldestFell <= EPS,
      `fell by ${coldestFell.toExponential(2)}`);
    check('the hot/cold gap never widens (monotone equalisation)', gapEverGrew <= EPS,
      `worst widening ${gapEverGrew.toExponential(2)}`);
    check('the two regions converge substantially',
      (end.hi - end.lo) < 0.15 * (start.hi - start.lo),
      `gap ${(start.hi - start.lo).toFixed(3)} -> ${(end.hi - end.lo).toFixed(3)}`);
    check('mean temperature is unchanged (no energy invented or lost)',
      Math.abs(endMean - startMean) < 0.01 * startMean,
      `mean ${startMean.toFixed(4)} -> ${endMean.toFixed(4)}`);
  }
}

console.log(failures === 0 ? '\nALL CHECKS PASSED' : `\n${failures} CHECK(S) FAILED`);
process.exit(failures === 0 ? 0 : 1);
