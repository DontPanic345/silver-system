// Round 7 — Liquid falls, gas rises.
//
//   node test/settling.js   (run via: npm test)
//
// Intent: closed box; heat water -> boils -> vapour rises -> cools -> condenses
// -> rains. Both fluids move by the SOLVED flow field, not by ad-hoc falling
// rules. This round adds the two body forces that make that happen:
//
//   * liquid water sinks through gas and collects below (AC 24);
//   * water vapour rises through air on its own — from a density/composition
//     force, NOT because it is hot (AC 25);
//   * a body of liquid at rest with no heat is STABLE — no creep, no runaway,
//     no growing oscillation over a long run (AC 26).
//
// Both are BODY FORCES on the ONE shared incompressible velocity field, exactly
// like `buoyancyStep`: injected before the pressure solve so the field stays
// divergence-free. There are NO separate momentum / settling fields.
//
// Grid orientation (established, not re-derived here): smaller j = UP on screen;
// positive v transports fluid DOWN; thermal buoyancy pushes warm cells toward
// -v (up).
//
// ------------------------------------------------------------------------
// Assumed API for the Green phase to implement to (js/fluid.js `createFluid`
// opts, and a body-force pass in `step` before `velStep`):
//
//   opts.gravity : number, default 0. A downward body force on the shared v
//     field proportional to the local LIQUID-fraction deviation from the mean
//     interior liquid fraction:
//         v[k] += gravity * (liquid[k] - meanLiquid) * dt
//     Deviation form: a spatially uniform liquid fraction produces exactly zero
//     force in every cell (AC 7 stays alive). Sign: a cell with more liquid than
//     average is pushed toward +v (down).
//
//   opts.vapourBuoyancy : number, default 0. An upward body force on the shared
//     v field proportional to the local VAPOUR-fraction deviation from the mean
//     interior vapour fraction:
//         v[k] -= vapourBuoyancy * (vapour[k] - meanVapour) * dt
//     Deviation form (uniform vapour fraction -> zero force). Independent of
//     temperature — vapour rises because it is vapour, not because it is warm.
//     Sign: a cell with more vapour than average is pushed toward -v (up).
//
//   Both forces are deviation-form so `step` on a uniform mixture with any
//   combination of buoyancy / gravity / vapourBuoyancy set produces NO motion.
//
// Cross-cutting guards folded in here:
//   * AC 33 determinism — a settling run (gravity + vapourBuoyancy active) is
//     bit-identical on repeat. (Green: fold the new opts into the determinism
//     scenario in test/conservation.js too, if any new persistent state appears
//     — this round adds only opts, no new field.)
//   * AC 34 performance — a step with gravity + vapourBuoyancy active at the
//     shipped grid size (N=96) stays under 16 ms.
//   * AC 35 — headless under plain node, no DOM.
//
// Scenario ids this file pins (Green must add these to js/scenarios.js with real,
// passing assertions — test/scenarios.js runs every declared assertion):
//   * "rain-falls"    — liquid suspended high in a gas box; gravity on;
//                       measurably its liquid centre of mass moves DOWN and
//                       total water is conserved.
//   * "vapour-rises"  — vapour low in an air box; vapourBuoyancy on; NO heat,
//                       NO thermal buoyancy; measurably its vapour centre of
//                       mass moves UP.
//   * "still-pool"    — a flat liquid pool at rest on the floor; gravity on; no
//                       heat; stays flat and still over a long run.
// ------------------------------------------------------------------------

import { createFluid, step, IX, hasNonFinite } from '../js/fluid.js';
import { interiorSum, interiorRange, weightedCentroid } from '../js/measure.js';

let failures = 0;
const check = (name, ok, detail = '') => {
  console.log(`  ${ok ? 'PASS' : 'FAIL'} ${name}${detail ? `  (${detail})` : ''}`);
  if (!ok) failures++;
};

const maxSpeed = (f) => {
  const { N, SIZE } = f;
  let m = 0;
  for (let j = 1; j <= N; j++) for (let i = 1; i <= N; i++) {
    const k = IX(SIZE, i, j);
    const s = Math.hypot(f.u[k], f.v[k]);
    if (s > m) m = s;
  }
  return m;
};

const perStepMs = (f, iters) => {
  const t0 = performance.now();
  for (let s = 0; s < iters; s++) step(f);
  return (performance.now() - t0) / iters;
};

const centroidJ = (f, arr) => weightedCentroid(f, arr, { axis: 'j' });

