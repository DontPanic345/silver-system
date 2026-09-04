# M1.1 closeout — Substrate and harness

**Closed:** 2026-09-05 (orchestrator)

## 1. Targets: each one, and whether it was met

All 5 milestone targets (from `cycle-log/tranche-1/m1.1/plan.md` §3) are
**met**.

1. **A scenario runs headless and emits JSON measurements.** Met —
   Round 3. `src/measure.rs`'s `run_headless(&Scenario, num_steps, dt) ->
   Measurement` steps a `Scenario`'s grid and emits total mass and cell
   count per material plus the tick count reached, via a hand-rolled
   `Measurement::to_json()`. A test asserts exact hand-derived values for
   the `stone_and_water_pool()` fixture (18 air/0.0 mass, 2 water/2.0
   mass, 4 stone/10.0 mass, totals 24 cells/12.0 mass) plus a 100-step
   exact-conservation smoke check — not merely "it parsed." Independently
   re-verified by me at Round 3 close-out (`cargo test --lib`) and again
   at the milestone Refactor pass (66/66 lib tests passing, 1 ignored).
2. **The same scenario renders.** Met — Round 4. `src/render.rs`'s pure
   `render_grid_to_rgb8` mirrors `render_frame`'s shape and implements the
   grid-row-to-image-row y-axis flip (flagged in advance by Round 1's
   Refactor as a future trap, landed exactly where predicted). A
   `#[wasm_bindgen] paint_scenario` export reuses the existing
   `paint_rect`/`putImageData` canvas pattern. Verified headlessly via
   *both* paths rather than just one: native PNG decode
   (`tests/render_native.rs`) and wasm/Playwright pixel read
   (`tests/e2e/scenario_canvas.test.mjs` + `www/scenario.html`) — no
   human looked at a screenshot to confirm this. The pre-existing M0.1
   pipeline (`canvas_rectangle.test.mjs`, `native_fallback.rs`) stayed
   green and unmodified throughout, confirmed independently by me at
   Round 4 close-out and again at the Refactor pass.
3. **The reference grid steps within a stated per-step budget.** Met —
   Round 5. Reference grid: **1024×1024** (1,048,576 cells), chosen as a
   meaningful stand-in for "the large universe" while timing sensibly on
   this dev machine. Measured via 5 warm-up + 50 measured `Grid::step`
   calls, `std::time::Instant`, on this dev container (16 logical CPUs,
   Intel Core Ultra 9 285H, 15Gi RAM — a dev container, not dedicated
   hardware). **Recorded budget: ~2.0ms/step (release build)** — 2.19ms
   and 1.91ms across two runs — with debug-build figures (14.86ms,
   13.24ms) kept alongside for context. This is the identity-step's
   mechanism floor, not a promise about future physics cost; M1.7 should
   cite it as the starting number to hold later milestones to. Made
   reproducible via a checked-in `#[ignore]`d test
   (`reference_grid_step_timing` in `src/grid.rs`) rather than a one-off
   scratch measurement — I independently re-ran it (`cargo test --lib --
   --ignored reference_grid`) and confirmed it passes in 0.86s (debug
   build, 55 total steps), consistent with the reported figures. Full
   detail in `cycle-log/tranche-1/m1.1/round-05.md`.
4. **`Vec2`/`GridIndex` are exercised by real running code.** Met —
   Round 1. `Grid` is `GridIndex`-addressed throughout its whole API
   (`linear_index`, `get`/`set`/`get_next`/`set_next`, `cell_center`
   delegating to `GridIndex::center`); the `#[allow(dead_code)]` was
   removed from both types once this became true. This closes the gap
   tranche 0 named explicitly (`cycle-log/tranche-0/closeout.md`,
   `.../tranche-0/m0.4/closeout.md`) as something to close honestly via a
   real grid, not forced wiring — and it was closed that way: `Grid` is
   the milestone's actual substrate, not a wrapper built to exercise two
   types.
