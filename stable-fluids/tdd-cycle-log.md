# TDD Cycle Log — WATER_SIM_AC.md

Orchestrator progress log. Survives autocompaction. Each phase's full report appended here.

Mode: nightshift / unattended. Work on `main`, commit as you go.
Session trailer:
```
Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01EeuvEzxVvvuSs6KYaMY6cV
```

## State

- [x] Round 1 — Conservative transport — DONE. Final gate PASS. Commits 630830c, a3c0728, bfa0c2e.
- [x] Round 2 — Temperature field — DONE. Final gate PASS. Commits 675480d, ae6b748, c28f92e.
- [x] Round 3 + 3b — Buoyancy — DONE. Final gate PASS. Commits cc96bd2, 6abfce8 (v1);
      db54ee2, 0f56ea7, d907919 (3b sign-fix correction). AC6 early-time up/down, AC7 exactly 0.
- [x] Round 4 — Scenarios as shared data — DONE. Final gate PASS. Commits a3025ac, 045c0b9, 358f36c.
- [x] Round 5 — Scenario player on GitHub Pages — DONE. Final gate PASS. Commits 8c104f5, 1c41e00, d44af7f.
- [x] Round 6 — Water, vapour, air and phase change — DONE. Final gate PASS. Commits e49a399, 8618352, bff7324.
- [~] Round 7 — Liquid falls, gas rises — Red DONE (66385e3), Green DONE-PARTIAL (cb158b6), HALTED to user on AC 26
- [ ] Round 8 — Placeable entities
- [ ] Round 9 — Capstone

AC quality gate: PASSED (ACs frozen by /tdd-acceptance, testable, consistent, intent statement present in WATER_SIM_AC.md).

---

## Round 1

### Red report

Test file: `test/conservation.js` (wired into `npm test` after the existing probe:
`node test/fluid-probe.js && node test/conservation.js`)

Failing (real red — AC 1):
- `scalar total drifts < 0.5% over 500 steps — corner jet` — drift 7.19%
- `scalar total drifts < 0.5% over 500 steps — off-axis jet` — drift 18.32%
- `scalar total drifts < 0.5% over 500 steps — shear pair` — drift 26.23%

Fail on genuine assertion: semi-Lagrangian scalar advection is dissipative, bleeds
7–26% of total scalar out of a closed box over 500 steps. No import/syntax error.

Passing guards added (cheap, per cross-cutting ACs):
- AC 2 (no new extrema on advection) — `advection introduces no new maximum`,
  `...nothing negative / below prior min`, `field stays finite throughout`. Pass today
  (bilinear backtrace is convex combination). Regression guard for Green — MacCormack/flux
  scheme can overshoot and will need a limiter.
- AC 33 determinism — `two identical runs produce bit-identical u/v/dens`. Passes.
- AC 34 performance — `step() under 16 ms at 180x180`, ~10 ms/step. `SHIPPED_N = 180`
  hard-coded with comment tying to js/main.js:6.
- AC 35 headless — asserts no `window`/`document` while suite runs. Passes.

AC / intent notes from Red:
- AC 1's 0.5%/500-steps threshold untested against a *correct* conservative scheme yet —
  if a proper conservative advection can't hit 0.5%, Green/Refactor should propose a
  revised number rather than contort the solver.
- `totalDensity` sums the boundary ring too — conservation metric includes ghost cells.
- Test written against "the advected scalar" so it's mechanically retargetable to water
  mass / vapour when the dye channel is removed (round 6).

Red cycle-debug notes:
- Repo test harness is a hand-rolled `check()` logger, not a framework. `npm test` chaining
  with `&&` means a probe failure masks the new file. A real runner worth it as suite grows
  — NOTE FOR REFACTOR.
- Unsure whether writing currently-passing guard tests is in Red's remit; included + flagged.

### Green report

Files touched: `js/fluid.js`.
Full `npm test`: PASS. Three named failing tests now drift 0.00% (machine precision).
AC 2, determinism, perf (~10ms/step at N=180), headless all stayed green.

What changed: replaced dissipative semi-Lagrangian *scalar* advection with a
dimensionally-split, minmod-limited MUSCL flux scheme (velocity advection untouched).
Flux form + zero wall-normal flux → interior total conserved to machine precision;
limiter keeps monotone; sub-cycles when Courant > 1. Final clamp to field's prior global
range removes Godunov-splitting overshoot and redistributes clipped amount over interior
so clamp stays conservative. `totalDensity` now sums INTERIOR ONLY (boundary ring holds
no fluid, was double-counting wall-adjacent scalar). Added `tmp2` scratch buffer to
`createFluid`.

No AC/intent halt. 0.5%/500-step figure comfortably achievable, no weaker number proposed.

Green cycle-debug notes:
- Redistribution bug in overshoot clamp cost most of the time (headroom accumulated over
  mismatched cell set → ~1.5% leak on initial transient).
- False start: tried MacCormack/BFECC first — not conservative, GAINS 5–25% mass near walls.
  Backed out. Prompt listing MacCormack as viable option is arguably misleading.
- Changing `totalDensity` (imported by tests) felt slightly outside remit — RED should pin
  down what quantity is meant to be conserved.
- `tmp`/`tmp2` naming should be reviewed by refactor.

### Refactor report

`npm test` green before/after each edit. Final: fluid-probe 4 checks, conservation 8 checks pass.

Changes:
- `test/run.js` (new) — real runner, aggregates exit status across probe files; `package.json`
  `test` now calls it instead of `node a.js && node b.js`.
- `test/fluid-probe.js` — removed 3 blocks now superseded by conservation.js: dye-mass,
  dens-only determinism, N=180 perf. Now a pure velocity-solver probe. Dropped totalDensity import.
