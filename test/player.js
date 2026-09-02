// Round 5 — Scenario player / controller (DOM-free half).
//
//   node test/player.js   (run via: npm test)
//
// Intent: the user opens the GitHub Pages site, picks a NAMED scenario from a
// list — THE SAME scenarios the automated suite asserts against — watches it
// play with play / pause / single-step / reset, sees a live readout of the
// conserved quantities, and has a sandbox for painting field state.
//
// AC 35 keeps `npm test` browser-free, so the page logic is split: a DOM-free
// controller module (js/player.js) owns which scenario is loaded, the
// play/pause/step/reset state machine, advancing the sim, and computing the live
// readout values from the sim + scenario. js/main.js stays a thin DOM binding
// (built + checked separately via `npm run shot`). This file unit-tests the
// controller under plain node.
//
// Slice covered here (AC 12–16):
//   * AC 12 — the controller lists the available scenarios by name + description,
//     and the list is exactly the shared js/scenarios.js list (adding a scenario
//     touches neither consumer).
//   * AC 13 — load(id) builds the scenario from its initial state; play advances
//     the step count over ticks; pause halts it; singleStep advances exactly one;
//     reset restores the initial state BIT-IDENTICALLY (ties to determinism).
//   * AC 14 — a live readout reports the scenario's conserved quantity ("total
//     energy" this round) and its value matches interiorSum over the raw field,
//     recomputed as the scenario plays so drift is visible. The readout set is
//     data-driven: each entry is { key, label, value }, so adding "total water"
//     in round 6 is a data change, not a structure change.
//   * AC 15 — sandbox mode starts empty; painting mutates the field; reset (to
//     empty) clears it.
//   * AC 16 — the controller exposes the sim's channels generically (derived from
//     the sim, not a hard-coded ['dens']), so the renderer can iterate whatever
//     channels the sim currently has.
//
// ------------------------------------------------------------------------
// Assumed API for the Green phase to implement to (js/player.js — a DOM-free ES
// module, safe to `import` under plain node; NO document / window / rAF):
//
//   export function createController(opts = {}) : Controller
//
//   Controller = {
//     // --- scenario catalogue (AC 12) ---
//     listScenarios() : [{ id, name, description }]   // from js/scenarios.js, in order
//
//     // --- loading + transport (AC 13) ---
//     load(id)      : void   // build fluid from the scenario, run its init, step 0
//     play()        : void   // playing = true
//     pause()       : void   // playing = false
//     singleStep()  : void   // advance exactly one solver step; does not set playing
//     reset()       : void   // rebuild from the loaded scenario's initial state
//     tick()        : void   // called once per animation frame; advances one solver
//                            //   step IFF playing, else no-op
//
//     // --- observable state ---
//     get playing()    : boolean
//     get stepCount()  : number    // solver steps since load / reset
//     get scenarioId() : string | null
//     get mode()       : 'scenario' | 'sandbox'
//     get sim()        : fluid     // the live fluid, for the renderer
//     channels()       : string[]  // field-channel names the sim currently has
//                                  //   (e.g. ['dens', 'temp']) — derived, not literal
//
//     // --- live readouts (AC 14) ---
//     readouts() : [{ key, label, value }]   // conserved quantities, recomputed live
//
//     // --- sandbox (AC 15) ---
//     loadSandbox(gridSize) : void  // mode = 'sandbox', every channel all-zero
//     paint({ channel, i, j, radius, amount }) : void   // add `amount` into a
//                                   //   (2*radius+1) square of `channel` at (i,j)
//   }
//
// The tests below currently fail because js/player.js does not exist yet.
// ------------------------------------------------------------------------

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { interiorSum } from '../js/measure.js';
import { scenarios as sharedScenarios } from '../js/scenarios.js';

let failures = 0;
const check = (name, ok, detail = '') => {
  console.log(`  ${ok ? 'PASS' : 'FAIL'} ${name}${detail ? `  (${detail})` : ''}`);
  if (!ok) failures++;
};

const here = dirname(fileURLToPath(import.meta.url));
const modPath = join(here, '..', 'js', 'player.js');

// --- load the module (guarded so a missing file is a clean check failure) ---
let mod = null;
let loadErr = null;
try {
  mod = await import('file://' + modPath);
} catch (e) {
  loadErr = e;
}

check('js/player.js exists and imports under plain node',
  mod != null, loadErr ? String(loadErr.message || loadErr).split('\n')[0] : '');

