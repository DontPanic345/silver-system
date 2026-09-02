// Round 5 — Scenario player / controller (DOM-free half).
//
// Owns which scenario is loaded, the play/pause/step/reset state machine,
// advancing the sim, exposing the sim's channels generically, and computing the
// live conserved-quantity readouts from the sim + scenario. js/main.js is a thin
// DOM binding over this. This module stays free of browser globals — it is a
// `npm test` dependency (AC 35) as well as the page's shared logic.

import { createFluid, step } from './fluid.js';
import { interiorSum } from './measure.js';
import { scenarios } from './scenarios.js';

// Readout descriptors, keyed by the channel that must be present for them to
// apply. Adding "total water" in a later round is another entry here — a data
// change, not a structural one (AC 14).
const READOUT_SPECS = [
  { channel: 'temp', key: 'energy', label: 'Total energy', field: 'temp' },
];

// Displayable channels come straight off the solver's explicit registry
// (js/fluid.js `f.channels`) — no shape-guessing denylist to rot as later rounds
// add liquid/vapour/air plus their scratch buffers (AC 16).
function deriveChannels(sim) {
  return Array.isArray(sim.channels) ? sim.channels.slice() : [];
}

export function createController(opts = {}) {
  let sim = null;
  let scenarioId = null;
  let mode = 'scenario';
  let playing = false;
  let stepCount = 0;
  let gridSize = null;

  // Rebuild the fluid for the currently-loaded scenario from its initial state.
  // Called by both load() and reset(), so reset is a true rebuild — bit-identical
  // Float32Array bytes to a fresh load, not an approximate restore (AC 13 /
  // determinism).
  function buildScenario(id) {
    const scenario = scenarios.find((s) => s.id === id);
    if (!scenario) throw new Error(`unknown scenario: ${id}`);
    gridSize = scenario.gridSize;
    sim = createFluid(scenario.gridSize, { ...opts, ...(scenario.opts || {}) });
    if (typeof scenario.init === 'function') scenario.init(sim);
    scenarioId = id;
    mode = 'scenario';
    stepCount = 0;
    playing = false;
  }

  function buildSandbox(n) {
    gridSize = n;
    sim = createFluid(n, { ...opts });
    scenarioId = null;
    mode = 'sandbox';
    stepCount = 0;
    playing = false;
  }

  function advanceOne() {
    step(sim);
    stepCount++;
  }

  return {
    listScenarios() {
      return scenarios.map((s) => ({
        id: s.id,
        name: s.name,
        description: s.description,
      }));
    },

    load(id) {
      buildScenario(id);
    },

    loadSandbox(n) {
      buildSandbox(n);
    },

    play() {
      playing = true;
    },

    pause() {
      playing = false;
    },

    singleStep() {
      if (!sim) return;
      advanceOne();
    },

    reset() {
      if (mode === 'sandbox') buildSandbox(gridSize);
      else buildScenario(scenarioId);
    },

    tick() {
      if (playing && sim) advanceOne();
    },

    paint({ channel, i, j, radius, amount }) {
      const arr = sim[channel];
      if (!(arr instanceof Float32Array)) throw new Error(`unknown channel: ${channel}`);
      const { N, SIZE } = sim;
      for (let b = -radius; b <= radius; b++) {
        for (let a = -radius; a <= radius; a++) {
          const ci = i + a;
          const cj = j + b;
          if (ci < 1 || ci > N || cj < 1 || cj > N) continue;
          arr[ci + SIZE * cj] += amount;
        }
      }
    },

    channels() {
      return sim ? deriveChannels(sim) : [];
    },

    readouts() {
      if (!sim) return [];
      const present = new Set(deriveChannels(sim));
      return READOUT_SPECS
        .filter((spec) => present.has(spec.channel))
        .map((spec) => ({
          key: spec.key,
          label: spec.label,
          value: interiorSum(sim, sim[spec.field]),
        }));
    },

    get playing() { return playing; },
    get stepCount() { return stepCount; },
    get scenarioId() { return scenarioId; },
    get mode() { return mode; },
    get sim() { return sim; },
  };
}
