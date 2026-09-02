// Round 4 — Scenarios as shared data.
//
// One declarative definition per scenario; two consumers (the headless suite in
// test/scenarios.js and the round-5 web page). Adding a scenario touches neither
// consumer. This module is DOM-free and import-safe under plain node.
//
// A Scenario is plain data:
//
//   { id, name, description, gridSize, steps, opts, entities,
//     init?(f), record?(f), assertions: [{ name, check(ctx) }] }
//
// `runScenario` is the shared runner: it builds the fluid from gridSize+opts,
// runs init, records history[0], steps `steps` times (recording after each step),
// then evaluates every assertion. ctx = { sim, history, scenario }.

import { createFluid, step, IX, hasNonFinite } from './fluid.js';
import { interiorSum, interiorRange, weightedCentroid } from './measure.js';

// --- shared runner -------------------------------------------------------------

export function runScenario(scenario) {
  const f = createFluid(scenario.gridSize, scenario.opts || {});
  if (typeof scenario.init === 'function') scenario.init(f);

  const snap = typeof scenario.record === 'function'
    ? () => scenario.record(f)
    : () => null;

  const history = [snap()];
  for (let s = 0; s < scenario.steps; s++) {
    step(f);
    history.push(snap());
  }

  const ctx = { sim: f, history, scenario };
  const results = (scenario.assertions || []).map((a) => {
    let out;
    try {
      out = a.check(ctx);
    } catch (e) {
      out = { pass: false, detail: `threw: ${String(e && e.message || e)}` };
    }
    return {
      name: a.name,
      pass: !!(out && out.pass),
      detail: (out && out.detail) || '',
    };
  });

  return {
    id: scenario.id,
    name: scenario.name,
    results,
    pass: results.every((r) => r.pass),
  };
}

export function runAllScenarios() {
  return scenarios.map(runScenario);
}

// --- helpers ------------------------------------------------------------------

// temperature-weighted vertical centre of mass (interior, 1-indexed rows).
// Weight is (temp - background) so a uniform field contributes nothing.
const tempComJ = (f, background) =>
  weightedCentroid(f, f.temp, { axis: 'j', weight: (t) => Math.max(0, t - background) });

// vertical centre of mass of a phase fraction (interior, 1-indexed rows).
const phaseComJ = (f, arr) => weightedCentroid(f, arr, { axis: 'j' });

// peak flow speed over the interior.
const peakSpeed = (f) => {
  const { N, SIZE } = f;
  let m = 0;
  for (let j = 1; j <= N; j++) for (let i = 1; i <= N; i++) {
    const k = IX(SIZE, i, j);
    const s = Math.hypot(f.u[k], f.v[k]);
    if (s > m) m = s;
  }
  return m;
};

// --- scenarios ---------------------------------------------------------------

const PLUME_BG = 0;
const PLUME_HOT = 1;

const HC_HOT = 40;
const HC_COLD = 0;
const HC_SPLIT = 20;

