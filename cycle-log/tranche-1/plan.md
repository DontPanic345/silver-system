# Tranche 1 plan — Physics

**Planned:** 2026-09-05T10:20+12:00

## 0. What was folded forward from tranche 0

Read: `cycle-log/tranche-0/closeout.md`, `cycle-log/tranche-0/m0.4/closeout.md`,
`src/math.rs`, `src/timestep.rs`, `src/lib.rs`, `Cargo.toml`.

- `Scalar = f32` and `FixedTimestep` exist, are unit-tested, and are already
  exercised by real running code (the M0.1 hello-world's `tick_and_draw`).
  Tranche 1 keeps using both rather than re-deciding them.
- `Vec2` and `GridIndex` exist and are unit-tested but **not yet exercised by
  running code** — named explicitly in tranche 0's closeout as this tranche's
  gap to close honestly, via M1.1's real grid, not by forcing them into
  unrelated code. This is not a mandate to bolt them onto something artificial;
  M1.1 building an actual grid *is* the honest close, and it happens
  naturally as the first thing this tranche needs anyway.
- Tranche 0 fixed two process defects mid-run and folded them into
  `cycle-contract`/`cycle-milestone`/`cycle-round` already (fork-vs-cold-agent
  for self-dispatch; strict one-round-at-a-time dispatch). Both are already
  live in the skill text I've read — nothing further to fold forward on that
  front, just to keep honoring them.
- A live GitHub Pages deployment and a minimal wasm-bindgen canvas renderer
  already exist (`src/lib.rs`, `www/`, `.github/workflows/deploy-pages.yml`).
  M1.1's "minimal renderer good enough to watch a scenario" builds on this
  pipeline rather than standing up a new one.
- No exit ramps were taken in tranche 0, so there's no unresolved "goal proved
  unattainable" thread to patch back into before pushing forward here.

## 1. Intent, restated

Tranche 1 gives the world matter that moves because of forces: gravity,
pressure, buoyancy, conduction — not scripted rules. It is the substrate every
later tranche stands on, so getting it *approximately physically right* now is
cheaper than discovering it's wrong once chemistry and biology depend on it.
The previous JS attempt (`stable-fluids/`) got through transport, temperature
and buoyancy and stalled on pressure — this tranche's centre of gravity is
proving pressure/incompressible flow actually works this time (M1.4), with
the milestones before it (substrate, granular solids, liquids) each building
what that milestone needs and each independently useful and testable on its
own.

**Serving the north star.** A resting pool that's actually still, sand that
piles and then genuinely stops, water that finds its own level — these are
the first believable things in the whole project, and they only read as
believable if they hold under scrutiny nobody is watching (5,000-step resting
tests with no human in the loop), not just in a single demo frame. This is
also where "the universe is full off-screen" first becomes checkable, because
M1.1 puts a viewer on it early enough that every later milestone in every
later tranche can be watched, not just measured.

## 2. What this tranche needs beyond PLAN.md's sketch

Applying the "does not support the north star without this" test:

- **Closing the `Vec2`/`GridIndex` gap is necessary, not optional-nice**:
  M1.1's grid representation literally cannot exist without a grid index
  type, and cell-to-cell forces need a vector type. This isn't new scope —
  it's tranche 0's own primitives finally meeting their first real caller.
- **Nothing else is being folded in beyond PLAN.md's tranche-1 sketch.**
  PLAN.md's M1.1–M1.7 breakdown already matches what this tranche's intent
  needs: substrate → granular → liquid → pressure → gas → temperature →
  consolidation, each building on the last, each independently scenario-
  testable. I looked for reach items and didn't find any worth naming; the
  plan as written is already scoped tightly to this tranche's job.

**Deferred, not folded in** (named here so the relevant future tranche's
planner finds it):

- Dissolved/mixed composition, moisture-in-solids, permeation — explicitly
  tranche 2's job (M2.5), and PLAN.md already says composition is per-cell,
  not region-mixed. Tranche 1's material table needs a `dissolves_in` /
  `permeable` hook to exist later, but tranche 1 itself does no dissolving.
- GPU execution — CLAUDE.md says don't engineer for it until the evidence
  demands it. M1.7 asks for *groundwork* (structure-of-arrays, no
  globally-sequential algorithm) but not an actual GPU backend; that stays
  deferred until a tranche's target requires it.
- Any rendering beyond "good enough to watch a scenario" (M1.1). Tranche 4
  owns the real viewer — overlays, camera, interaction. M1.1's renderer is a
  debugging tool, and should stay cheap and minimal on purpose.

## 3. Tranche targets (measurable)

Restated from PLAN.md, sharpened with concrete tolerances where PLAN.md left
them to planning:

1. **U-pipe**: water in a U-shaped pipe (one arm filled, connected at the
   bottom, both arms open to air) levels to within 1 cell of height
   difference between the two arms, and holds that level (no arm's surface
   height changes by more than 1 cell) for 5,000 consecutive steps.
2. **Mass conservation**: total mass in every closed scenario (no open
   boundary) is conserved to within 0.1% of its starting value over 10,000
   steps. (0.1% chosen as "tight enough nobody would call it drift" per
   PLAN.md's own framing — not zero, not loose enough to hide a leak; M1.1
   or M1.3, whichever first has a real mass-conserving scenario, is where
   this gets its first empirical check and can tighten or loosen the number
   with evidence.)
3. **Energy conservation**: total energy in a closed thermal scenario is
   conserved to within 0.5% over 10,000 steps (looser than mass's 0.1%
   because temperature/buoyancy coupling is new and noisier; M1.6 measures
   and can tighten this once real data exists).
4. **Resting stability**: a resting configuration (pool, pile, sealed gas)
   shows no cell-state change, no velocity above a stated small epsilon, and
   no visible checkerboard pattern, for 5,000 steps after reaching rest.
5. **Every primitive scenario passes headless**, asserted by a test, with no
   human judgement call in the assertion path.
6. **Performance**: a step of the reference grid (size to be fixed at M1.1,
   since no grid exists yet to measure) completes within a stated per-step
   budget on this dev machine; the number and the grid size are recorded at
   M1.1 and re-checked at M1.7.

## 4. Milestones

Ordered so each is buildable on what precedes it; each after M1.1 is
independently watchable via M1.1's renderer.

### M1.1 — Substrate and harness
The bones: grid representation (closes the `Vec2`/`GridIndex` gap), the
material table as data, the scenario harness (one definition, headless +
viewer consumers), a fixed-timestep step function, and the minimal renderer.
**First milestone a human can watch anything from** — its own target says so
explicitly. Also the milestone that fixes the reference grid size and
records the first per-step budget number, since tranche target 6 needs a
number to hold later milestones to.

### M1.2 — Granular solids
Sand/powder falling and piling to a stable angle of repose; static solids
that just hold; the at-rest/asleep distinction (resting matter costs
nothing). First scenario-driven milestone; exercises the harness built in
M1.1 for real.

### M1.3 — Liquids and the free surface
Liquid transport with a sharp gas interface; a genuinely still resting pool
(the exact failure that halted the JS attempt); a falling column that finds
a level. Builds directly on M1.2's transport mechanics but adds the
free-surface problem pressure will need in M1.4.

### M1.4 — Pressure and incompressible flow
The milestone the whole tranche exists to prove: a pressure solve without a
checkerboard null mode, hydrostatic pressure with depth, the U-pipe
scenario (tranche target 1), sealed-volume pressure resisting compression.
Judged **risky by default** at the round-planning level — this is exactly
where the previous attempt died, it's a shared mechanism every later
milestone in this tranche and gas/thermal work depends on, and it bears
directly on a stated tranche target.

### M1.5 — Gases
Gas transport/mixing, multiple species, density-driven stratification,
pressure equalisation across a connected region — builds on M1.4's pressure
solve rather than reinventing one for gas.

### M1.6 — Temperature
Conduction in flux form (the JS attempt's named recurring bug — leaked until
rewritten in flux form; don't repeat it), heat carried by moving matter,
buoyancy, heat capacity/conductivity per material, placeable hot/cold
sources. Depends on M1.2–M1.5's matter/gas transport being in place to have
something for heat to move through and move.

### M1.7 — Consolidation and scale
Fills out the material table to what tranches 2-3 will need (named, not
invented from scratch — PLAN.md and tranche 2/3's own material mentions are
the source list), measures performance against M1.1's budget, does the
GPU-readiness *groundwork check* (not a GPU backend), and runs the full
primitive-scenario completeness sweep. Closing milestone; where tranche
targets 5 and 6 get their final check.

## 5. Sequencing note

M1.4 is the one milestone flagged risky ahead of time, in the sense
`cycle-plan` round-level guidance means: it touches a shared primitive
(pressure) every later milestone in this tranche depends on, and it bears
directly on tranche target 1. That doesn't fix every *round* inside M1.4 as
risky — that's `cycle-plan` at milestone/round level's call once M1.4 starts
— but the milestone-level plan for M1.4 should expect to spend the
Red/Green/Refactor budget there rather than assume single-pass throughout.

No round is listed here — per `cycle-plan`, rounds are planned one at a time
because round N+1's shape isn't knowable before round N reports. That's
`cycle-milestone`'s job once each milestone starts.

## 6. Report

**What I folded forward.** Tranche 0's closeout (Vec2/GridIndex gap, the
fork-vs-cold-agent and sequential-round-dispatch process fixes already live
in the skills, the existing Pages pipeline and renderer to build on rather
than replace).

**Push on vs. patch back.** Push on — tranche 0 closed clean with no exit
ramps and no unresolved defect thread; the only carried-forward item
(Vec2/GridIndex unused) is closed by M1.1's own natural first step, not a
detour.

**Deferred, and where recorded.** Dissolved/mixed composition and
permeation → tranche 2 (§2 above, and PLAN.md M2.5 already states it). GPU
backend work beyond groundwork → left to whichever future tranche's evidence
demands it (CLAUDE.md's own standing rule, restated in §2 above).
