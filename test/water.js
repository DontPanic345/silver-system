// Round 6 — Water, vapour, air and phase change.
//
//   node test/water.js   (run via: npm test)
//
// Intent: a closed box holds air, liquid water and water vapour, each carrying a
// temperature. Heat the water -> it boils -> vapour rises -> cools -> condenses
// -> rains back down. A closed water cycle driven by physics; mass and energy go
// round the loop and are conserved to a good approximation ("nothing appears or
// vanishes; drift does not grow without bound").
//
// This slice is COMPOSITION + PHASE CHANGE + LATENT HEAT + CONSERVATION. Liquid
// falling and vapour rising by the solved flow is round 7 and is NOT asserted
// here (decision 1 in the AC doc: one shared incompressible velocity field with
// the phase fractions advected through it — no separate momentum fields, no
// settling velocity yet).
//
//   * AC 17 — every cell carries liquid + vapour + air fractions that sum to the
//     cell's capacity; air is an explicit tracked field, so vapour displaces air
//     rather than appearing from nowhere.
//   * AC 18 — in a closed domain, total water (liquid + vapour) is conserved to
//     within 1% across any amount of boiling and condensation.
//   * AC 19 — liquid at/above the boiling threshold converts to vapour and that
//     conversion REMOVES latent heat: the boiling cell ends COOLER than the same
//     cell in an otherwise identical run with phase change disabled.
//   * AC 20 — vapour meeting the condensation condition converts to liquid and
//     RELEASES latent heat: the condensing cell ends WARMER than in the
//     phase-change-disabled run.
//   * AC 21 — total energy (sensible + latent) in a closed domain with no
//     sources drifts < 2% across a full boil-then-condense round trip.
//   * AC 22 — air alone is conserved: its interior total is unchanged by any
//     amount of phase change.
//   * AC 23 — a "boil-a-pool" scenario exists; measurably, its total vapour
//     fraction rises while heat is applied.
//
// Carry-forward from round 4 (conservative conduction):
//   * The Jacobi diffuse/linSolve conduction path conserves the interior
//     thermal-energy sum only AT CONVERGENCE — ~2.3% drift at the default
//     iter (24). "hot-meets-cold" currently papers over this with iter: 100.
//     Pin conduction conservation at the DEFAULT iteration count: hot-beside-
//     cold, no flow, interior temp-sum drift < 0.5% over 200 steps. RED now;
//     forces Green to make conduction conservative BY CONSTRUCTION (flux-form
//     update, or solve-then-rescale to the pre-solve sum). Once fixed,
//     "hot-meets-cold" can drop back to a normal iter.
//
// ------------------------------------------------------------------------
// Assumed API for the Green phase to implement to:
//
//   createFluid(N, opts):
//     opts.capacity     : per-cell mixture capacity. Default 1.0.
//     opts.phaseChange  : boolean, default true. `false` disables all
//                         liquid<->vapour conversion (AC 19/20 compare a run
//                         with it on against an otherwise identical run off).
//     opts.latentHeat   : energy per unit water converted, in (temp * capacity)
//                         units. Moved out of / into f.temp on conversion.
//     opts.boilTemp     : liquid at or above this converts to vapour.
//     opts.condenseTemp : vapour at or below this converts to liquid.
//     opts.water0(i, j) : optional seed for an interior cell, 1-indexed; returns
//                         { liquid, vapour } and air = capacity - liquid - vapour.
//                         Default: all air (liquid = vapour = 0, air = capacity).
//     opts.heat         : optional; a Number or (i, j) => delta added into
//                         f.temp each step. A crude pre-round-8 forcing so the
//                         "boil-a-pool" scenario can sustain heating without the
//                         placeable heat-source entities (those are round 8).
//
//   f.liquid, f.vapour, f.air : Float32Array(SIZE*SIZE), interior 1..N plus the
//     one-cell boundary ring, like every other field. Each is advected by the
//     SHARED (f.u, f.v) flow with the SAME conservative MUSCL scheme as
//     dens/temp. After advection the three fractions are renormalised so
//     liquid + vapour + air == capacity in every interior cell.
//   f.liquidPrev, f.vapourPrev, f.airPrev : in-place-swapped scratch.
//
//   f.channels grows to include 'liquid' and 'vapour' (both get renderers in
//     js/main.js — see test/player.js). 'air' stays a tracked field but is NOT
//     in f.channels: it needs no dedicated view (it is capacity minus the
//     visible water) — a deliberate call, revisit if a later round needs it.
//
//   step(f): after advection (+ conduction) a phase-change pass converts
//     liquid<->vapour 1:1 by mass at cells meeting the threshold, moving
//     latentHeat * (converted / capacity) out of f.temp on boiling and into
//     f.temp on condensation. Bounded conversion per step for stability.
//
//   Energy accounting used by the assertions below:
//     sensible = interiorSum(f.temp)                 (uniform heat capacity)
//     latent   = opts.latentHeat * interiorSum(f.vapour)
//     total    = sensible + latent
//
// The water assertions currently fail because f.liquid / f.vapour / f.air do
// not exist yet; the conduction carry-forward fails on real drift.
// ------------------------------------------------------------------------

