// Round 1 — Conservative transport (groundwork).
//
//   node test/conservation.js   (run via: npm test)
//
// Intent: the sim is heading for a closed water cycle where mass and energy go
// round the loop and are conserved to a good approximation — "nothing appears
// or vanishes, and drift does not grow without bound". These probes pin that
// down for the one advected scalar we have today (the dye channel):
//
//   * a closed box with no sources or sinks must very nearly conserve the
//     total of an advected scalar over a long run (AC 1);
//   * a single advection step must never invent a value outside the range the
//     field already held — no new maxima, nothing negative (AC 2);
//
// plus the cross-cutting guards that hold every round:
//
//   * determinism — same scenario twice, bit-identical state (AC 33);
//   * performance — a step stays under 16 ms at the shipped grid size (AC 34);
//   * the suite runs headless under node with no browser (AC 35).
//
// The 0.5% / 500-step figure in AC 1 is a starting point, not gospel: if a
// genuinely conservative scheme cannot hit it, the fix is to report a better
// number, not to contort the solver.

import {
  createFluid, step, splat,
  totalDensity, hasNonFinite, IX,
} from '../js/fluid.js';

let failures = 0;
const check = (name, ok, detail = '') => {
  console.log(`  ${ok ? 'PASS' : 'FAIL'} ${name}${detail ? `  (${detail})` : ''}`);
  if (!ok) failures++;
};

const perStepMs = (f, iters) => {
  const t0 = performance.now();
  for (let s = 0; s < iters; s++) step(f);
  return (performance.now() - t0) / iters;
};

// Range of the advected scalar over the whole grid (interior + boundary ring).
const scalarRange = (f) => {
  let lo = Infinity, hi = -Infinity;
  for (let i = 0; i < f.dens.length; i++) {
    const v = f.dens[i];
    if (v < lo) lo = v;
    if (v > hi) hi = v;
  }
  return { lo, hi };
};

// The grid size main.js actually ships at. Kept in sync by AC 34's probe.
const SHIPPED_N = 180;

// --- AC 1: closed box, no sources/sinks, scalar total barely drifts ---------
{
  console.log('conservation (AC 1)');
  // A few different momentum kicks -> a few different velocity fields the
  // solver produces. Dye and momentum are injected ONCE, then the box is
  // closed: 500 pure steps, no further splats, fade off.
  const scenarios = [
    { tag: 'corner jet',   kick: (f) => splat(f, 16, 16, 4, 1, 0.06, 0.04) },
    { tag: 'off-axis jet', kick: (f) => splat(f, 24, 40, 4, 1, -0.05, 0.03) },
    { tag: 'shear pair',   kick: (f) => {
        splat(f, 20, 30, 3, 1, 0.05, 0);
        splat(f, 44, 34, 3, 1, -0.05, 0);
      } },
  ];
  for (const { tag, kick } of scenarios) {
    const f = createFluid(64, { dt: 0.15, fade: 0 });
    kick(f);
    const start = totalDensity(f);
    for (let s = 0; s < 500; s++) step(f);
    const end = totalDensity(f);
    const drift = Math.abs(end - start) / start;
    check(
      `scalar total drifts < 0.5% over 500 steps — ${tag}`,
      Number.isFinite(end) && drift < 0.005,
      `drift ${(drift * 100).toFixed(2)}%  (start ${start.toFixed(3)} -> end ${end.toFixed(3)})`,
    );
  }
}

// --- AC 2: advection never creates a value outside the prior range ----------
{
  console.log('no new extrema (AC 2)');
  const f = createFluid(64, { dt: 0.2, fade: 0 });
  // A lumpy non-negative scalar and a live velocity field.
  splat(f, 20, 32, 6, 1.0, 0.05, 0.02);
  splat(f, 40, 30, 4, 0.5, -0.03, 0.04);
  step(f); // establish a moving field

  let worstOver = 0;   // how far above the prior max any cell reached
  let worstUnder = 0;  // how far below the prior min (or below zero) any cell reached
  const EPS = 1e-6;
  for (let s = 0; s < 120; s++) {
    const before = scalarRange(f);
    step(f);
    const after = scalarRange(f);
    worstOver = Math.max(worstOver, after.hi - before.hi);
    worstUnder = Math.max(worstUnder, before.lo - after.lo, -after.lo);
  }
  check('advection introduces no new maximum', worstOver <= EPS,
    `worst overshoot ${worstOver.toExponential(2)}`);
  check('advection introduces nothing negative / below prior min', worstUnder <= EPS,
    `worst undershoot ${worstUnder.toExponential(2)}`);
  check('field stays finite throughout', !hasNonFinite(f));
}

