// Round 4 — Scenarios as shared data.
//
//   node test/scenarios.js   (run via: npm test)
//
// Intent: the demo is becoming a water sim with phase change. The user opens the
// GitHub Pages site, picks a NAMED scenario, and watches it play — THE SAME
// scenarios the automated suite asserts against — with play/pause/step/reset and
// a live conserved-quantity readout. This round pulls that scenario layer ahead
// of the physics so every later round ships something visible AND asserted.
//
// The strongest structural call in the project (AC doc): ONE declarative
// definition per scenario; TWO consumers (this headless suite + the round-5 web
// page); adding a scenario touches NEITHER consumer. "Scenarios are the tests" —
// this suite iterates the scenario list and runs each scenario's own declared
// assertions.
//
//   * AC 8  — a scenario is declarative data in one module: initial field state,
//     entity placements, run duration (in steps), and its list of assertions.
//   * AC 9  — the same module is consumed by both the headless suite and the web
//     page; it must therefore be DOM-free and import-safe under plain node.
//   * AC 10 — this suite runs every scenario's declared assertions.
//   * AC 11 — at least two scenarios exercise rounds 2–3 behaviour (buoyancy
//     direction; hot-meets-cold equalisation).
//
// ------------------------------------------------------------------------
// Assumed API for the Green phase to implement to  (js/scenarios.js — a DOM-free
// ES module, safe to `import` under plain node with no window/document):
//
//   export const scenarios : Scenario[]        // ordered list, stable ids
//
//   Scenario = {
//     id          : string   // stable slug, unique across the list
//     name        : string   // human label for the page's picker
//     description : string    // one line shown beside the name
//     gridSize    : number    // N passed to createFluid
//     steps       : number    // run duration, in solver steps
//     opts        : object    // opts bag for createFluid (dt, kappa, buoyancy…)
//     entities    : array     // heat/cold source placements — EMPTY until round 8,
//                             //   but the slot exists in the format now
//     init(f)     : function  // optional; seeds field state beyond opts.temp0,
//                             //   given the freshly-created fluid
//     record(f)   : function  // optional; returns the per-step snapshot value
//                             //   pushed onto `history` (assertions that need a
//                             //   trend declare this; default records null)
//     assertions  : Assertion[]   // non-empty
//   }
//
//   Assertion = {
//     name        : string
//     check(ctx)  : ({ pass: boolean, detail?: string })
//   }
//   ctx = { sim, history, scenario }
//     sim     : the fluid after `steps` steps
//     history : [record(f) @ step 0, … , record(f) @ step `steps`]  (length steps+1)
//
//   export const runScenario(scenario) : {
//     id, name,
//     results : [{ name, pass, detail }]   // one per declared assertion, in order
//     pass    : boolean                    // results.every(r => r.pass)
//   }
//   // builds the fluid from gridSize+opts, runs init, steps `steps` times while
//   // recording history, then evaluates each assertion. USED BY BOTH CONSUMERS.
//
//   export const runAllScenarios() : ReturnType<runScenario>[]
//
// AC 11 — the two round-2/3 scenarios this suite expects (by id):
//   * "warm-plume-rises"  — warm blob, uniform cold background, buoyancy on,
//     kappa 0. Asserts: temperature-weighted centre of mass moves UP (−j);
//     field stays finite. (Guards round-3 buoyancy DIRECTION.)
//   * "hot-meets-cold"    — left half hot, right half cold, no flow, conduction
//     on. Asserts: the hottest cell never gets hotter (needs history); the hot/
//     cold gap shrinks substantially; interior thermal energy drift < 1%.
//     (Guards round-2 equalisation + conservation.)
//   Each scenario earns DISTINCT assertions — buoyancy direction is asserted
//   once, conservation is asserted once — not the same invariant five times.
//
// The tests below currently fail because js/scenarios.js does not exist yet.
// ------------------------------------------------------------------------

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

let failures = 0;
const check = (name, ok, detail = '') => {
  console.log(`  ${ok ? 'PASS' : 'FAIL'} ${name}${detail ? `  (${detail})` : ''}`);
  if (!ok) failures++;
};

const here = dirname(fileURLToPath(import.meta.url));
const modPath = join(here, '..', 'js', 'scenarios.js');
const modUrl = 'file://' + modPath;

// --- load the module (guarded so a missing file is a clean check failure) ---
let mod = null;
let loadErr = null;
try {
  mod = await import(modUrl);
} catch (e) {
  loadErr = e;
}

check('js/scenarios.js exists and imports under plain node',
  mod != null, loadErr ? String(loadErr.message || loadErr).split('\n')[0] : '');

// AC 9 — DOM-free / import-safe. Importing above already ran with no window or
// document defined (this is plain node); a scenario module that reached for the
// DOM at import time would have thrown. Also scan the source for DOM globals so
// a lazily-referenced `document`/`window` can't slip through.
{
  let src = null;
  try { src = readFileSync(modPath, 'utf8'); } catch { /* covered above */ }
  if (src == null) {
    check('js/scenarios.js source is DOM-free (no window/document/localStorage)', false, 'source unreadable');
  } else {
    const domHit = src.match(/\b(document|window|localStorage|navigator|HTMLElement|requestAnimationFrame)\b/);
    check('js/scenarios.js source is DOM-free (no window/document/localStorage)',
      domHit == null, domHit ? `references ${domHit[1]}` : '');
  }
  check('importing js/scenarios.js does not require a DOM', loadErr == null);
}

if (mod == null) {
  console.log(`\n${failures} CHECK(S) FAILED`);
  process.exit(1);
}

const { scenarios, runScenario, runAllScenarios } = mod;