export const scenarios = [
  {
    id: 'warm-plume-rises',
    name: 'Warm plume rises',
    description: 'A warm blob on a uniform cold background drifts upward under buoyancy.',
    gridSize: 64,
    steps: 14,
    opts: { buoyancy: 0.6, dt: 0.15, kappa: 0, temp0: PLUME_BG },
    entities: [],
    init(f) {
      const { N, SIZE } = f;
      const cx = (N + 1) / 2, cy = (N + 1) / 2, r = 8;
      for (let j = 1; j <= N; j++) for (let i = 1; i <= N; i++) {
        if ((i - cx) ** 2 + (j - cy) ** 2 <= r * r) {
          f.temp[IX(SIZE, i, j)] = PLUME_HOT;
        }
      }
    },
    record(f) {
      return { comJ: tempComJ(f, PLUME_BG), finite: !hasNonFinite(f) };
    },
    assertions: [
      {
        name: 'temperature-weighted centre of mass rises (smaller j) before wall reflection',
        check({ history }) {
          const start = history[0].comJ;
          // measure over the first ~60% of the run, pre wall-reflection
          const early = history.slice(1, Math.max(2, Math.ceil(history.length * 0.6)));
          const minCom = Math.min(...early.map((h) => h.comJ));
          return {
            pass: minCom < start - 0.05,
            detail: `com ${start.toFixed(3)} -> ${minCom.toFixed(3)}`,
          };
        },
      },
      {
        name: 'field stays finite throughout the plume run',
        check({ history, sim }) {
          const ok = history.every((h) => h.finite) && !hasNonFinite(sim);
          return { pass: ok, detail: ok ? '' : 'non-finite value appeared' };
        },
      },
    ],
  },

  {
    id: 'hot-meets-cold',
    name: 'Hot meets cold',
    description: 'A hot left half and cold right half equalise by conduction with no flow.',
    gridSize: 40,
    steps: 40,
    opts: {
      kappa: 0.1,
      // Conduction is now conservative by construction (flux-form explicit
      // update in js/fluid.js `conduct`), so this scenario no longer needs to
      // crank `iter` for the energy-drift assertion — the solver default is fine.
      buoyancy: 0,
      temp0: (i) => (i <= HC_SPLIT ? HC_HOT : HC_COLD),
    },
    entities: [],
    record(f) {
      const { lo, hi } = interiorRange(f, f.temp);
      return { hot: hi, cold: lo, gap: hi - lo, energy: interiorSum(f, f.temp) };
    },
    assertions: [
      {
        name: 'hottest cell never gets hotter than it started',
        check({ history }) {
          const start = history[0].hot;
          const worst = Math.max(...history.map((h) => h.hot));
          return {
            pass: worst <= start + 1e-4,
            detail: `hot max ${worst.toFixed(4)} vs start ${start.toFixed(4)}`,
          };
        },
      },
      {
        name: 'hot/cold gap shrinks substantially by the end',
        check({ history }) {
          const start = history[0].gap;
          const end = history[history.length - 1].gap;
          return {
            pass: end < start * 0.7,
            detail: `gap ${start.toFixed(3)} -> ${end.toFixed(3)}`,
          };
        },
      },
      {
        name: 'interior thermal energy drift under 1%',
        check({ history }) {
          const start = history[0].energy;
          const end = history[history.length - 1].energy;
          const drift = Math.abs(end - start) / Math.abs(start);
          return { pass: drift < 0.01, detail: `drift ${(drift * 100).toFixed(3)}%` };
        },
      },
    ],
  },

  {
    id: 'boil-a-pool',
    name: 'Boil a pool',
    description: 'A pool of liquid water along the floor is heated until it boils and vapour fills the box.',
    gridSize: 44,
    steps: 260,
    opts: {
      dt: 0.1,
      kappa: 0.08,
      buoyancy: 0.08,
      capacity: 1,
      phaseChange: true,
      latentHeat: 3,
      boilTemp: 100,
      condenseTemp: 100,
      // a pool along the floor, near boiling; cooler air above.
      temp0: (i, j) => (j >= 36 ? 95 : 55),
      water0: (i, j) => (j >= 36 ? { liquid: 1, vapour: 0 } : { liquid: 0, vapour: 0 }),
      // crude pre-round-8 forcing: sustained heat into the floor cells. The
      // closed box warms until it holds vapour rather than raining it straight
      // back out — the water cycle's "kettle with the lid on".
      heat: (i, j) => (j >= 41 ? 5 : 0),
    },
    entities: [],
    record(f) {
      return {
        vapour: interiorSum(f, f.vapour),
        water: interiorSum(f, f.liquid) + interiorSum(f, f.vapour),
        finite: !hasNonFinite(f),
      };
    },
    assertions: [
      {
        name: 'total vapour rises measurably while heat is applied',
        check({ history }) {
          const start = history[0].vapour;
          const mid = history[Math.floor(history.length / 2)].vapour;
          const end = history[history.length - 1].vapour;
          return {
            pass: mid > start && end > start + 0.5,
            detail: `vapour ${start.toFixed(2)} -> mid ${mid.toFixed(2)} -> end ${end.toFixed(2)}`,
          };
        },
      },
      {
        name: 'total water (liquid + vapour) stays within 1% across the boil',
        check({ history }) {
          const w0 = history[0].water;
          const worst = Math.max(...history.map((h) => Math.abs(h.water - w0) / Math.abs(w0)));
          return { pass: worst < 0.01, detail: `worst water drift ${(worst * 100).toFixed(2)}%` };
        },
      },
      {
        name: 'the field stays finite throughout the boil',
        check({ history, sim }) {
          const ok = history.every((h) => h.finite) && !hasNonFinite(sim);
          return { pass: ok, detail: ok ? '' : 'non-finite value appeared' };
        },
      },
    ],
  },

  {
    id: 'rain-falls',
    name: 'Rain falls',
    description: 'A slab of liquid water suspended high in a gas box falls under gravity and collects below.',
    gridSize: 56,
    steps: 160,
    opts: {
      dt: 0.12,
      buoyancy: 0,
      gravity: 0.6,
      vapourBuoyancy: 0,
      phaseChange: false,
      capacity: 1,
      temp0: 20,
      water0: (i, j) => (j >= 5 && j <= 12 ? { liquid: 1, vapour: 0 } : { liquid: 0, vapour: 0 }),
    },
    entities: [],
    record(f) {
      return {
        comJ: phaseComJ(f, f.liquid),
        water: interiorSum(f, f.liquid) + interiorSum(f, f.vapour),
        finite: !hasNonFinite(f),
      };
    },
    assertions: [
      {
        name: 'liquid centre of mass moves downward (larger j) over the run',
        check({ history }) {
          const start = history[0].comJ;
          const end = history[history.length - 1].comJ;
          return { pass: end > start + 3, detail: `comJ ${start.toFixed(2)} -> ${end.toFixed(2)}` };
        },
      },
      {
        name: 'total water is conserved within 1% as it falls',
        check({ history }) {
          const w0 = history[0].water;
          const worst = Math.max(...history.map((h) => Math.abs(h.water - w0) / Math.abs(w0)));
          return { pass: worst < 0.01, detail: `worst water drift ${(worst * 100).toFixed(3)}%` };
        },
      },
      {
        name: 'field stays finite throughout the fall',
        check({ history, sim }) {
          const ok = history.every((h) => h.finite) && !hasNonFinite(sim);
          return { pass: ok, detail: ok ? '' : 'non-finite value appeared' };
        },
      },
    ],
  },

  {
    id: 'vapour-rises',
    name: 'Vapour rises',
    description: 'A pocket of vapour low in an air box rises on the composition buoyancy force alone — no heat.',
    gridSize: 56,
    steps: 160,
    opts: {
      dt: 0.12,
      buoyancy: 0,
      vapourBuoyancy: 0.6,
      gravity: 0.6,
      phaseChange: false,
      capacity: 1,
      temp0: 20,
      water0: (i, j) => (j >= 44 && j <= 51 ? { liquid: 0, vapour: 0.4 } : { liquid: 0, vapour: 0 }),
    },
    entities: [],
    record(f) {
      const r = interiorRange(f, f.temp);
      return {
        comJ: phaseComJ(f, f.vapour),
        vapour: interiorSum(f, f.vapour),
        tempSpread: r.hi - r.lo,
        finite: !hasNonFinite(f),
      };
    },
    assertions: [
      {
        name: 'vapour centre of mass moves upward (smaller j) over the run',
        check({ history }) {
          const start = history[0].comJ;
          const end = history[history.length - 1].comJ;
          return { pass: end < start - 3, detail: `comJ ${start.toFixed(2)} -> ${end.toFixed(2)}` };
        },
      },
      {
        name: 'no heat is applied — interior temperature stays uniform (rise is composition-driven)',
        check({ history, scenario }) {
          const worst = Math.max(...history.map((h) => h.tempSpread));
          const noHeat = !scenario.opts.heat && !scenario.opts.buoyancy;
          return { pass: noHeat && worst < 1e-6, detail: `worst temp spread ${worst.toExponential(2)}` };
        },
      },
      {
        name: 'total vapour is conserved within 0.5% while it rises',
        check({ history }) {
          const v0 = history[0].vapour;
          const worst = Math.max(...history.map((h) => Math.abs(h.vapour - v0) / Math.abs(v0)));
          return { pass: worst < 0.005, detail: `worst vapour drift ${(worst * 100).toFixed(3)}%` };
        },
      },
    ],
  },

  {
    id: 'still-pool',
    name: 'Still pool',
    description: 'A flat-bottomed pool of liquid at rest under gravity stays flat and still over a long run.',
    gridSize: 56,
    steps: 500,
    opts: {
      dt: 0.12,
      buoyancy: 0,
      gravity: 0.6,
      vapourBuoyancy: 0.6,
      phaseChange: false,
      capacity: 1,
      temp0: 20,
      water0: (i, j) => (j >= 38 ? { liquid: 1, vapour: 0 } : { liquid: 0, vapour: 0 }),
    },
    entities: [],
    record(f) {
      return {
        liquid: interiorSum(f, f.liquid),
        comJ: phaseComJ(f, f.liquid),
        speed: peakSpeed(f),
        finite: !hasNonFinite(f),
      };
    },
    assertions: [
      {
        name: 'total liquid stays constant within 0.5% over 500 steps (no creep)',
        check({ history }) {
          const l0 = history[0].liquid;
          const worst = Math.max(...history.map((h) => Math.abs(h.liquid - l0) / Math.abs(l0)));
          const finite = history.every((h) => h.finite);
          return { pass: finite && worst < 0.005, detail: `worst liquid drift ${(worst * 100).toFixed(3)}%` };
        },
      },
      {
        name: 'liquid centre of mass stays stationary within one cell',
        check({ history }) {
          const c0 = history[0].comJ;
          const worst = Math.max(...history.map((h) => Math.abs(h.comJ - c0)));
          return { pass: worst < 1.0, detail: `worst centroid shift ${worst.toFixed(3)} cells` };
        },
      },
      {
        name: 'peak speed stays bounded and does not grow over the run',
        check({ history }) {
          const speeds = history.map((h) => h.speed);
          const peak = Math.max(...speeds);
          const early = Math.max(...speeds.slice(20, 120));
          const late = Math.max(...speeds.slice(400));
          return {
            pass: Number.isFinite(peak) && peak < 0.05 && late <= early * 1.2 + 1e-6,
            detail: `peak ${peak.toExponential(2)}  early ${early.toExponential(2)} -> late ${late.toExponential(2)}`,
          };
        },
      },
    ],
  },
];