5. **Test tagging and a fast path exist from round 1.** Met — Round 1.
   `README.md` documents `cargo test --lib` (fast — in-crate unit/
   scenario tests only, 0.01s) versus `cargo test` (full suite including
   native-subprocess PNG-decode integration tests, ~1.7s) as the two
   commands, plus the `#[ignore]` convention for genuinely slow tests
   (used for Round 5's timing test). Every round from 2 onward used this
   convention without needing to invent anything new. Final numbers,
   independently confirmed at the Refactor pass: `cargo test --lib` 66
   passed/0 failed/1 ignored in 0.01s; full `cargo test` ~1.7s.

## 2. Rounds run, and the timing roll-up

| Round | Shape | Goal | Verdict |
|---|---|---|---|
| 1 | Risky (Red/Green/Refactor) | Grid + Material representation, closes Vec2/GridIndex gap, establishes fast-path convention | Advance, 5/5 goals |
| 2 | Single-pass | `Grid::step` wired to `FixedTimestep`, `Scenario` definition | Advance, 5/5 goals |
| 3 | Single-pass | Headless measurement + JSON (target 1) | Advance, 5/5 goals |
| 4 | Single-pass | Minimal renderer (target 2) | Advance, 5/5 goals |
| 5 | Single-pass | Reference grid size + performance budget (target 3) | Advance, 5/5 goals |
| — | Milestone-scope Refactor | Whole-milestone fold-in, flag dispositions, adversarial sweep | Advance |

**Timing.** Round 1 (the only risky round): Red ~7 minutes, Green and
Refactor each ran as background agents well under their 30-minute budgets
(reported ~100s and ~180s of tool time respectively — no exit ramp taken
by either phase). Rounds 2-4: each reported completing well within the
30-minute single-pass budget, no exit ramps taken (exact self-reported
wall-clock timings were not uniformly captured in this log for these
three, a process gap noted below). Round 5: explicitly timed, ~4 minutes
(2026-09-05T11:00:47 → 11:04:30). Milestone-scope Refactor: ~6 minutes
(2026-09-05T11:06 → 11:12), well under its 30-minute budget. No round or
phase in this milestone took an exit ramp or cycled; every phase advanced
on its first pass.

**Commits:** 19 commits across `cycle-log/tranche-1/m1.1/*` and the code
they describe, from `620a73c` (milestone plan) through `e3a7972`
(Refactor report). Full list in `git log --oneline cycle-log/tranche-1/m1.1/`.

## 3. What was learned that changes the plan going forward

- **The "risky round" classification worked as designed.** Round 1 (the
  shared Grid/Material primitive) was the only round judged risky ahead
  of time, and it was the right call — every later round in this
  milestone, and presumably every later milestone in the tranche, builds
  on it directly. Rounds 2-5 being single-pass and all landing cleanly on
  the first try validates `cycle-plan`'s risk criteria as applied here:
  none of them touched a shared primitive with blast radius past
  themselves, none bore on a conservation/determinism target, none were a
  hard-to-reverse interface change.
- **Flagging forward works.** Round 1's Refactor flagged the y-axis flip
  as a future trap; Round 4 hit exactly that seam and handled it
  correctly, with a dedicated pinning test. Rounds 2-5 consistently
  carried forward the accumulated flag list (density/heat_capacity
  units, out-of-bounds contract, owned `MaterialTable`, JSON escaping,
  `dt` assumption) rather than re-discovering or silently dropping them,
  and the milestone-scope Refactor pass was able to give all six an
  evidence-backed disposition in one pass specifically because they'd
  been kept visible and accurately documented in doc comments the whole
  way through.
- **"One definition, two consumers" held up as a real constraint, not
  just a slogan.** `Scenario`'s plain-data shape (no closures, no
  builder) meant both `measure.rs` and `render.rs` could consume the same
  `stone_and_water_pool()` fixture without either needing to know about
  the other — demonstrated concretely rather than asserted.
- **Physics itself is still entirely deferred**, correctly — `Grid::step`
  remains an identity transform throughout this milestone. M1.2 is where
  real per-material behaviour starts, and it inherits a settled
  substrate: SoA double-buffered grid, data-driven material table,
  scenario harness, headless JSON measurement, a renderer, and a recorded
  performance floor to hold itself against.
- **`density`/`heat_capacity`'s unit mismatch is the one target-shaped gap
  worth naming for the next planner**, even though it correctly stayed
  deferred through every round: it's currently harmless because no step
  logic reasons numerically about it yet, but M1.2+ (gravity, viscosity)
  or later heat/energy tranches will need a real answer. Whoever plans
  the milestone that first does energy math should read
  `src/material.rs`'s `reference()` doc comment before assuming the
  current placeholder values mean anything physically.

## 4. Open gaps and flags carried forward

All of these are documented in-code (doc comments) as well as here, per
the Refactor pass's explicit dispositions:

- **`density`/`heat_capacity` unit mismatch** in
  `MaterialTable::reference()` — deferred to whichever future
  milestone/tranche first does real heat/energy calculations that would
  pin the units against something. (`src/material.rs`)
- **`paint_scenario`'s hardcoding to `stone_and_water_pool()`** rather
  than accepting an arbitrary `Scenario` — deferred; real feature work
  (public API + caller-selection design in both wasm and native paths),
  out of scope for what M1.1's target 2 actually required. Revisit before
  any milestone that needs to render more than one fixture interactively.
  (`src/lib.rs`)