// --- Carry-forward guard: force a Godunov overshoot --------------------------
// The conservative advection has a clamp-and-redistribute branch for the
// sub-milli overshoot that dimensional splitting can leave behind the 1D
// limiter. No other test drives a field sharp/fast enough to exercise it.
// A hard step discontinuity in the scalar plus a high-Courant velocity field
// (large dt, strong kick) is what triggers it — and it must still come out
// conservative and monotone (no value outside the pre-step range).
{
  console.log('Godunov overshoot stays conservative + monotone (carry-forward)');
  const f = createFluid(64, { dt: 0.6, fade: 0 });
  // A sharp slab: half the interior at 1, half at 0 — a one-cell-wide gradient.
  for (let j = 1; j <= 64; j++) {
    for (let i = 1; i <= 64; i++) f.dens[IX(f.SIZE, i, j)] = i <= 32 ? 1 : 0;
  }
  // Strong, sheared momentum so the Courant number across the step is well
  // above 1 and the split axes disagree.
  splat(f, 30, 24, 6, 0, 0.18, 0.05);
  splat(f, 34, 44, 6, 0, 0.14, -0.07);

  const start = totalDensity(f);
  let worstOver = 0, worstUnder = 0, worstDrift = 0;
  for (let s = 0; s < 60; s++) {
    const before = scalarRange(f);
    step(f);
    const after = scalarRange(f);
    worstOver = Math.max(worstOver, after.hi - before.hi);
    worstUnder = Math.max(worstUnder, before.lo - after.lo, -after.lo);
    worstDrift = Math.max(worstDrift, Math.abs(totalDensity(f) - start) / start);
  }
  const EPS = 1e-6;
  check('sharp-gradient / high-Courant run stays conservative', worstDrift < 0.005,
    `worst drift ${(worstDrift * 100).toFixed(3)}%`);
  check('sharp-gradient / high-Courant run invents no new extrema',
    worstOver <= EPS && worstUnder <= EPS,
    `over ${worstOver.toExponential(2)}  under ${worstUnder.toExponential(2)}`);
  check('field stays finite through the overshoot scenario', !hasNonFinite(f));
}

// --- Carry-forward guard: sub-cycling path conserves the scalar total -------
// At large dt the scalar advection sub-cycles (Courant > 1 -> multiple inner
// sweeps per step). Nothing pinned that the interior total survives that path.
{
  console.log('sub-cycled advection conserves scalar total (carry-forward)');
  const f = createFluid(64, { dt: 0.9, fade: 0 }); // deliberately large dt
  splat(f, 24, 32, 5, 1.0, 0.12, 0.06);
  splat(f, 44, 28, 4, 0.6, -0.09, 0.04);
  const start = totalDensity(f);
  let worst = 0;
  for (let s = 0; s < 200; s++) {
    step(f);
    worst = Math.max(worst, Math.abs(totalDensity(f) - start) / start);
  }
  check('scalar total drifts < 0.5% over 200 sub-cycled steps', Number.isFinite(worst) && worst < 0.005,
    `worst drift ${(worst * 100).toFixed(3)}%`);
}

// --- AC 33: determinism — same scenario twice, bit-identical state ----------
{
  console.log('determinism (AC 33)');
  const run = () => {
    // kappa + a temperature gradient so the advect-and-conduct temp path does
    // real work and its determinism is actually exercised, not just u/v/dens.
    const f = createFluid(72, {
      dt: 0.18, fade: 0, kappa: 0.0002,
      temp0: (i) => (i < 36 ? 1 : 0),
    });
    splat(f, 18, 36, 3, 0.7, 0.05, 0.02);
    splat(f, 50, 20, 2, 0.4, -0.03, 0.05);
    for (let s = 0; s < 150; s++) step(f);
    return f;
  };
  const a = run(), b = run();
  const fields = ['u', 'v', 'dens', 'temp'];
  let identical = true;
  let firstDiff = '';
  for (const key of fields) {
    const x = a[key], y = b[key];
    if (x.length !== y.length) { identical = false; break; }
    for (let i = 0; identical && i < x.length; i++) {
      if (x[i] !== y[i]) {
        identical = false;
        firstDiff = `${key}[${i}] ${x[i]} vs ${y[i]}`;
      }
    }
  }
  check('two identical runs produce bit-identical u/v/dens/temp', identical, firstDiff);
}

// --- AC 34: a step stays under 16 ms at the shipped grid size ---------------
{
  console.log('performance (AC 34)');
  console.log(`  shipped grid size: N = ${SHIPPED_N}`);
  const f = createFluid(SHIPPED_N, { dt: 0.12, fade: 0 });
  splat(f, 40, 90, 3, 0.6, 0.03, 0);
  perStepMs(f, 10); // warm up JIT
  const ms = perStepMs(f, 40);
  check(`step() under 16 ms at ${SHIPPED_N}x${SHIPPED_N}`, ms < 16, `${ms.toFixed(1)} ms/step`);
}

// --- AC 35: the suite runs headless, no browser ----------------------------
{
  console.log('headless (AC 35)');
  check('no DOM globals present while the suite runs',
    typeof window === 'undefined' && typeof document === 'undefined');
}

console.log(failures === 0 ? '\nALL CHECKS PASSED' : `\n${failures} CHECK(S) FAILED`);
process.exit(failures === 0 ? 0 : 1);