// --- AC 8: a scenario is declarative data with the required slots ----------
{
  console.log('every scenario is declarative data with the required fields (AC 8)');
  check('`scenarios` is a non-empty array', Array.isArray(scenarios) && scenarios.length > 0,
    Array.isArray(scenarios) ? `${scenarios.length} scenarios` : `got ${typeof scenarios}`);

  const list = Array.isArray(scenarios) ? scenarios : [];
  const ids = new Set();
  let allWellFormed = list.length > 0;
  for (const s of list) {
    const ok =
      s && typeof s === 'object' &&
      typeof s.id === 'string' && s.id.length > 0 &&
      typeof s.name === 'string' && s.name.length > 0 &&
      typeof s.description === 'string' && s.description.length > 0 &&
      Number.isFinite(s.gridSize) && s.gridSize > 0 &&
      Number.isFinite(s.steps) && s.steps > 0 &&
      Array.isArray(s.entities) &&                       // entity-placement slot exists
      Array.isArray(s.assertions) && s.assertions.length > 0 &&
      s.assertions.every((a) => a && typeof a.name === 'string' && typeof a.check === 'function');
    if (!ok) { allWellFormed = false; console.log(`    (malformed: ${s && s.id})`); }
    if (s && typeof s.id === 'string') ids.add(s.id);
  }
  check('each scenario has id/name/description/gridSize/steps/entities/assertions',
    allWellFormed);
  check('scenario ids are unique', ids.size === list.length, `${ids.size} ids / ${list.length} scenarios`);
  check('a scenario declares its initial field state (init fn or initial spec)',
    list.length > 0 && list.every((s) => typeof s.init === 'function' || s.initial != null || s.opts != null),
    'expected init(f), or an `initial` / `opts` state spec, on every scenario');
}

// --- AC 10: the suite runs every scenario's own declared assertions --------
{
  console.log('the headless suite runs every declared assertion of every scenario (AC 10)');
  check('runScenario is a function', typeof runScenario === 'function');
  check('runAllScenarios is a function', typeof runAllScenarios === 'function');

  if (typeof runScenario === 'function' && Array.isArray(scenarios)) {
    let total = 0, passed = 0, coveredEvery = true;
    for (const s of scenarios) {
      let res;
      try {
        res = runScenario(s);
      } catch (e) {
        console.log(`    (${s.id} threw: ${String(e.message || e).split('\n')[0]})`);
        coveredEvery = false;
        continue;
      }
      const declared = Array.isArray(s.assertions) ? s.assertions.length : 0;
      const got = res && Array.isArray(res.results) ? res.results.length : -1;
      if (got !== declared) {
        coveredEvery = false;
        console.log(`    (${s.id}: ${got} results for ${declared} declared assertions)`);
      }
      for (const r of (res && res.results) || []) {
        total++;
        if (r.pass) passed++;
        else console.log(`    FAIL ${s.id} :: ${r.name}${r.detail ? `  (${r.detail})` : ''}`);
      }
    }
    check('runScenario returns exactly one result per declared assertion, for every scenario',
      coveredEvery);
    check('every declared assertion of every scenario passes', total > 0 && passed === total,
      `${passed}/${total} assertions passed`);
  }
}

// --- AC 11: at least two scenarios exercise rounds 2–3 --------------------
{
  console.log('at least two scenarios exercise rounds 2–3 behaviour (AC 11)');
  const list = Array.isArray(scenarios) ? scenarios : [];
  const byId = new Map(list.map((s) => [s.id, s]));

  check('there are at least two scenarios', list.length >= 2, `${list.length} scenarios`);

  const plume = byId.get('warm-plume-rises');
  const hotcold = byId.get('hot-meets-cold');
  check('a "warm-plume-rises" scenario exists (round 3 — buoyancy direction)', plume != null);
  check('a "hot-meets-cold" scenario exists (round 2 — equalisation + conservation)', hotcold != null);

  check('"warm-plume-rises" turns buoyancy on', plume != null && plume.opts && plume.opts.buoyancy > 0,
    plume ? `buoyancy = ${plume.opts && plume.opts.buoyancy}` : '');
  check('"hot-meets-cold" turns conduction on', hotcold != null && hotcold.opts && hotcold.opts.kappa > 0,
    hotcold ? `kappa = ${hotcold.opts && hotcold.opts.kappa}` : '');

  // The two scenarios must not just re-assert one invariant. Their assertion
  // name sets should be substantially different.
  if (plume && hotcold) {
    const a = new Set(plume.assertions.map((x) => x.name));
    const b = new Set(hotcold.assertions.map((x) => x.name));
    const overlap = [...a].filter((n) => b.has(n));
    check('the two scenarios assert distinct things (no copy-pasted invariant set)',
      overlap.length === 0, overlap.length ? `shared: ${overlap.join(', ')}` : '');
  }

  // The behaviours themselves — asserted here directly so RED is meaningful even
  // before the scenario objects carry their own checks.
  if (typeof runScenario === 'function' && plume) {
    const r = runScenario(plume);
    check('"warm-plume-rises" reports its centre-of-mass-rises assertion as passing',
      r.pass === true && r.results.some((x) => x.pass),
      r.results.map((x) => `${x.name}:${x.pass ? 'ok' : 'X'}`).join(' | '));
  }
  if (typeof runScenario === 'function' && hotcold) {
    const r = runScenario(hotcold);
    check('"hot-meets-cold" reports its equalisation + conservation assertions as passing',
      r.pass === true, r.results.map((x) => `${x.name}:${x.pass ? 'ok' : 'X'}`).join(' | '));
  }
}

console.log(failures === 0 ? '\nALL CHECKS PASSED' : `\n${failures} CHECK(S) FAILED`);
process.exit(failures === 0 ? 0 : 1);