- **`total_mass`'s unpinned unit, and the hand-rolled JSON writer's lack
  of string-escaping** — both deferred and both currently inert (no
  string field exists in the JSON shape yet, so escaping is a latent, not
  live, gap). Revisit if a string field is ever added to `Measurement`.
  (`src/measure.rs`)
- **`Scenario.materials` owned, not shared, `MaterialTable`** — deferred;
  each `Scenario` self-contained and small at this milestone's scale, no
  measured duplication/drift cost found. Revisit if scenario count or
  material-table size grows enough to matter. (`src/scenario.rs`)
- **`run_headless`'s `dt`-matches-internal-`FixedTimestep` assumption** —
  deferred; currently always true by construction (one call site builds
  the `FixedTimestep`). Revisit if a second construction path is ever
  added. (`src/measure.rs`)
- **The stray untracked `test/` directory** (PNG output from an early
  native-binary run, unrelated to any round's tracked work) — harmless,
  `.gitignore`-or-delete decision explicitly outside any single round's
  scope, and `rm` was blocked by the sandboxing layer on first attempt
  (Round 1) and not re-attempted since, per that denial's own guidance
  not to work around it. Whoever has shell access outside this sandbox
  should clean it up or add a `.gitignore` entry; not urgent.
- **`Grid`'s out-of-bounds panic contract** — settled and now fully
  test-covered on all four violation angles (negative i, negative j,
  i==width, j==height) as of the Refactor pass. No longer "provisional";
  this is a closed item, listed here only so the next milestone doesn't
  re-flag it as open.

## 5. What the cycle itself got wrong

Read honestly, not manufactured: **no clear `cycle-*` skill defect
surfaced across this milestone's 5 rounds and Refactor pass** — a
different outcome from tranche 0, which found two real process defects
(fork-vs-cold-agent for self-dispatch, and concurrent round dispatch)
that are now fixed in the skills and were followed correctly here
(every round in this milestone was dispatched sequentially via fresh
`Agent` calls, never `subagent_type: "fork"`, and I waited for each
round's close-out before dispatching the next).

One genuine, if minor, **process gap on my own side rather than the
skills'**: I did not consistently record each single-pass round's exact
wall-clock elapsed time in a uniform way across Rounds 2-4 the way Round
1 (via its Red/Green/Refactor phase timestamps) and Round 5 (which
explicitly logged start/end `date -Is` timestamps) did. This made the
timing roll-up in §2 above less precise than it should be for those three
rounds. Worth a small process note for future milestone orchestrators:
ask every single-pass round's dispatch prompt to log explicit start/end
timestamps the way Round 5's did, so the milestone timing roll-up doesn't
have this gap. This is not severe enough to call a skill defect — the
skills don't forbid it, I simply didn't enforce it uniformly — but it's
worth naming so the next orchestrator does better.

## 6. PLAN.md

Checked `PLAN.md`'s Tranche 1 / M1.1 section against what was actually
built: no changes needed. M1.1 executed within the shape PLAN.md and the
tranche-1 plan already described — grid/material representation, scenario
harness, headless measurement, minimal renderer, Vec2/GridIndex closure,
and (per the tranche-1 plan's explicit deferral) the reference-grid-size
performance number, which is now fixed at ~2.0ms/step (1024×1024, release
build) and available for `PLAN.md`'s tranche target 6 to cite going
forward if/when that document is next revised at the tranche level — that
edit belongs to whoever next touches the tranche-level plan, not this
milestone closeout.

## 7. Verification basis

Every quantitative claim above was independently checked by me, not taken
solely on a dispatched agent's word:

- `cargo test --lib`: 66 passed, 0 failed, 1 ignored, 0.01s (re-run at
  Refactor-pass close, matching the agent's report exactly).
- `cargo test` (full native suite, incl. `tests/render_native.rs`,
  `tests/native_fallback.rs`): all green, ~1.7s.
- `cargo build --lib --target wasm32-unknown-unknown`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo fmt --check`: clean.
- `git status --short`: clean apart from the pre-existing untracked
  `test/` directory.
- `cargo test --lib -- --ignored reference_grid`: passes, 0.86s (debug
  build), consistent with Round 5's reported debug-mode per-step figures.
- `git log --oneline cycle-log/tranche-1/m1.1/`: confirms the full round
  and phase commit history cited above.

## 8. Status

**M1.1 is closed.** All 5 milestone targets met and independently
verified. The milestone-scope Refactor pass recommends Advance; I concur
— no open correctness finding requires cycling, and every carried-forward
flag has an explicit, evidence-backed disposition rather than riding
silently forward.

Per this milestone's own instructions: **M1.2 is not started here.**
Whether and how to proceed into M1.2 is the tranche orchestrator's call.
