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

const interiorSum = (f, arr) => {
  const { N, SIZE } = f;
  let s = 0;
  for (let j = 1; j <= N; j++) for (let i = 1; i <= N; i++) s += arr[IX(SIZE, i, j)];
  return s;
};

const interiorRange = (f, arr) => {
  const { N, SIZE } = f;
  let lo = Infinity, hi = -Infinity;
  for (let j = 1; j <= N; j++) for (let i = 1; i <= N; i++) {
    const v = arr[IX(SIZE, i, j)];
    if (v < lo) lo = v;
    if (v > hi) hi = v;
  }
  return { lo, hi };
};

// temperature-weighted vertical centre of mass (interior, 1-indexed rows).
// Weight is (temp - background) so a uniform field contributes nothing.
const tempComJ = (f, background) => {
  const { N, SIZE } = f;
  let wsum = 0, jsum = 0;
  for (let j = 1; j <= N; j++) for (let i = 1; i <= N; i++) {
    const w = Math.max(0, f.temp[IX(SIZE, i, j)] - background);
    wsum += w;
    jsum += w * j;
  }
  return wsum > 0 ? jsum / wsum : (N + 1) / 2;
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
      iter: 100,
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
];