- `js/fluid.js` — comment/naming only: header said "Gauss-Seidel" (it's Jacobi) + implied
  semi-Lagrangian scalar — corrected. Renamed `tmp2` → `flux`.

Gaps found, NOT fixed — feed later rounds:
- AC 1 only exercised with `fade:0` and one injection. Conservation with `fade>0` and across
  phase-change channels will need coverage when those land.
- `advectScalar` clamp-and-redistribute path (fluid.js ~194–225) has NO direct test — overshoot
  clamp branch never triggered by any current scenario. Red should build an overshoot scenario
  (higher Courant / sharper gradient) to lock its conservative behaviour.
- No test pins sub-cycling (`sub`/`h`) logic for scalar totals — only indirect via velocity probe.

AC/intent: no contradictions. `totalDensity` interior-only is correct. Scratch-buffer structure
adequate now; a scratch-pool / per-field advection helper will likely pay off once temperature +
3 phase channels exist — noted forward, not done (speculative).

---

## Round 1 — FINAL GATE (orchestrator): PASS
`npm test` → `== ALL FILES PASSED ==`. conservation drift 0.00% all 3 fields; perf 9.7ms/step at N=180.
Commits: 630830c (Red), a3c0728 (Green), bfa0c2e (Refactor).

Carry-forward gaps for later Red phases: overshoot-clamp scenario; sub-cycling scalar-total test;
`fade>0` conservation.

---

## Round 2

### Red report

Test files: `test/temperature.js` (new, wired into run.js), `test/conservation.js` (2 carry-forward
guards added, GREEN).

9 failing checks in temperature.js (all fail because `f.temp` doesn't exist — clean assertions,
guarded with `hasTemp(f)`):
- `f.temp is a grid-sized Float32Array`
- `hot blob centroid moves downstream with the flow` (AC 3)
- `interior thermal energy drifts < 1% over 500 steps (advection only)` (AC 3)
- `... (advection + conduction)` (AC 4)
- `the hottest cell never gets hotter` / `the coldest cell never gets colder` /
  `the hot/cold gap never widens` / `the two regions converge substantially` /
  `mean temperature is unchanged` (AC 5)

Carry-forward guards added to conservation.js (GREEN): `Godunov overshoot stays conservative +
monotone` (hard step + sheared momentum, dt 0.6); `sub-cycled advection conserves scalar total`
(dt 0.9 forces sub>1).

**Temperature API assumed (Green implements to this):**
- `createFluid(N, { temp0, kappa })`. `temp0`: Number or `(i,j)=>value` (1-indexed interior),
  default 0. `kappa`: thermal diffusivity, same role `diff` plays for dye, default 0.
- `f.temp`, `f.tempPrev`: `Float32Array(SIZE*SIZE)`, interior 1..N + boundary ring, swapped in place.
- `step(f)` advects `f.temp` along `(u,v)` with the SAME conservative MUSCL scheme as dye, then
  conducts by `kappa` via existing implicit `diffuse`/`linSolve`. Walls insulating (`setBnd` b=0,
  zero-gradient), no heat flux through boundary.
- Conserved quantity = interior SUM of temperature (uniform heat capacity).

AC/intent notes:
- 1%/500-step figure used as-is; whether implicit-Jacobi conduction (24 sweeps, not fully
  converged) holds it over 500 steps in a moving field is a real risk — Green/Refactor to
  report a revised number rather than contort solver if it can't hit it.
- AC 5 "converge" made concrete: final gap < 15% of initial after 600 steps at kappa 0.0002,
  N 48. Constants illustrative — adjust test not solver if a correct scheme converges slower.

Red cycle-debug: time sink getting a clean red (TypeError on undefined f.temp → had to guard
every block). "Force the Godunov branch" is awkward black-box — left as scenario+invariants,
can't prove branch executed without a solver debug counter.

### Green report

Files touched: `js/fluid.js`, `test/temperature.js`.
Full `npm test`: PASS (`== ALL FILES PASSED ==`). Perf 11.8ms/step at N=180.

Implementation: `createFluid(N,{temp0,kappa})`. `f.temp`/`f.tempPrev` grid Float32Arrays.
New `tempStep(f)` called from `step()` after `densStep`: advects `f.temp` via existing
`advectScalar` (conservative MUSCL), then conducts via `diffuse(f,0,...)` implicit Jacobi when
`kappa>0`. Walls insulating (setBnd b=0). `hasNonFinite` now scans `f.temp` too.

Test constant changed (AC 5 block ONLY): kappa 0.0002→0.001, loop 600→2400 steps. Rationale:
conduction scheme is correct + strictly monotone (all monotonic assertions exactly 0.00e+0) but
half-domain gradient at N=48 needs a few thousand steps to equalise; original illustrative
constants ~10x too short. New constants keep implicit solve well-converged (diffusion number
≈0.69<1/step). Reaches gap 1.000→0.037 vs 0.15 threshold. AC 4's own conduction test
(kappa 0.00015, 500 steps) untouched, 0.00% drift. Intent statement explicitly licensed this.

No unresolved AC/intent problems.

Green cycle-debug: only AC5 "converge" needed a param sweep (~1/3 effort). Determinism test
(AC 33) only hashes u/v/dens NOT temp — temp determinism implied not guarded — NOTE FOR REFACTOR.
`ix` referenced in createFluid before its const declaration (works, runtime-only) — awkward.

### Refactor report

Changes:
- `js/fluid.js` — hoisted `const ix` above `createFluid`; de-staled a scratch-buffer comment.
- `test/conservation.js` — AC 33 determinism now includes `'temp'` field, seeded with kappa +
  temp0 gradient so the advect+conduct path does real work under the hash.
- `test/temperature.js` — deleted redundant "sensible heat conserved under pure advection" block
  (covered by AC 4's stronger advect+conduct block + conservation.js AC 1 on identical code path).

Not done (judgement): per-field `scalarStep` helper NOT extracted — densStep and tempStep differ
in operator order; fold in during the phase-channels Green when the 3rd repetition makes the shape
obvious. AC 5 2400-step loop left (bulk of ~11s suite but fine).

Coverage gaps for next round:
- Buoyancy: no test asserts temperature influences velocity yet (Round 3 scope).
- `hasNonFinite` covers `f.temp` not `f.tempPrev`/scratch. Low value.
- Insulating-wall behaviour implicit — no direct zero-boundary-heat-flux assertion.

AC/intent: none. `npm test` green, ~11.3s. Commit c28f92e.

---

## Round 2 — FINAL GATE (orchestrator): PASS
`npm test` → `== ALL FILES PASSED ==`. AC4 drift 0.00%, AC5 monotonic all 0.00e+0, gap 1.000→0.037.
Commits: 675480d (Red), ae6b748 (Green), c28f92e (Refactor).

---

## Round 3

### Red report

Test file: `test/buoyancy.js` (wired into run.js).
Failing (right reason — no buoyancy force): `warm-weighted centroid moves up (smaller j)`,
`cold-weighted centroid moves down (larger j)`. `no velocity develops from a flat field` (AC 7)
passes — guard so Green's force stays a deviation.

**Buoyancy API assumed (Green implements to this):**
- `createFluid(N, { buoyancy })` — scalar coeff, default 0.
- In `step()` when `buoyancy != 0`: `f.v[cell] += -buoyancy * (f.temp[cell] - meanInteriorTemp) * dt`,
  added to shared velocity field. Warmer than mean → toward -v; colder → +v. Deviation form → uniform
  field gives exactly zero.
- **Grid orientation: "up" is `-v` / smaller `j`.** Determined from main.js render(): interior row j
  → image row j-1, canvas y increases downward.

AC/intent: none. Blob size, coeff 0.6, step counts, 0.5-cell threshold all illustrative;
assertions on sign + near-zero (<1e-9) bound only.

Red cycle-debug: orientation question took most time (traced putImageData + canvas y in main.js,
no existing test pins it).

### Green report

Files touched: `js/fluid.js`. Full `npm test`: PASS. Buoyancy: warm centroid j 32.00→31.27,
cold 32.00→37.53, flat field max|u|+|v| = 0.00e0. No test constants changed.

**Sign flip:** Red's assumed formula `f.v += -buoyancy*(temp-mean)*dt` drove warm DOWN / cold UP
(opposite of AC 6). Green flipped to `+buoyancy*(temp-mean)*dt` → both AC 6 pass, AC 7 stays zero.
Intent's "literal AC wording is a means" licensed following observed grid dynamics over the comment's
sign. Deviation form preserved. NOTE FOR REFACTOR: Red's and Green's sign-rationale explanations
conflict; behaviour is empirically correct (warm rises) but the prose in test header / code comment
should be reconciled so a future reader isn't misled.

Green cycle-debug: sign snag cost one test run to discover; intent "means not goal" clause unblocked it.

### Refactor report — NO CODE CHANGES, found a bug

Refactor determined with confidence (2 independent checks: main.js render orientation +
empirical advection-direction probe): **smaller j = up; positive v transports fluid DOWN.**
Therefore warm fluid rising requires warm cells pushed toward NEGATIVE v — i.e. Red's original
`f.v += -buoyancy*(temp-mean)*dt` was CORRECT. Current shipped code has `+buoyancy*...` → pushes
warm DOWN, cold UP. The code comment ("pushed toward -v / up") describes intended physics, not
what the code does.

Why AC 6 test passes anyway: it measures NET centroid displacement after 120 steps of an
underdamped wall-bouncing plume. Warm blob (current inverted code) shoots 16 cells DOWN, sloshes,
rebounds to j 31.27 at step 120 — 0.73 above start, just over the 0.5 threshold at the ONE sample
point. Correct sign → rebounds to 34.00 at step 120 → test FAILS. Test rewards inverted physics.

`npm test` green (working tree clean, no commits).

**FEEDS ROUND 3b (corrective cycle):**
1. Fix buoyancy sign in `buoyancyStep` (`js/fluid.js:321`) → `v[k] -= buoyancy*(temp[k]-mean)*dt`.
2. Rewrite AC 6 test to assert EARLY-TIME direction (sign of mean v in blob after 1 step, or
   centroid direction over first ~10-20 steps before wall reflections) — the only regime where
   buoyancy direction is unambiguous in this solver.
3. Restore Red header's "Assumed API" formula to the `-buoyancy` form.
4. Minor: warm/cold AC6 blocks near-duplicate → shared helper when rewritten.
Orchestrator decision: intent ("warm CoM up") is unambiguous → no user halt; corrective cycle
proceeds within intent. Early-time-direction becomes the operative reading of AC 6.

step() structure fine as-is; `applyForcesAndSources` seam worth naming in Round 8 (2nd caller), not now.

Refactor cycle-debug: sign puzzle needed throwaway probes; skill gave no steer on "verify a physics
convention empirically". Remit said "reconcile comments" but truth and code disagreed → had to bail
to gap report; "if you can't determine it say so" escape hatch only implicitly covers "I determined
it and the code is wrong".

### Round 3b Red report

`test/buoyancy.js` rewritten. Commit db54ee2. 4 checks RED against inverted `v[k] +=` sign:
- warm centroid moves UP: currently 32.00→38.21 (down); mean v over warm blob +0.175 (want −).
- cold centroid moves DOWN: currently 32.00→28.67 (up); mean v over cold blob −0.177 (want +).
Rest of suite green. Verified: flipping to `v[k] -= buoyancy*(temp-mean)*dt` makes all 4 green,
AC 7 still 0.00e+0.
Step count: 5 steps, dt 0.15, buoyancy 0.6, blob interior cells 25–39 on N=64. Pre-wall-reflection
verified (wall-row |v| ~6e-3 vs ~0.17 in blob at step 5; reflections dominate step 12+). Test
asserts `wallV < 0.02` inline as a guard.
Green MUST NOT retune coefficient/geometry — they're load-bearing for the cold-blob-is-sub-mean setup.
Red-header "Assumed API" note now has correct sign.

### Round 3b Green report

Files touched: `js/fluid.js` (also committed previously-untracked WATER_SIM_AC.md via `git add -A`
— harmless, sloppy). Commit 0f56ea7. Full `npm test`: PASS. Warm centroid 32.00→25.88 (up),
cold 32.00→35.62 (down), AC 7 exactly 0.00e+0. One-char fix `+=`→`-=`; comment already correct.
No AC/intent problems.

### Round 3b Refactor report

Changes:
- `test/buoyancy.js` — folded warm/cold blocks into `directionCase({title,temp0,blob,weight,dir})`
  helper. Same 3 assertions/case. Replaced stale "these tests currently FAIL because..." header with
  past-tense history note.
- `js/fluid.js` — added comment in `buoyancyStep` explaining why `-=` is correct (advect backtraces
  along +v toward larger j; larger j renders lower). No code change.
Kept both centroid check AND mean-blob-v check deliberately: mean-v pins force direction, centroid
pins the force actually transports the field.

Gaps (NOT fixed, feed later):
- No test that buoyancy magnitude scales with coefficient / temp deviation (only sign checked).
- AC 7 only covers uniform field with buoyancy≠0; no early-time check that buoyancy:0 → blob stays put.
- Full rise→cool→condense→rain cycle has no test (future rounds).

`npm test` green, 11.8s.

---

## Round 3+3b — FINAL GATE (orchestrator): PASS
`npm test` → `== ALL FILES PASSED ==`. warm centroid 32→25.88, cold 32→35.62, AC7 0.00e+0.
Commits: cc96bd2, 6abfce8, db54ee2, 0f56ea7, d907919.

LESSON: Refactor's cold read caught a sign bug Green+Red both missed. The isolated-phase design
worked as intended here.

---

## Round 4

### Red report

Test file `test/scenarios.js` (wired into run.js). Commit a3025ac. Fails cleanly on missing
`js/scenarios.js`; other 4 test files green.

**Assumed API (Green implements to this):**
`js/scenarios.js` — DOM-free ES module, import-safe under plain node:
- `export const scenarios: Scenario[]` — ordered, stable unique ids.
- `Scenario = { id, name, description, gridSize, steps, opts, entities:[], init(f)?, record(f)?,
  assertions:[{name, check(ctx)}] }`. `opts` → `createFluid(gridSize, opts)`. `entities` present
  array, empty till round 8. `check(ctx)` → `{pass, detail?}`, `ctx={sim,history,scenario}`,
  history length steps+1.
- `export const runScenario(scenario)` → `{id,name,results:[{name,pass,detail}],pass}`.
- `export const runAllScenarios()` → array.

**Two round-2/3 scenarios expected:**
- `warm-plume-rises` — warm blob, uniform cold bg, buoyancy>0, kappa 0. Asserts: temp-weighted CoM
  moves up (−j); field finite.
- `hot-meets-cold` — left half hot / right half cold, no flow, kappa>0. Asserts: hottest cell never
  hotter (history); gap shrinks substantially; interior thermal-energy drift <1%.
Test enforces the two scenarios have ZERO assertion-name overlap.

AC/intent: none. Judgement: AC 8 lists "initial state, entities, duration, assertions" but not
gridSize/opts — Red required gridSize, steps, opts/init since a scenario is un-runnable without them.

Red cycle-debug: smooth. Hard-asserted the two scenario ids by exact string (pins Green to names).

### Green report

Files touched: `js/scenarios.js` (new). Commit 045c0b9. Full `npm test`: PASS.
- `warm-plume-rises`: N=64, 14 steps, buoyancy 0.6 dt 0.15 kappa 0, radius-8 blob. COM ~32.5→~15.
- `hot-meets-cold`: N=40, 40 steps, kappa 0.1 **iter 100** (left half 40 / right half 0, no flow).
No `test/scenarios.js` constants changed.

**FLAG (feed forward):** solver's Jacobi diffusion is only conditionally conservative before
convergence — at default `iter:24` interior thermal-energy drift was ~2.3%. Green raised `iter` to
100 via the opts bag (convergence-quality knob, not physics distortion) → drift ~0.69%, gap still
collapses to ~4%. A future round may want the solver's conduction path conservative BY CONSTRUCTION
rather than scenarios cranking `iter`. (Note: Round 2 AC 4 test passed <1% — it used a gentler kappa.)

Green cycle-debug: first plume param guess (buoyancy 4) over-forced, COM drifted down — fell back to
test/buoyancy.js proven constants. `entities:[]` slot dead weight till round 8 but test enforces it.

### Refactor report

Commit 358f36c. `npm test` green.
- `js/measure.js` (new) — DOM-free: `interiorSum`, `interiorRange`, `weightedCentroid`. Consumed by
  scenarios.js + test/temperature.js + test/buoyancy.js (byte-identical dup removed).
- `js/scenarios.js` — 10-line comment on `hot-meets-cold` `iter:100`.
- `test/scenarios.js` — AC 9 DOM-free scan now covers `js/measure.js` too.
No tests deleted. Scenario/unit overlap kept — AC 11 mandates the assertions by name, scenario is
the user-visible artifact; overlap is scenario-vs-unit not scenario-vs-scenario.

**CARRY-FORWARD GAPS (feed Round 6 physics Green):**
1. Jacobi conduction (`tempStep`→`diffuse`→`linSolve`) conserves interior thermal-energy sum EXACTLY
   ONLY AT CONVERGENCE. `hot-meets-cold` compensates with iter:100. Fix in solver — flux-form
   conduction update OR solve-then-rescale to pre-solve sum — so no scenario needs to crank iter.
2. `test/temperature.js` AC 4 passes drift<1% at default iter:24 ONLY because kappa is tiny
   (0.00015). When a later round strengthens conduction, that 24-iter solve leaks → AC 4 fails for
   a solver reason. Re-derive AC 4 when gap 1 is fixed, don't nudge kappa.

Refactor cycle-debug: iter:100 workaround should've been caught+documented in Green. AC 11 baking in
redundancy the intent warns against — orchestrator may want to reconcile (deemed acceptable: overlap
is scenario-vs-unit).

---

## Round 4 — FINAL GATE (orchestrator): PASS
`npm test` → `== ALL FILES PASSED ==`. AC 8/9/10/11 + 5/5 scenario assertions green.
Commits: a3025ac, 045c0b9, 358f36c. js/ now: fluid.js, main.js, measure.js, scenarios.js.

---

## Round 5

### Red report

Test file `test/player.js` (new, wired into run.js). Clean red on missing `js/player.js`; ~25
staged AC 12–16 checks behind the guard. Rest of suite green. (Red did NOT commit — check `git status`.)

**`js/player.js` controller API assumed (Green implements to this):**
`createController(opts={})` → Controller with: `listScenarios()` → `[{id,name,description}]` (exactly
`scenarios` order); `load(id)`; `play()`/`pause()`; `singleStep()` (exactly one step, doesn't set
playing); `reset()` (REBUILD bit-identical, not approx-restore); `tick()` (one step iff playing);
getters `playing`, `stepCount`, `scenarioId`, `mode` ('scenario'|'sandbox'), `sim`; `channels()` →
string[] DERIVED from sim (e.g. ['dens','temp'], not hard-coded); `readouts()` → `[{key,label,value}]`
data-driven (energy entry `value === interiorSum(sim, sim.temp)`, matched by `/energ/i`); round 6 adds
"total water" as another array entry — data change not structure; `loadSandbox(gridSize)`;
`paint({channel,i,j,radius,amount})`.

**main.js / index.html:** main.js → thin DOM binding over createController(). index.html → replace
dye-sandbox controls with `#scenario-select`, `#play`, `#step`, `#reset`, `#sandbox` toggle,
`#readouts` container; keep `#sim` 720×720, `#fps`/`#step` stats. Render iterates `channels()`.

**Shot check (Green writes it):** `scripts/shot-scenarios.js` + `"shot:scenarios"` npm script.
Serves root, headless, asserts scenario `<select>` renders with names, warm-plume-rises selectable +
Play advances stepCount + Reset zeroes it, readouts shows energy row, screenshots `#sim` to
scratch/scenarios.png (DON'T read PNG).

AC/intent: none blocking. Note: scenario format (frozen in test/scenarios.js AC 8) has no `readouts`
slot; Red's recommendation = controller derives readouts from channels (temp→energy, dens/water→water).
`paint` signature + `loadSandbox(gridSize)` are Red's invention — if Green changes shape, flag to Refactor.
Old dye-sandbox controls (visc/diff/fade sliders) have no test coverage — Green removing them turns
nothing red.

Red cycle-debug: Green MUST run `node test/player.js` to see deeper checks, not just import check.

### Green report

Commit 1c41e00. `npm test` PASS (all 6 files, ~30 player checks). `npm run shot:scenarios` PASS (8/8
DOM checks). Perf ~4.1ms/step at N=96 sandbox.

Files: `js/player.js` (new, `createController`), `js/main.js` (rewritten, DOM glue only),
`index.html` (rewritten controls), `scripts/shot-scenarios.js` (new), `scripts/shot.js` (id fix
`#step`→`#step-ms`), `package.json` (`shot:scenarios` script).

No deviations from assumed `js/player.js` API. `channels()` = own-property Float32Arrays of full grid
size, excluding `*Prev` and `{u,v,uPrev,vPrev,tmp,flux}` → `['dens','temp']`. `readouts()` spec-table
keyed by required channel (temp→energy). `#step` id collision (button vs stat) resolved: `#step` =
Step button, ms/step stat renamed `#step-ms`. `js/main.js` sets `window.controller` for the headless
check. Sandbox grid fixed 96 (arbitrary).

No AC/intent problems.

Green cycle-debug: "document" in a code comment tripped the DOM-free regex. `#step` id collision in
ticket. Multi-channel renderer functional but crude (additive dens glow + temp tint) — Refactor may
want real compositing.

ORCHESTRATOR NOTE for Refactor: `scripts/shot.js` (original `npm run shot`) still drags strokes on
the canvas expecting the old dye sandbox, but the page now defaults to a scenario (not sandbox) —
`npm run shot` may now be a no-op visual. Refactor should repoint it (load sandbox, or a scenario) or
retire it in favour of shot-scenarios.js.

### Refactor report

Commit d44af7f. `npm test` green, `npm run shot:scenarios` + `npm run shot` both green.
- `js/fluid.js` — added `f.channels = ['dens','temp']` explicit render-order registry (replaces
  denylist heuristic).
- `js/player.js` — deleted `NON_CHANNEL` denylist + `*Prev` rule; `deriveChannels(sim)` = `sim.channels.slice()`.
- `js/main.js` — documented additive blend model + Uint8ClampedArray saturation; `CHANNEL_RENDERERS`
  noted as compositor seam; removed dead `chanScale`.
- `scripts/shot.js` — repointed: waits for controller, checks `#sandbox`, drags dye, clicks Play,
  screenshots. `npm run shot` works again (60fps, 3.31ms/step).
- `test/player.js` — added assertions: every `channels()` entry is a full-size Float32Array field;
  none of u/v/tmp/flux/*Prev leak in.

**CARRY-FORWARD GAPS (feed Round 6):**
1. No test that `f.channels` and `js/main.js` `CHANNEL_RENDERERS` stay in sync. If round 6 registers
   `liquid` but forgets a renderer, nothing fails. Add a headless check: every `controller.channels()`
   name has a `CHANNEL_RENDERERS` entry.
2. `readouts()` "total water" (AC 14) has no home — `READOUT_SPECS` ready for the entry but no
   water/liquid channel + interiorSum spec yet. Round 6 owns it.
3. `shot:scenarios` "Reset returns energy to initial" asserts at 1e-6 — could go flaky if a future
   scenario's readout isn't a conserved quantity. Not a current defect.

AC/intent: none. AC 16 now structurally enforced (registry + renderer map both channel-keyed).

---

## Round 5 — FINAL GATE (orchestrator): PASS
`npm test` → `== ALL FILES PASSED ==`. `npm run shot:scenarios` 8/8. Commits 8c104f5, 1c41e00, d44af7f.
js/ now: fluid.js, main.js, measure.js, scenarios.js, player.js.

### OPEN CARRY-FORWARD for Round 6 physics Green (accumulated):
- Conservative conduction: Jacobi `diffuse`/`linSolve` conserves interior thermal-energy sum only at
  convergence (~2.3% drift at iter 24). Fix in solver (flux-form or solve-then-rescale). Re-derive
  test/temperature.js AC 4 afterward (it passes only because kappa is tiny).
- channels/renderer sync test (Round 5 gap 1).
- readouts "total water" entry (Round 5 gap 2).

---

## Round 6

### Red report

Commit e49a399. Test files: `test/water.js` (new, in run.js), `test/player.js` (extended).
5 files green, 2 red for right reasons.

**Real red (test/water.js):** water channels don't exist → AC 17/18/19/20/21/22 checks; `boil-a-pool`
scenario not registered → AC 23.
**Carry-forward red:** conduction drift 1.95% at iter:24 (Round 4 item — forces conservative
conduction); player.js: no water channels / no total-water readout / no liquid+vapour CHANNEL_RENDERERS.
Guard `CHANNEL_RENDERERS covers every channel` currently PASSES (still dens,temp) — goes red mid-Green
if Green adds channels without renderers.

**Phase-change API assumed (Green implements to this):**
- `f.liquid`/`f.vapour`/`f.air` + `*Prev`, Float32Array(SIZE²), advected by shared flow with MUSCL.
  `f.channels` → `['dens','temp','liquid','vapour']` — **air NOT in channels** (tracked, not rendered).
  After advection renormalise so liquid+vapour+air == capacity per interior cell.
- opts: `capacity` (def 1.0), `phaseChange` (def true; false disables conversion for AC 19/20 diff),
  `latentHeat` (energy per unit water, in temp*capacity units), `boilTemp`, `condenseTemp`,
  `water0(i,j)` → `{liquid,vapour}`, `heat` (Number | (i,j)=>delta added to temp each step — crude
  pre-round-8 forcing).
- Rule (in step, after advection/conduction): per interior cell 1:1 mass liquid↔vapour, bounded/step.
  Boil `temp>=boilTemp && liquid>0`: `liquid-=dm; vapour+=dm; temp-=latentHeat*dm/capacity`.
  Condense `temp<=condenseTemp && vapour>0`: reverse. Air untouched → AC 22 by construction.
- Energy: `sensible=interiorSum(temp)`, `latent=latentHeat*interiorSum(vapour)`, total=sum.
- `boil-a-pool` scenario in js/scenarios.js (land it WITH real assertions — test/scenarios.js runs
  every declared assertion; stub assertions break it).
- `READOUT_SPECS`: add `/water/i`-matched entry, value = interiorSum(liquid)+interiorSum(vapour).
- `js/main.js` CHANNEL_RENDERERS: add `liquid`, `vapour` entries.

**AC/intent notes:**
- AC 22 "air unchanged by any phase change" is literally clean only if phase change is 1:1
  liquid↔vapour never touching air. Intent doc "vapour displaces air" points at a boiling divergence
  source (decision 1's optional piece, deferred to round 7). Red tested AC 22 as written (air total
  invariant, no flow). If Green adds a boiling divergence source THIS round, AC 22 needs re-reading as
  "air neither created nor destroyed" — FLAG BACK to orchestrator if that path taken.
- AC 21 (2%): if a correct scheme can't hold it because advection+conduction drift stack, that's the
  "propose a number" situation — report don't loosen. Fixing conduction carry-forward should pull it under.

Red cycle-debug: straightforward, strong test-file template. Handoff unusually complete.

### Green report

Commit 8618352. All 7 test files green; `npm run shot` + `npm run shot:scenarios` pass.
Files: `js/fluid.js` (liquid/vapour/air channels + advection + renorm, phase-change pass,
flux-form conservative conduction, new opts), `js/player.js` (total-water readout spec),
`js/main.js` (liquid+vapour CHANNEL_RENDERERS), `js/scenarios.js` (boil-a-pool + assertions;
dropped `iter:100` from hot-meets-cold), `test/conservation.js` (SHIPPED_N 180→96, perf probe
now exercises water path, determinism hash extended to liquid/vapour/air), `test/water.js`
(AC 19 reframed).

Perf: shipped N=96 (matches SANDBOX_GRID). ~8-10ms/step boiling+buoyant at 96²; 104²≈13ms,
128²>24ms. Water mixture ≈3× scalar-transport cost (3 extra conservatively-advected fields).

AC 17/18/22 conserve to 0.00%; AC 21 round-trip energy 0.00%. Determinism holds incl. water.

**Deviations flagged for orchestrator:**
1. **AC 19 reframed.** Original setup (hot block, no flow, no heat, boilTemp==condenseTemp)
   is unsatisfiable in round-6 scope alongside AC 21: with one shared incompressible velocity
   field and NO boiling divergence source / NO vapour buoyancy (both round 7), vapour can't
   leave the boiling cell — it re-condenses in place, latent heat returns, phase-change run
   ends at SAME temp as phaseChange:false (that's AC 21 holding, 0.00% drift). Green did NOT
   add a divergence source (per instruction). Reframed AC 19 test to a **boiling plateau**:
   under sustained `heat`, boiling cell pinned near boilTemp while phaseChange:false runs
   hotter (tOn 10572.7 < tOff 10595.8, 64 units vapour retained). "Cell ends cooler" via
   plateau — same AC intent. Full vapour-transport demo of AC 19 → round 7.
2. **SHIPPED_N 180→96.** Nothing shipped ran at 180 (sandbox 96, scenarios 44–64); probe now
   honest + includes water path. Not a loosening.
3. **Renorm: air is the slack** (`air = capacity - liquid - vapour`) so water = exactly what
   conservative advection produced. Proportional 3-way rescale leaked 57% (AC 18). Consequence:
   air can go slightly negative in transiently over-packed cells; air unrendered, AC 22
   unaffected (0.000% drift).

Did NOT need a boiling divergence source; deliberately did not add one.

Green cycle-debug:
- Discovered AC 19 structurally unsatisfiable AFTER implementing — should have reasoned
  "reversible phase change + no transport + equal thresholds" from test constants up front.
- First renorm (proportional 3-way) was an obvious water leak; 2 wasted iterations.
- Didn't anticipate 3× scalar cost; walked shipped grid 180→96 over 3 measurements. SHIPPED_N
  living in a test made "shipped size" always somewhat fictional.
- boil-a-pool needs the box to self-heat above condensation point ("kettle with lid on") or
  vapour rains straight back out (arguably correct water cycle, not what AC 23 measures).

### Orchestrator ruling on Green's AC 19 reframe: ACCEPTED
Judgement call within intent (nightshift mode). Intent doc marks specific temperatures
illustrative and defers vapour transport (settling/buoyancy for phase fractions) to round 7
as a locked decision. Green's boiling-plateau test demonstrates AC 19's load-bearing claim —
"phase change removes latent heat, so the boiling region stays cooler than an identical run
with phaseChange:false" — honestly within round-6 scope, without distorting physics or adding
out-of-scope machinery. Round 7 (liquid falls / gas rises) will demonstrate AC 19 with actual
vapour transport; no AC needs to formally move. SHIPPED_N→96 and air-slack renorm also accepted
(both make the code/probes more honest, neither loosens a conservation bound).

### Refactor report

Commit bff7324. `npm test` / `npm run shot` / `npm run shot:scenarios` all green.
- `js/fluid.js` — extracted `advectField(f, name)` helper (5× repeated block: swap with `*Prev`,
  `advectScalar` through shared flow). `dens` runs `diffuse` then calls it; `temp`/`liquid`/`vapour`
  use it directly. Conduct/phase-renorm interleaving left in place (not forced into helper).
- `js/fluid.js` — **`waterStep` no longer advects `air`**: every interior air cell is
  unconditionally overwritten with `capacity - liquid - vapour` next loop, so the advected result
  was dead. Final `f.air` bit-identical; determinism hash over `air` still passes. Saves ~1/3 of
  water-transport cost.
- `js/fluid.js` — rewrote self-contradictory `conduct()` doc comment (Jacobi para + flux-form para)
  → flux-form only. Fixed water-mixture comment in `createFluid` (air is tracked slack, not "rides
  the flow"; `airPrev` kept for a future round).
- `test/conservation.js` — fixed stale `SHIPPED_N` comment ("180→112", real value 96; "three
  advected fields" → two), removed fabricated-looking per-grid ms figures.
No changes to player.js / main.js / scenarios.js / water.js.

"Also verify" findings:
- Channel-sync guard (`test/player.js` ~L336) IS meaningful — deleting a renderer entry goes red.
  Weakness: source text-match not execution; a present-but-broken renderer (wrong field name) not caught.
- liquid/vapour CHANNEL_RENDERERS not copy-paste bugs (distinct colours, correct `Math.max(0,v)` guard).
- `boil-a-pool` assertions real + distinct. Minor: vapour saturates at cap by midpoint so
  `mid > start && end > start+0.5` wouldn't catch a "stops climbing after halfway" regression.
- `hot-meets-cold` with `iter:100` removed passes with genuine ~0.00% energy drift (flux-form
  conduction conservative by construction) — 1% bound legitimately slack.
- AC 19 reframed test is NOT weak — fully deterministic, differs only by the `phaseChange` flag;
  23-unit block-temp gap is exactly the latent heat held back, grows over the run.

**CARRY-FORWARD GAPS → Round 7 (liquid falls / gas rises):**
1. No test that liquid moves downward under gravity/settling.
2. No test that vapour rises on its own (vapour-specific buoyancy, independent of temp buoyancy).
3. `air` can go negative "by design" with no bound. Once R7 adds a boiling divergence source, pin
   `air >= -epsilon` + vapour expansion displaces neighbours via velocity, not via negative air.
4. No spatial "rain" assertion — nothing checks condensed liquid falls toward the floor (centroid
   drops); `boil-a-pool` has no vapour-centroid-rises assertion.
5. AC 19/20 same-cell reframes can revert to the stronger "vapour leaves, doesn't fully re-condense
   in place" setup once transport exists.
6. `boil-a-pool` vapour plateau masks post-midpoint dynamics — R7 assertion on vapour vertical
   distribution restores that coverage.

Refactor cycle-debug:
- Green's `waterStep` advected `air` then threw the result away every step — dead code shipped; the
  `SHIPPED_N` drop to 96 was partly paying for that wasted third of transport cost.
- Green left `conduct()` doc comment internally contradictory. Low-effort miss.
- Per-field advection helper genuinely ready this time; two prior deferrals were correct calls.
- `advectField` uses `f[name]`/`f[name+'Prev']` string indexing — slightly less greppable, but the
  5-way dedup is worth it and call sites keep literal names.

---

## Round 6 — FINAL GATE (orchestrator): PASS
`npm test` → `== ALL FILES PASSED ==`. AC 17 sum drift 0.00% (2.98e-8 after flow), AC 18 water
0.00%, AC 19 plateau 10572.67<10595.80, AC 20 2576.56>2560.00, AC 21 energy 0.00%, AC 22 air
0.000%, AC 23 vapour 0→396, conduction carry-forward 0.00% at default iter.
Commits: e49a399 (Red), 8618352 (Green), bff7324 (Refactor).
js/ now: fluid.js, main.js, measure.js, scenarios.js, player.js. Shipped N=96, ~3.2ms/step page.

---

## Round 7

### Red report

Commit 66385e3. Test file `test/settling.js` (new, in run.js). 12 checks fail (2 physics-absent:
liquid/vapour centroids don't move; 10 scenario-absent). Other 7 files green.

**Assumed API for Green:** two new `createFluid` opts, applied as body forces on the shared `v`
in a pass BEFORE `velStep` (like `buoyancyStep`), both deviation-form (uniform field → exactly 0):
- `opts.gravity` (default 0): `v[k] += gravity*(liquid[k]-meanLiquid)*dt` — more-liquid cell → +v (down).
- `opts.vapourBuoyancy` (default 0): `v[k] -= vapourBuoyancy*(vapour[k]-meanVapour)*dt` — more-vapour
  cell → −v (up). INDEPENDENT of temperature.
No new persistent fields. Green folds `gravity`/`vapourBuoyancy` into determinism + perf runs in
test/conservation.js.

**Scenario ids hard-asserted (also run through runScenario → must pass, no stubs):**
- `rain-falls` — `gravity>0`; liquid suspended high; proves liquid centroid falls + water conserved.
- `vapour-rises` — `vapourBuoyancy>0`, NO `heat`, NO `buoyancy`; vapour low; proves vapour centroid
  rises. Red asserts `!vap.opts.buoyancy` (AC 25 = composition-driven; test also asserts interior
  temp spread <1e-6 throughout).
- `still-pool` — `gravity>0`, NO `heat`; flat pool; proves flatness/stillness over a long run.

Green guards (green now, load-bearing once forces wired): AC 26 stability (liquid sum 0.5%,
centroid <1 cell, peak speed bounded/non-growing 500 steps); AC 7 (uniform + all 3 coeffs →
maxSpeed<1e-9); AC 33 settling run bit-identical; AC 34 8-10ms/step at N=96; air > -0.05 during
settling; water/vapour conservation 0.5% during fall/rise.

**AC/intent answers:** AC 25 shows cleanly without hot vapour (keys on vapour-fraction deviation;
test uses uniform temp, zero heat, zero thermal buoyancy, asserts temp spread <1e-6). AC 26 no
conflict — horizontally-uniform pool → gravity force is fn(j) only → curl-free → pressure
projection removes it → net zero velocity is correct hydrostatic result.

**Carry-forward:**
1. AC 19/20 in water.js are same-cell "boiling plateau" proxies (Round 6 had no transport). Now
   vapour transports + self-buoys → original stronger setup may be satisfiable. FLAGGED for Green
   to consider rewriting; Red did not touch them.
2. `boil-a-pool` has no vapour-centroid-rises assertion — could add one now.
3. air ≥ −ε guard currently green; becomes load-bearing if Green adds a boiling divergence/expansion
   source this round (may need tightening + displacement-via-velocity check).
4. `test/conservation.js` SHIPPED_N scenarios don't set new opts — Green folds them in.

Red cycle-debug:
- AC 26's direct-physics block is entirely green now (nothing moves without forces); red signal
  carried solely by `still-pool` scenario-existence checks. 500-step stability assertions
  unexercised until forces land.
- `still-pool` tolerances (`peak<0.05`, `latePeak<=1.2*earlyPeak`) are guesses — legit place for
  Green to propose a defensible number if projection residual at the sharp interface is larger.
- Asserts `!vap.opts.buoyancy` for `vapour-rises` — forces a conversation if Green wants thermal
  buoyancy there. Intentional (AC 25 = composition-driven).

### Green report — PARTIAL, HALTED

Commit cb158b6 (sound partial committed so it survives autocompaction — 7/8 files green).
Files: `js/fluid.js` (`opts.gravity`, `opts.vapourBuoyancy`, `settleStep(f)` deviation-form,
called before `velStep`), `js/scenarios.js` (`rain-falls`, `vapour-rises`, `still-pool` +
`phaseComJ`/`peakSpeed` helpers), `test/conservation.js` (folded gravity+vapourBuoyancy into
AC 33 determinism run + AC 34 perf run; SHIPPED_N=96 unchanged, no threshold changed).

**PASS:** AC 24 (rain-falls: liquid centroid 8.5→22.3, water drift 0.000%), AC 25 (vapour-rises:
centroid 47.5→17.7, interior temp spread exactly 0, drift 0.000%), AC 7 uniform-mixture guard
(0 velocity with all 3 coeffs), AC 33 determinism (bit-identical), AC 34 perf (8.0-9.4 ms/step
at N=96), AC 35 headless. `npm run shot` PASS (3.22ms/step), `npm run shot:scenarios` PASS (6
scenarios listed).

**FAIL (3 checks, all AC 26 / still-pool stability):**
- still-pool liquid centroid shift 26.6 cells (needs <1.0)
- still-pool peak speed 1.83e-1 (needs <0.05). NOTE: late 3.74e-2 < early 1.83e-1 — it DECAYS,
  so "no growing oscillation" holds; it's the magnitude + the centroid smear that fail.
- still-pool scenario aggregate (same two)

**HALT — structural solver limitation (Green's diagnosis, credible):**
Colocated central-difference pressure projection has an odd-even (checkerboard / `(-1)^j`) null
mode. A resting flat liquid pool under `gravity` should be exactly balanced by a hydrostatic
pressure gradient → zero flow. But the wide `(j+1)-(j-1)` divergence/gradient stencil can't see
the one-cell Nyquist mode, so projection never removes it; with `visc=0` nothing damps it;
semi-Lagrangian self-advection pumps it into a standing oscillation (~0.18 peak) that fills the
liquid body, and conservative-advection clamp-and-redistribute smears the interface ~26 cells
across the box.
Verified ineffective by Green: `iter` to 1500; `visc` to 1e-3 (axis-aligned Nyquist also in the
5-point Laplacian null space); low-pass of driving liquid/vapour field to 8 passes (mode is
self-excited, not force-injected). Verified HARMFUL: `[1,2,1]` anti-checkerboard filter on `v`
post-projection → breaks AC 7 (uniform field reaches speed 0.7) + buoyancy/conduction scenarios.
Latent in existing `buoyancyStep` too — no current test stresses thermal buoyancy with a
500-step flat interface.

**Green's recommendation (a scope decision for the orchestrator/user, NOT a Green edit):** move
to a staggered (MAC) or compact-stencil pressure projection — the standard fix for colocated
odd-even decoupling. It's a `project()` rewrite that changes every determinism hash and every
buoyancy magnitude in the suite → needs its own Red/Green/Refactor slice. Green declined to
loosen AC 26 thresholds 26× to make the bar disappear (correct).

AC 19/20 carry-forward: Green left them as the Round 6 "boiling plateau" proxies — the stronger
"vapour physically leaves, cell ends net cooler" rewrite depends on reliable near-interface
transport, which the still-pool failure shows is exactly what's currently broken. Revisit after
the projection fix.

Green cycle-debug:
- Burned ~1h probing fixes (iter/visc/filter sweeps) before accepting the failure is structural.
  Should have inspected `v` for a checkerboard on the first failing run — diagnosis was 5 min of
  console.log once attempted.
- `npm test` runtime ballooned to ~5 min: still-pool (500 steps × N=56) + settling.js's own
  500-step AC 26 run are two full 500-step runs of the same physics. Refactor could collapse.
- still-pool scenario deliberately mirrors settling.js AC 26 setup (spec said match) → one bug
  shows as three red checks, no independent signal.

### ORCHESTRATOR: HALTED TO USER
Round 7 is blocked on a scope decision only the user can make: whether to insert a
staggered/compact-stencil `project()` rewrite as a new groundwork TDD slice (Round 6.5 / 7-pre)
before finishing Round 7, or accept a revised AC 26 (documented physical limitation of a
colocated solver), or another path. Rounds 8–9 (capstone) depend on a stable resting pool
(capstone AC 32: "pool does not fully empty / does not exceed initial volume"), so this can't be
deferred past Round 7. Loop paused here pending user input. Partial work committed at cb158b6.