// ==========================================================================
// AC 24 — liquid water falls through gas and collects below.
// ==========================================================================
{
  console.log('liquid water falls through gas and collects below (AC 24)');
  const N = 56;
  const f = createFluid(N, {
    dt: 0.12,
    buoyancy: 0,          // isolate the settling force from thermal buoyancy
    gravity: 0.6,         // <-- the force under test
    phaseChange: false,   // no liquid<->vapour conversion; pure transport
    capacity: 1,
    temp0: 20,            // uniform: nothing thermal is happening
    // A slab of liquid water suspended across the top of an otherwise air box.
    water0: (i, j) => (j >= 5 && j <= 12 ? { liquid: 1, vapour: 0 } : { liquid: 0, vapour: 0 }),
  });

  const startCentroid = centroidJ(f, f.liquid);
  const startWater = interiorSum(f, f.liquid) + interiorSum(f, f.vapour);

  let worstWaterDrift = 0;
  let worstNegAir = 0;
  let finite = true;
  for (let s = 0; s < 160; s++) {
    step(f);
    const water = interiorSum(f, f.liquid) + interiorSum(f, f.vapour);
    worstWaterDrift = Math.max(worstWaterDrift, Math.abs(water - startWater) / startWater);
    worstNegAir = Math.min(worstNegAir, interiorRange(f, f.air).lo);
    if (hasNonFinite(f)) finite = false;
  }
  const endCentroid = centroidJ(f, f.liquid);

  check('the liquid centre of mass moves measurably DOWNWARD (larger j) over the run',
    finite && endCentroid > startCentroid + 3,
    `liquid comJ ${startCentroid.toFixed(2)} -> ${endCentroid.toFixed(2)} (grid ${N})`);

  check('total water (liquid + vapour) is conserved to within 0.5% while it falls',
    worstWaterDrift < 0.005,
    `worst water drift ${(worstWaterDrift * 100).toFixed(3)}%`);

  check('the field stays finite as the liquid falls', finite);

  // Carry-forward guard (gap 3): displacement must happen via velocity, not by
  // driving the slack air phase negative. May be green already (no divergence
  // source this round) — becomes load-bearing if Green adds vapour expansion.
  check('slack air phase never goes appreciably negative during settling (carry-forward guard)',
    worstNegAir > -0.05,
    `min interior air ${worstNegAir.toFixed(4)}`);
}

// ==========================================================================
// AC 25 — vapour rises through air from the buoyancy force ALONE (not heat).
// ==========================================================================
{
  console.log('vapour rises through air with no "rise" instruction and no heat (AC 25)');
  const N = 56;
  const f = createFluid(N, {
    dt: 0.12,
    buoyancy: 0,           // NO thermal buoyancy
    vapourBuoyancy: 0.6,   // <-- the force under test
    gravity: 0.6,          // on, but there is no liquid for it to act on
    phaseChange: false,
    capacity: 1,
    temp0: 20,             // uniform temperature — vapour is NOT hot
    // no `heat` — nothing injects energy anywhere
    // A pocket of vapour sitting low in the box, air everywhere else.
    water0: (i, j) => (j >= 44 && j <= 51 ? { liquid: 0, vapour: 0.4 } : { liquid: 0, vapour: 0 }),
  });

  const startCentroid = centroidJ(f, f.vapour);
  const startVapour = interiorSum(f, f.vapour);
  const startTempSpread = (() => { const r = interiorRange(f, f.temp); return r.hi - r.lo; })();

  let worstVapourDrift = 0;
  let worstTempSpread = startTempSpread;
  let finite = true;
  for (let s = 0; s < 160; s++) {
    step(f);
    const v = interiorSum(f, f.vapour);
    worstVapourDrift = Math.max(worstVapourDrift, Math.abs(v - startVapour) / startVapour);
    const r = interiorRange(f, f.temp);
    worstTempSpread = Math.max(worstTempSpread, r.hi - r.lo);
    if (hasNonFinite(f)) finite = false;
  }
  const endCentroid = centroidJ(f, f.vapour);

  check('the vapour centre of mass moves measurably UPWARD (smaller j) over the run',
    finite && endCentroid < startCentroid - 3,
    `vapour comJ ${startCentroid.toFixed(2)} -> ${endCentroid.toFixed(2)} (grid ${N})`);

  check('the vapour rise is composition-driven, NOT thermal (temperature stays uniform)',
    worstTempSpread < 1e-6,
    `worst interior temperature spread ${worstTempSpread.toExponential(2)}`);

  check('total vapour is conserved to within 0.5% while it rises',
    worstVapourDrift < 0.005,
    `worst vapour drift ${(worstVapourDrift * 100).toFixed(3)}%`);

  check('the field stays finite as the vapour rises', finite);
}