// AC 35 — the controller module must be DOM-free (it is a `npm test` dependency
// and also the page's shared logic).
{
  const domRe = /\b(document|window|localStorage|navigator|HTMLElement|requestAnimationFrame)\b/;
  let src = null;
  try { src = readFileSync(modPath, 'utf8'); } catch { /* missing covered above */ }
  if (src == null) {
    check('js/player.js source is DOM-free', false, 'source unreadable');
  } else {
    const hit = src.match(domRe);
    check('js/player.js source is DOM-free (no window/document/rAF)',
      hit == null, hit ? `references ${hit[1]}` : '');
  }
}

if (mod == null) {
  console.log(`\n${failures} CHECK(S) FAILED`);
  process.exit(1);
}

const { createController } = mod;
check('createController is a function', typeof createController === 'function');

if (typeof createController !== 'function') {
  console.log(`\n${failures} CHECK(S) FAILED`);
  process.exit(1);
}

// --- AC 12: lists the shared scenarios by name + description ---------------
{
  console.log('the controller lists the available scenarios by name and description (AC 12)');
  const c = createController();
  let list = [];
  try { list = c.listScenarios(); } catch (e) {
    check('listScenarios() runs', false, String(e.message || e).split('\n')[0]);
  }
  check('listScenarios() returns one entry per shared scenario, in order',
    Array.isArray(list) && list.length === sharedScenarios.length &&
      list.every((e, i) => e && e.id === sharedScenarios[i].id),
    `got ${Array.isArray(list) ? list.length : typeof list}, shared ${sharedScenarios.length}`);
  check('each listed scenario carries a non-empty name and description',
    list.length > 0 && list.every((e) =>
      typeof e.name === 'string' && e.name.length > 0 &&
      typeof e.description === 'string' && e.description.length > 0));
  check('listed name + description match the shared module (single source of truth)',
    list.length === sharedScenarios.length && list.every((e, i) =>
      e.name === sharedScenarios[i].name && e.description === sharedScenarios[i].description));
}

// --- AC 13: load from initial state; play / pause / step / reset ----------
const firstId = sharedScenarios[0].id;

{
  console.log('load(id) builds the scenario from its initial state at step 0 (AC 13)');
  const c = createController();
  c.load(firstId);
  check('scenarioId reflects the loaded scenario', c.scenarioId === firstId, String(c.scenarioId));
  check('mode is "scenario" after load', c.mode === 'scenario', String(c.mode));
  check('stepCount is 0 immediately after load', c.stepCount === 0, String(c.stepCount));
  check('not playing until play() is called', c.playing === false, String(c.playing));
  check('sim exists with the scenario grid size',
    c.sim && c.sim.N === sharedScenarios[0].gridSize,
    c.sim ? `N = ${c.sim.N}` : 'no sim');
}

{
  console.log('play advances over ticks; pause halts; singleStep advances exactly one (AC 13)');
  const c = createController();
  c.load(firstId);

  // paused: ticks do nothing
  c.tick(); c.tick();
  check('ticks while paused do not advance the sim', c.stepCount === 0, String(c.stepCount));

  // playing: each tick is one solver step
  c.play();
  check('playing is true after play()', c.playing === true);
  c.tick(); c.tick(); c.tick();
  check('three ticks while playing advance the step count by three',
    c.stepCount === 3, String(c.stepCount));

  // pause: freezes again
  c.pause();
  const held = c.stepCount;
  c.tick(); c.tick();
  check('ticks after pause() do not advance the step count',
    c.stepCount === held, `${held} -> ${c.stepCount}`);

  // singleStep: exactly one, and it does not resume playback
  c.singleStep();
  check('singleStep() advances the step count by exactly one',
    c.stepCount === held + 1, `${held} -> ${c.stepCount}`);
  check('singleStep() leaves the controller paused', c.playing === false);
}