import { createFluid, step, IX, hasNonFinite } from '../js/fluid.js';
import { interiorSum } from '../js/measure.js';
import { scenarios, runScenario } from '../js/scenarios.js';

let failures = 0;
const check = (name, ok, detail = '') => {
  console.log(`  ${ok ? 'PASS' : 'FAIL'} ${name}${detail ? `  (${detail})` : ''}`);
  if (!ok) failures++;
};

const sizeOK = (f, a) => a instanceof Float32Array && a.length === f.SIZE * f.SIZE;
const hasWater = (f) => sizeOK(f, f.liquid) && sizeOK(f, f.vapour) && sizeOK(f, f.air);
const sum = (f, a) => interiorSum(f, a);
const blockSum = (f, a, i0, i1, j0, j1) => {
  let s = 0;
  for (let j = j0; j <= j1; j++) for (let i = i0; i <= i1; i++) s += a[IX(f.SIZE, i, j)];
  return s;
};
const blockRange = (N) => { const c = Math.floor(N / 2); return [c - 3, c + 4, c - 3, c + 4]; };
const inBlock = (N, i, j) => {
  const [i0, i1, j0, j1] = blockRange(N);
  return i >= i0 && i <= i1 && j >= j0 && j <= j1;
};

// --- carry-forward (round 4): conservative conduction at the DEFAULT iter -----
{
  console.log('conduction conserves the interior thermal-energy sum at the DEFAULT iteration count (carry-forward)');
  const N = 40;
  const f = createFluid(N, {
    dt: 0.15,
    kappa: 0.1,
    buoyancy: 0,
    phaseChange: false,
    temp0: (i) => (i <= N / 2 ? 40 : 0), // hot left half beside cold right half
  });
  // No flow, no sources: pure conduction. iter left at the solver default (24).
  const start = interiorSum(f, f.temp);
  let worst = 0;
  for (let s = 0; s < 200; s++) {
    step(f);
    worst = Math.max(worst, Math.abs(interiorSum(f, f.temp) - start) / Math.abs(start));
  }
  check('interior temp-sum drift stays under 0.5% over 200 steps at the default iter',
    !hasNonFinite(f) && worst < 0.005,
    `worst drift ${(worst * 100).toFixed(2)}%`);
}

// --- AC 17: three fractions per cell summing to capacity; air is explicit -----
{
  console.log('cells carry liquid + vapour + air summing to capacity; air is tracked and displaced (AC 17)');
  const cap = 1.0;
  const N = 32;
  const f = createFluid(N, {
    dt: 0.12, capacity: cap, phaseChange: true,
    latentHeat: 2, boilTemp: 100, condenseTemp: 100, temp0: 0,
    water0: (i, j) => (j >= 17 ? { liquid: cap, vapour: 0 } : { liquid: 0, vapour: 0 }),
  });
  if (!hasWater(f)) {
    check('f.liquid / f.vapour / f.air are grid-sized Float32Arrays', false, 'water channels missing');
    check('liquid + vapour + air == capacity in every interior cell (initial)', false, 'no water channels');
    check('the per-cell sum stays == capacity after the flow moves the fractions', false, 'no water channels');
  } else {
    check('f.liquid / f.vapour / f.air are grid-sized Float32Arrays', true);
    const worstCellDrift = () => {
      let w = 0;
      for (let j = 1; j <= f.N; j++) for (let i = 1; i <= f.N; i++) {
        const k = IX(f.SIZE, i, j);
        w = Math.max(w, Math.abs(f.liquid[k] + f.vapour[k] + f.air[k] - cap));
      }
      return w;
    };
    check('liquid + vapour + air == capacity in every interior cell (initial)',
      worstCellDrift() <= 1e-5, `worst cell drift ${worstCellDrift().toExponential(2)}`);
    // Give the field a shove so advection actually redistributes the fractions.
    for (let j = 1; j <= f.N; j++) for (let i = 1; i <= f.N; i++) {
      f.u[IX(f.SIZE, i, j)] = 0.03;
      f.v[IX(f.SIZE, i, j)] = -0.02;
    }
    for (let s = 0; s < 40; s++) step(f);
    check('the per-cell sum stays == capacity after the flow moves the fractions',
      !hasNonFinite(f) && worstCellDrift() <= 1e-3,
      `worst cell drift ${worstCellDrift().toExponential(2)} — vapour must displace air, not appear from nowhere`);
  }
}

