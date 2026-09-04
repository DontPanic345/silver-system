# M1.1 milestone-scope Refactor pass

**Dispatched:** 2026-09-05 (orchestrator)

## Scope

The whole milestone: everything built across Rounds 1-5 —
`src/math.rs` (only as touched: `#[allow(dead_code)]` removals),
`src/grid.rs`, `src/material.rs`, `src/scenario.rs`, `src/measure.rs`,
`src/render.rs`, the `src/lib.rs` additions (`paint_scenario`,
`paint_rgb8_to_canvas`, getters), `src/bin/native_viewer.rs`'s
extension, `Cargo.toml`'s `ImageData` feature addition, and the new test
files (`tests/render_native.rs`, `tests/e2e/scenario_canvas.test.mjs`,
`www/scenario.html`). Not in scope: the pre-existing M0.1 pipeline itself
(`draw`, `paint_rect`, `render_frame`, `tick_and_draw`, etc.) except where
this milestone's new code interacts with it — don't rewrite working M0.1
code for its own sake, but do check the seams.

## Focus

Chosen from what the round logs have been complaining about, repeatedly
and consistently, across Rounds 1-5:

1. **Whole-crate `cargo fmt` adoption.** Every round from 2 onward has
   flagged whole-crate formatting drift and deferred it here explicitly.
   Run it, scoped correctly this time (the milestone orchestrator's Round
   2 fix already showed `cargo fmt` touching files outside a round's
   scope is a real risk — verify nothing substantive is intermixed with
   the formatting diff before committing).
2. **Explicit calls on the accumulated small flags**, rather than letting
   them silently ride forever. For each of the following, either fix it
   (if small/mechanical) or make an explicit, reasoned decision to defer
   it past M1.1, and say which:
   - `density`/`heat_capacity` unit mismatch in `MaterialTable::reference()`
     (unpinned/water-normalized density vs. real J/(g·K) heat_capacity) —
     flagged since Round 1.
   - `Grid`'s out-of-bounds panic contract on `linear_index` — flagged as
     provisional since Round 1.
   - `paint_scenario`'s hardcoding to the `stone_and_water_pool()` fixture
     rather than an arbitrary `Scenario` parameter — flagged Round 4,
     judged acceptable for M1.1's stated target but worth an explicit
     look here.
   - `total_mass`'s unpinned unit, and the hand-rolled JSON writer's lack
     of string-escaping, in `measure.rs` — flagged since Round 3.
   - `Scenario.materials` owned (not shared) `MaterialTable` — flagged
     since Round 2.
   - `run_headless`'s assumption that its `dt` parameter matches the
     internal `FixedTimestep`'s `dt` — flagged since Round 3.
3. **An adversarial sweep** across the whole milestone's new code,
   independent of the round-by-round diffs that already got scrutiny in
   their own Refactor/self-review passes: resting-state behaviour (does
   an all-air or all-stone grid stay put under `step`?), symmetry
   (does a symmetric placement produce symmetric measurements?),
   boundary cells (`GridIndex` at the grid's edges), the test suite
   itself (anything tautological, anything slow enough to threaten the
   fast-path convention established in Round 1).
4. **Mechanical issues anywhere you find them** — clippy lints, stale
   doc comments, dead imports, stray `dbg!`/`println!` — fix immediately
   wherever found, in scope or not, per this phase's standing instruction.

Not in focus: adding new features, new physics, or anything not already
named as a target/flag across Rounds 1-5. This is consolidation, not
forward motion.

## Not this pass's job

- The stray untracked `test/` directory — `rm` has already been tried and
  explicitly blocked by sandboxing (Round 1's close-out); don't re-attempt
  it, note it as still-open if you like, but no new attempt.
- Milestone closeout itself — that's the orchestrator's job after this
  pass reports back.

## Timing

30-minute budget per `cycle-refactor`'s own protocol, though this is
milestone-scope rather than round-scope — use the time well; going over is
a signal to report back with what's left rather than silently truncating
the sweep.

## Report

Append your report to this file (`cycle-log/tranche-1/m1.1/refactor.md`),
following `cycle-refactor`'s report format: change list with rationale,
what the adversarial pass tried (including negative results), any
correctness findings with evidence, current suite runtime, the verdict,
and what you'd have done with another 30 minutes.

Commit with the standard trailer. Explicit `git add <paths>`, never `-A`.