{
  console.log('reset restores the loaded scenario to a bit-identical initial state (AC 13 / determinism)');
  const c = createController();
  c.load(firstId);

  // snapshot every channel of the freshly-loaded sim
  const chans = c.channels();
  check('channels() lists more than just density (derived from the sim, AC 16)',
    Array.isArray(chans) && chans.includes('dens') && chans.includes('temp'),
    Array.isArray(chans) ? chans.join(',') : String(chans));

  const snap0 = {};
  for (const ch of chans) snap0[ch] = Float32Array.from(c.sim[ch]);

  c.play();
  for (let n = 0; n < 12; n++) c.tick();
  check('the sim actually evolved before reset',
    chans.some((ch) => !eq(c.sim[ch], snap0[ch])), 'no channel changed over 12 steps');

  c.reset();
  check('stepCount is back to 0 after reset', c.stepCount === 0, String(c.stepCount));
  check('playing is false after reset', c.playing === false);

  let bitIdentical = true;
  for (const ch of chans) if (!eq(c.sim[ch], snap0[ch])) bitIdentical = false;
  check('every channel is bit-identical to the freshly-loaded state after reset',
    bitIdentical, 'reset must rebuild, not approximately restore');

  // determinism: replaying the same steps from reset lands on the same state
  c.play();
  for (let n = 0; n < 12; n++) c.tick();
  const replay = {};
  for (const ch of chans) replay[ch] = Float32Array.from(c.sim[ch]);
  c.reset();
  c.play();
  for (let n = 0; n < 12; n++) c.tick();
  let deterministic = true;
  for (const ch of chans) if (!eq(c.sim[ch], replay[ch])) deterministic = false;
  check('replaying 12 steps from reset reproduces the same field exactly', deterministic);
}

// --- AC 14: live readout of the scenario's conserved quantity -------------
{
  console.log('a live readout reports total energy and tracks the raw field as it plays (AC 14)');
  const c = createController();
  c.load(firstId);

  const r0 = c.readouts();
  check('readouts() returns a non-empty array of { key, label, value }',
    Array.isArray(r0) && r0.length > 0 && r0.every((e) =>
      e && typeof e.key === 'string' && typeof e.label === 'string' && Number.isFinite(e.value)),
    Array.isArray(r0) ? JSON.stringify(r0) : String(r0));

  const energy = r0.find((e) => /energ/i.test(e.key) || /energ/i.test(e.label));
  check('there is a total-energy readout for the loaded scenario', energy != null);

  const expected0 = interiorSum(c.sim, c.sim.temp);
  check('the energy readout equals interiorSum over the raw temperature field',
    energy != null && Math.abs(energy.value - expected0) <= 1e-6 * (Math.abs(expected0) + 1),
    energy ? `readout ${energy.value} vs interiorSum ${expected0}` : '');

  // recomputed live — after stepping, it matches the new field, not the old value
  c.play();
  for (let n = 0; n < 8; n++) c.tick();
  const rN = c.readouts();
  const energyN = rN.find((e) => /energ/i.test(e.key) || /energ/i.test(e.label));
  const expectedN = interiorSum(c.sim, c.sim.temp);
  check('after 8 steps the energy readout still equals interiorSum over the live field',
    energyN != null && Math.abs(energyN.value - expectedN) <= 1e-6 * (Math.abs(expectedN) + 1),
    energyN ? `readout ${energyN.value} vs interiorSum ${expectedN}` : '');
}

// --- AC 15: sandbox mode -------------------------------------------------
{
  console.log('sandbox mode starts empty; paint mutates a channel; reset clears it (AC 15)');
  const c = createController();
  c.loadSandbox(32);
  check('mode is "sandbox" after loadSandbox()', c.mode === 'sandbox', String(c.mode));

  const chans = c.channels();
  const allZero = () => chans.every((ch) => interiorSum(c.sim, c.sim[ch]) === 0);
  check('every channel is empty (interior sum 0) in a fresh sandbox', allZero(),
    chans.map((ch) => `${ch}:${interiorSum(c.sim, c.sim[ch])}`).join(' '));

  c.paint({ channel: 'dens', i: 16, j: 16, radius: 3, amount: 1 });
  check('painting density raises that channel\'s interior sum above zero',
    interiorSum(c.sim, c.sim.dens) > 0, String(interiorSum(c.sim, c.sim.dens)));

  c.reset();
  check('reset in sandbox mode returns every channel to empty', allZero(),
    chans.map((ch) => `${ch}:${interiorSum(c.sim, c.sim[ch])}`).join(' '));
  check('sandbox mode is retained after reset', c.mode === 'sandbox', String(c.mode));
}

// bit-identical Float32Array compare (reads back the exact stored bits)
function eq(a, b) {
  if (!a || !b || a.length !== b.length) return false;
  const ba = new Uint8Array(a.buffer, a.byteOffset, a.byteLength);
  const bb = new Uint8Array(b.buffer, b.byteOffset, b.byteLength);
  for (let i = 0; i < ba.length; i++) if (ba[i] !== bb[i]) return false;
  return true;
}

console.log(failures === 0 ? '\nALL CHECKS PASSED' : `\n${failures} CHECK(S) FAILED`);
process.exit(failures === 0 ? 0 : 1);