// --- AC 22: air total is untouched by phase change ---------------------------
{
  console.log('air alone is conserved: boiling and condensation do not change the air total (AC 22)');
  const N = 32, cap = 1.0;
  const f = createFluid(N, {
    dt: 0.1, kappa: 0, buoyancy: 0,
    capacity: cap, phaseChange: true, latentHeat: 2, boilTemp: 100, condenseTemp: 100,
    temp0: 200, // well above boiling everywhere
    water0: () => ({ liquid: 0.5 * cap, vapour: 0 }), // air = 0.5*cap everywhere; no flow
  });
  if (!hasWater(f)) {
    check('interior air total is unchanged by boiling (AC 22)', false, 'no water channels');
  } else {
    const a0 = sum(f, f.air);
    const v0 = sum(f, f.vapour);
    let worstAir = 0;
    for (let s = 0; s < 120; s++) {
      step(f);
      worstAir = Math.max(worstAir, Math.abs(sum(f, f.air) - a0) / Math.abs(a0));
    }
    check('boiling actually occurred (vapour total rose)', sum(f, f.vapour) - v0 > 0.1,
      `vapour +${(sum(f, f.vapour) - v0).toFixed(3)}`);
    check('interior air total is unchanged by boiling (AC 22)',
      !hasNonFinite(f) && worstAir < 1e-3, `worst air drift ${(worstAir * 100).toFixed(3)}%`);
  }
}

// --- AC 19: boiling removes latent heat -------------------------------------
{
  console.log('boiling removes latent heat: the boiling cell ends cooler than with phase change disabled (AC 19)');
  const N = 32, cap = 1.0, latentHeat = 4;
  const base = {
    dt: 0.1, kappa: 0.02, buoyancy: 0,
    capacity: cap, latentHeat, boilTemp: 100, condenseTemp: 100,
    temp0: (i, j) => (inBlock(N, i, j) ? 180 : 60),
    water0: (i, j) => (inBlock(N, i, j) ? { liquid: cap, vapour: 0 } : { liquid: 0, vapour: 0 }),
  };
  const on = createFluid(N, { ...base, phaseChange: true });
  const off = createFluid(N, { ...base, phaseChange: false });
  if (!hasWater(on)) {
    check('the boiling cell ends cooler than the phase-change-disabled run (AC 19)', false, 'no water channels');
  } else {
    for (let s = 0; s < 60; s++) { step(on); step(off); }
    const r = blockRange(N);
    const tOn = blockSum(on, on.temp, ...r);
    const tOff = blockSum(off, off.temp, ...r);
    check('some liquid actually converted to vapour', sum(on, on.vapour) > 0.05,
      `vapour ${sum(on, on.vapour).toFixed(3)}`);
    check('the boiling cell ends cooler than the phase-change-disabled run (AC 19)',
      !hasNonFinite(on) && tOn < tOff - 1e-3,
      `block temp-sum: phaseChange on ${tOn.toFixed(2)} vs off ${tOff.toFixed(2)}`);
  }
}

// --- AC 20: condensation releases latent heat -------------------------------
{
  console.log('condensation releases latent heat: the condensing cell ends warmer than with phase change disabled (AC 20)');
  const N = 32, cap = 1.0, latentHeat = 4;
  const base = {
    dt: 0.1, kappa: 0.02, buoyancy: 0,
    capacity: cap, latentHeat, boilTemp: 100, condenseTemp: 100,
    temp0: 40, // below the condensation threshold everywhere
    water0: (i, j) => (inBlock(N, i, j) ? { liquid: 0, vapour: cap } : { liquid: 0, vapour: 0 }),
  };
  const on = createFluid(N, { ...base, phaseChange: true });
  const off = createFluid(N, { ...base, phaseChange: false });
  if (!hasWater(on)) {
    check('the condensing cell ends warmer than the phase-change-disabled run (AC 20)', false, 'no water channels');
  } else {
    for (let s = 0; s < 60; s++) { step(on); step(off); }
    const r = blockRange(N);
    const tOn = blockSum(on, on.temp, ...r);
    const tOff = blockSum(off, off.temp, ...r);
    check('some vapour actually converted to liquid', sum(on, on.liquid) > 0.05,
      `liquid ${sum(on, on.liquid).toFixed(3)}`);
    check('the condensing cell ends warmer than the phase-change-disabled run (AC 20)',
      !hasNonFinite(on) && tOn > tOff + 1e-3,
      `block temp-sum: phaseChange on ${tOn.toFixed(2)} vs off ${tOff.toFixed(2)}`);
  }
}