// ==========================================================================
// AC 26 — a liquid body at rest with no heat is STABLE over a long run.
// (Carry-forward guard: passes trivially until Green adds the forces, then it
//  pins that the forces don't destabilise a flat pool.)
// ==========================================================================
{
  console.log('a resting liquid pool with no heat is stable over 500 steps (AC 26)');
  const N = 56;
  const floor = 38; // pool fills j = floor..N, full width -> a flat interface
  const f = createFluid(N, {
    dt: 0.12,
    buoyancy: 0,
    gravity: 0.6,          // the pool feels gravity and must STILL settle to rest
    vapourBuoyancy: 0.6,   // set, though there is no vapour
    phaseChange: false,
    capacity: 1,
    temp0: 20,             // uniform, no heat anywhere
    water0: (i, j) => (j >= floor ? { liquid: 1, vapour: 0 } : { liquid: 0, vapour: 0 }),
  });

  const startLiquid = interiorSum(f, f.liquid);
  const startCentroid = centroidJ(f, f.liquid);

  let worstLiquidDrift = 0;
  let worstCentroidShift = 0;
  const speeds = [];
  let finite = true;
  for (let s = 0; s < 500; s++) {
    step(f);
    const liq = interiorSum(f, f.liquid);
    worstLiquidDrift = Math.max(worstLiquidDrift, Math.abs(liq - startLiquid) / startLiquid);
    worstCentroidShift = Math.max(worstCentroidShift, Math.abs(centroidJ(f, f.liquid) - startCentroid));
    speeds.push(maxSpeed(f));
    if (hasNonFinite(f)) finite = false;
  }

  check('total liquid stays constant within 0.5% over 500 steps (no creep)',
    finite && worstLiquidDrift < 0.005,
    `worst liquid drift ${(worstLiquidDrift * 100).toFixed(3)}%`);

  check('the liquid centre of mass stays stationary within one cell (flat surface holds)',
    worstCentroidShift < 1.0,
    `worst centroid shift ${worstCentroidShift.toFixed(3)} cells`);

  const peak = Math.max(...speeds);
  const earlyPeak = Math.max(...speeds.slice(20, 120));
  const latePeak = Math.max(...speeds.slice(400));
  check('peak speed stays bounded over 500 steps — no runaway, no growing oscillation',
    finite && Number.isFinite(peak) && peak < 0.05 && latePeak <= earlyPeak * 1.2 + 1e-6,
    `peak ${peak.toExponential(2)}  early ${earlyPeak.toExponential(2)} -> late ${latePeak.toExponential(2)}`);
}

// ==========================================================================
// AC 7 stays alive — a uniform mixture with every body-force coeff set moves
// exactly nowhere. (Carry-forward guard: green today, load-bearing once Green
// wires gravity / vapourBuoyancy in.)
// ==========================================================================
{
  console.log('a spatially uniform mixture develops no motion with gravity + vapourBuoyancy + buoyancy all set (AC 7)');
  const N = 44;
  const f = createFluid(N, {
    dt: 0.15,
    buoyancy: 0.6,
    gravity: 0.6,
    vapourBuoyancy: 0.6,
    phaseChange: false,
    capacity: 1,
    temp0: 0.7,
    water0: () => ({ liquid: 0.5, vapour: 0.2 }),
  });
  for (let s = 0; s < 200; s++) step(f);
  check('no velocity develops from a flat mixture',
    !hasNonFinite(f) && maxSpeed(f) < 1e-9,
    `max speed after 200 steps = ${maxSpeed(f).toExponential(2)}`);
}

// ==========================================================================
// AC 33 — a settling run is deterministic (the new force paths included).
// ==========================================================================
{
  console.log('a settling run is bit-identical on repeat (AC 33)');
  const run = () => {
    const f = createFluid(40, {
      dt: 0.16,
      buoyancy: 0.1,
      gravity: 0.5,
      vapourBuoyancy: 0.5,
      kappa: 0.01,
      phaseChange: true,
      latentHeat: 3,
      boilTemp: 100,
      condenseTemp: 100,
      temp0: (i, j) => (j > 20 ? 90 : 30),
      water0: (i, j) => (j > 24
        ? { liquid: 0.8, vapour: 0.1 }
        : { liquid: 0, vapour: 0.2 }),
    });
    for (let s = 0; s < 120; s++) step(f);
    return f;
  };
  const a = run(), b = run();
  const fields = ['u', 'v', 'dens', 'temp', 'liquid', 'vapour', 'air'];
  let identical = true, firstDiff = '';
  for (const key of fields) {
    const x = a[key], y = b[key];
    for (let i = 0; identical && i < x.length; i++) {
      if (x[i] !== y[i]) { identical = false; firstDiff = `${key}[${i}] ${x[i]} vs ${y[i]}`; }
    }
  }
  check('two identical settling runs produce bit-identical fields', identical, firstDiff);
}