// --- AC 18 + AC 21: closed box, water + energy conserved over a boil/condense round trip
{
  console.log('closed box: total water within 1% and total energy within 2% across boil-then-condense (AC 18, AC 21)');
  const N = 48, cap = 1.0, latentHeat = 3, boilTemp = 100;
  const f = createFluid(N, {
    dt: 0.12, kappa: 0.06, buoyancy: 0.4,
    capacity: cap, phaseChange: true, latentHeat, boilTemp, condenseTemp: boilTemp,
    // a hot liquid pool along the bottom; cool air above it. The pool boils, the
    // vapour rises into the cool air and condenses — a full round trip, no sources.
    temp0: (i, j) => (j >= N - 10 ? 160 : 10),
    water0: (i, j) => (j >= N - 10 ? { liquid: cap, vapour: 0 } : { liquid: 0, vapour: 0 }),
  });
  if (!hasWater(f)) {
    check('total water (liquid + vapour) conserved within 1% over the run (AC 18)', false, 'no water channels');
    check('boiling then condensation both actually occur over the run', false, 'no water channels');
    check('total energy (sensible + latent) drift < 2% across the round trip (AC 21)', false, 'no water channels');
  } else {
    const water = () => sum(f, f.liquid) + sum(f, f.vapour);
    const energy = () => sum(f, f.temp) + latentHeat * sum(f, f.vapour);
    const w0 = water(), e0 = energy();
    let vapPeak = 0, worstWater = 0, worstEnergy = 0;
    for (let s = 0; s < 500; s++) {
      step(f);
      vapPeak = Math.max(vapPeak, sum(f, f.vapour));
      worstWater = Math.max(worstWater, Math.abs(water() - w0) / Math.abs(w0));
      worstEnergy = Math.max(worstEnergy, Math.abs(energy() - e0) / Math.abs(e0));
    }
    const vapEnd = sum(f, f.vapour);
    check('total water (liquid + vapour) conserved within 1% over the run (AC 18)',
      !hasNonFinite(f) && worstWater < 0.01, `worst water drift ${(worstWater * 100).toFixed(2)}%`);
    check('boiling then condensation both actually occur over the run',
      vapPeak > 0.5 && vapEnd < vapPeak * 0.7,
      `vapour peak ${vapPeak.toFixed(3)} -> end ${vapEnd.toFixed(3)}`);
    check('total energy (sensible + latent) drift < 2% across the round trip (AC 21)',
      worstEnergy < 0.02, `worst energy drift ${(worstEnergy * 100).toFixed(2)}%`);
  }
}

// --- AC 23: a "boil-a-pool" scenario, vapour rises while heat is applied -----
{
  console.log('a "boil-a-pool" scenario exists and its total vapour fraction rises while heat is applied (AC 23)');
  const sc = scenarios.find((s) => s.id === 'boil-a-pool');
  check('a scenario with id "boil-a-pool" is registered in js/scenarios.js', sc != null);

  if (sc == null) {
    check('total vapour fraction rises measurably over the boil-a-pool run', false, 'scenario missing');
    check('total water (liquid + vapour) stays conserved within 1% over the run', false, 'scenario missing');
    check("the boil-a-pool scenario's own declared assertions all pass", false, 'scenario missing');
  } else {
    const f = createFluid(sc.gridSize, sc.opts || {});
    if (typeof sc.init === 'function') sc.init(f);
    if (!hasWater(f)) {
      check('total vapour fraction rises measurably over the boil-a-pool run', false, 'no water channels');
      check('total water (liquid + vapour) stays conserved within 1% over the run', false, 'no water channels');
    } else {
      const water = () => sum(f, f.liquid) + sum(f, f.vapour);
      const w0 = water() || 1;
      const vStart = sum(f, f.vapour);
      const half = Math.floor(sc.steps / 2);
      let vMid = vStart, worstWater = 0;
      for (let s = 0; s < sc.steps; s++) {
        step(f);
        if (s === half) vMid = sum(f, f.vapour);
        worstWater = Math.max(worstWater, Math.abs(water() - w0) / Math.abs(w0));
      }
      const vEnd = sum(f, f.vapour);
      check('total vapour fraction rises measurably over the boil-a-pool run',
        !hasNonFinite(f) && vMid > vStart && vEnd > vStart + 0.05,
        `vapour ${vStart.toFixed(3)} -> mid ${vMid.toFixed(3)} -> end ${vEnd.toFixed(3)}`);
      check('total water (liquid + vapour) stays conserved within 1% over the run',
        worstWater < 0.01, `worst water drift ${(worstWater * 100).toFixed(2)}%`);
    }
    let r = null;
    try { r = runScenario(sc); } catch (e) { /* reported below */ }
    check("the boil-a-pool scenario's own declared assertions all pass",
      r != null && r.pass === true,
      r ? (r.results || []).map((x) => `${x.name}:${x.pass ? 'ok' : 'X'}`).join(' | ') : 'runScenario threw');
  }
}

console.log(failures === 0 ? '\nALL CHECKS PASSED' : `\n${failures} CHECK(S) FAILED`);
process.exit(failures === 0 ? 0 : 1);