// ==========================================================================
// AC 34 — a step with the new forces active stays under 16 ms at N = 96.
// ==========================================================================
{
  console.log('a settling step stays under 16 ms at the shipped grid size (AC 34)');
  const N = 96;
  const f = createFluid(N, {
    dt: 0.12,
    buoyancy: 0.15,
    gravity: 0.6,
    vapourBuoyancy: 0.6,
    kappa: 0.05,
    phaseChange: true,
    latentHeat: 3,
    boilTemp: 100,
    condenseTemp: 100,
    temp0: (i, j) => (j > N * 0.6 ? 150 : 30),
    water0: (i, j) => (j > N * 0.6 ? { liquid: 1, vapour: 0 } : { liquid: 0, vapour: 0.1 }),
    heat: (i, j) => (j > N - 4 ? 5 : 0),
  });
  perStepMs(f, 20); // warm up JIT + spin the flow up
  const ms = perStepMs(f, 40);
  check(`step() under 16 ms at ${N}x${N} with gravity + vapourBuoyancy active`,
    ms < 16, `${ms.toFixed(1)} ms/step`);
}

// ==========================================================================
// AC 35 — headless.
// ==========================================================================
{
  console.log('runs headless under plain node (AC 35)');
  check('no DOM globals present while the suite runs',
    typeof window === 'undefined' && typeof document === 'undefined');
}

// ==========================================================================
// AC 24 / 25 / 26 — the named page scenarios exist and are real.
// ==========================================================================
{
  console.log('the rain-falls / vapour-rises / still-pool scenarios exist on the page (AC 24, 25, 26)');
  let mod = null, loadErr = null;
  try {
    mod = await import('../js/scenarios.js');
  } catch (e) { loadErr = e; }

  check('js/scenarios.js imports', mod != null,
    loadErr ? String(loadErr.message || loadErr).split('\n')[0] : '');

  const list = (mod && Array.isArray(mod.scenarios)) ? mod.scenarios : [];
  const byId = new Map(list.map((s) => [s.id, s]));
  const runScenario = mod && mod.runScenario;

  const rain = byId.get('rain-falls');
  check('a "rain-falls" scenario exists (AC 24)', rain != null);
  check('"rain-falls" turns the settling force on', !!(rain && rain.opts && rain.opts.gravity > 0),
    rain ? `gravity = ${rain.opts && rain.opts.gravity}` : '');

  const vap = byId.get('vapour-rises');
  check('a "vapour-rises" scenario exists (AC 25)', vap != null);
  check('"vapour-rises" turns the vapour-buoyancy force on', !!(vap && vap.opts && vap.opts.vapourBuoyancy > 0),
    vap ? `vapourBuoyancy = ${vap.opts && vap.opts.vapourBuoyancy}` : '');
  check('"vapour-rises" applies no heat (rise must be composition-driven, not thermal)',
    !!(vap && vap.opts && !vap.opts.heat && !vap.opts.buoyancy),
    vap ? `heat=${vap.opts && vap.opts.heat}  buoyancy=${vap.opts && vap.opts.buoyancy}` : '');

  const pool = byId.get('still-pool');
  check('a "still-pool" scenario exists (AC 26)', pool != null);
  check('"still-pool" feels gravity and applies no heat',
    !!(pool && pool.opts && pool.opts.gravity > 0 && !pool.opts.heat),
    pool ? `gravity=${pool.opts && pool.opts.gravity}  heat=${pool.opts && pool.opts.heat}` : '');

  if (typeof runScenario === 'function') {
    for (const [id, s] of [['rain-falls', rain], ['vapour-rises', vap], ['still-pool', pool]]) {
      if (!s) { check(`"${id}" every declared assertion passes`, false, 'scenario missing'); continue; }
      let res = null;
      try { res = runScenario(s); } catch (e) { /* fall through */ res = { pass: false, results: [{ name: 'threw', pass: false, detail: String(e.message || e) }] }; }
      check(`"${id}" every declared assertion passes`,
        res && res.pass === true && Array.isArray(res.results) && res.results.length > 0,
        res && res.results ? res.results.map((r) => `${r.name}:${r.pass ? 'ok' : 'X'}`).join(' | ') : '');
    }
  }
}

console.log(failures === 0 ? '\nALL CHECKS PASSED' : `\n${failures} CHECK(S) FAILED`);
process.exit(failures === 0 ? 0 : 1);
