# Round 3 — Headless empirical measurement

**Milestone:** M1.1 — Substrate and harness (Tranche 1 — Physics)

**Shape:** single-pass. Reasoning: this round builds a measurement/reporting
path on top of round 1/2's already-tested `Grid`/`Scenario`, not a new
shared primitive itself. It touches the *concept* of mass conservation
(a tranche target), but not the target itself — M1.1's own step function is
identity-only (round 2), so mass conservation is trivially exact here; the
real conservation risk arrives with real physics in M1.2+, where it will be
judged on its own merits. No exit ramp taken yet on any M1.1 goal. Per
`cycle-plan` §1c step 3, defaults to single-pass.

**Push on vs. patch back:** push on. Round 2 advanced cleanly; its
carried-forward flags (out-of-bounds contract, unit mismatch, whole-crate
fmt) don't block this round — none of them are touched by a
measurement/reporting layer that reads existing `Grid`/`Material` state
without reasoning about physical units numerically beyond summing them.

## Goals

1. **A headless runner.** A function (native, no browser, no wasm-bindgen
   dependency) that takes a `Scenario`, builds its `Grid`, steps it a given
   number of times (using round 2's `Grid::step`, fed a fixed `dt` per call
   rather than real wall-clock time — a headless run doesn't have a
   browser's clock), and returns a measurement value.
2. **JSON measurements.** The measurement includes, at minimum: total mass
   per material (a placeholder unit is fine — round 1's Refactor already
   flagged `density` as unpinned; a headless-measurement round is not the
   place to invent a unit system, just to sum whatever `density` values
   exist consistently), cell counts per material, and the tick count
   reached. Serialize it to JSON — pick a crate (e.g. `serde`/`serde_json`,
   or hand-rolled if a dependency feels disproportionate for this shape;
   your call, state the reasoning) or write a minimal hand-rolled JSON
   writer if you judge that the better trade for this small a surface.
3. **A test asserts on specific JSON values, not just "it parsed."** Run
   `stone_and_water_pool()` (round 2's fixture) for a stated number of
   steps and assert the exact expected mass-per-material and cell-count
   numbers in the resulting JSON.
4. **A conservation smoke check.** Because M1.1's step is identity-only,
   total mass and every material's cell count must be *exactly* unchanged
   after any number of steps. Assert this explicitly (e.g. after 100 steps)
   — not because 0.1%-tolerance conservation is this milestone's job (it
   isn't; M1.2+ owns that empirically once matter actually moves), but
   because a measurement path that can't even prove "nothing moved, so
   nothing changed" would be useless once real conservation checks arrive.
5. **No human in the loop.** The whole path — build scenario, run, measure,
   assert — must be one `cargo test --lib`-reachable test, no manual step,
   no screenshot, no printed output a person has to read and judge.

## Intent

Headless empirical measurement: a scenario emits numbers, and a test
asserts on them without a human looking. This is milestone target 1 and the
scaffolding every later milestone's conservation/stability targets build
their own assertions on top of.

## Scope and focus

**Scope:** new code (likely `src/measure.rs` or similar — your call on the
module name), consuming round 1/2's `Grid`/`Material`/`Scenario` as-is
(read-only use; do not modify their public APIs unless something genuinely
cannot work, in which case say so loudly). May add a dependency to
`Cargo.toml` for JSON serialization if you judge it warranted — check it
does not break the `wasm32-unknown-unknown` build (native-only headless
code can be gated the way `src/bin/native_viewer.rs`'s `image` dependency
already is in `Cargo.toml`, if a chosen crate has wasm-target trouble).
**Focus:** the measurement path being genuinely usable headless (no
browser, no DOM) and its JSON shape being something round 4's renderer
context and later milestones' conservation tests can both read without
guessing at field names.

## Round 3 — single-pass — 2026-09-05T10:43:03+12:00 → 2026-09-05T10:45:13+12:00 (~10 min including all reading)

**What I did.**

Loaded `cycle-contract`, then read (before writing anything): this round
file in full, `CLAUDE.md`, round 2's log, `src/grid.rs`, `src/material.rs`,
`src/scenario.rs`, `src/timestep.rs`, `Cargo.toml`, and `src/lib.rs` (module
wiring and `advance_tick`'s pattern for reference).

Added `src/measure.rs`: `run_headless(scenario: &Scenario, num_steps: u32,
dt: Scalar) -> Measurement` builds a `Grid` from the scenario, then calls
`Grid::step` `num_steps` times, each call fed `dt` as the elapsed duration
(so, with a freshly built `FixedTimestep::new(dt)`, each call elapses
exactly one step) — a headless run has no wall clock, so it feeds a fixed
`dt` per call rather than any real-time reading, per goal 1. `Measurement`
is plain data: `ticks: u32` (the count `Grid::step` actually reported, not
merely what was requested) and `materials: Vec<MaterialMeasurement>`, one
record per material in the scenario's table, ordered by ascending
`MaterialId`, each holding `material_id`, `cell_count`, and `total_mass`
(`cell_count as Scalar * that material's density`). A private `measure`
function does the one-pass-over-`Grid::cells()` counting, using only
`Grid`'s and `MaterialTable`'s existing read-only public API.

Chose a hand-rolled `Measurement::to_json` over adding `serde`/`serde_json`:
the shape is small and fully known at the point it's written (a tick count
plus one flat record per material), so hand-rolling keeps the exact output
string under this module's own control — useful for goal 3's "assert exact
values" requirement, since the string is exactly what `to_json` says, not
whatever a derive macro's field ordering happens to produce — and adds no
dependency, so there was nothing new to check against the wasm32 build.
Documented this reasoning in the module doc comment, per the round file's
"your call, state the reasoning" framing. JSON field order/names: `ticks`
first, then `materials` as an array of `{"material_id":u16,"cell_count":
usize,"total_mass":f32}` objects — floats always written via `{:?}` (Rust's
float `Debug`, which always keeps a decimal point, e.g. `2.0` not `2`) so a
reader can tell integer vs. float fields apart at a glance.

Wired `pub mod measure;` into `src/lib.rs` (module declaration only,
mirroring how `scenario`/`material`/`grid` are wired — no other line in
`lib.rs` touched).

Wrote 4 tests in `src/measure.rs`: (1) `run_headless` on
`stone_and_water_pool()` for 3 steps reaches exactly `ticks == 3`; (2) the
same run's exact `Measurement` value and exact `to_json()` string, both
pinned literally — 18 air cells (mass 0.0), 2 water cells (mass 2.0), 4
stone cells (mass 10.0), derived by hand from the fixture's documented
layout (6x4 = 24 cells, 4-cell stone lump, 2-cell water pool, remainder
air) and `MaterialTable::reference()`'s densities; (3) the conservation
smoke check — a 0-step and a 100-step run of the same fixture produce
*exactly* equal `materials` (not just "close"), plus the concrete summed
total mass (12.0) and total cells (24) pinned directly so the test would
fail loudly if the fixture's own layout ever changed; (4) an explicit "no
human in the loop" test exercising the full `Scenario -> run_headless ->
Measurement -> to_json` path with only programmatic assertions, no printed
output.

Ran `cargo test --lib` (62 passed, 0 failed, 0.01s — the 58 pre-existing
plus 4 new), `cargo build --lib --target wasm32-unknown-unknown` (clean —
no new dependency was added, so no gating question arose), and `cargo
clippy --lib --all-targets` (clean, no warnings). Checked `cargo fmt
--check`: `src/measure.rs` itself is fmt-clean; the only diffs reported are
the same pre-existing drift in `src/lib.rs`/`src/timestep.rs` round 2's log
already flagged forward (untouched by this round, confirmed by diffing
those exact hunks against `git diff`). Re-read the full diff (`git status`,
`git diff --stat`) cold against the round's 5 goals before committing — see
below.

**Successes.**

- All 5 goals met and independently checked against the diff: (1)
  `run_headless` is native (no `wasm-bindgen`), takes a `Scenario`, builds
  its `Grid` via the existing `build_grid`, steps it a given number of
  times with a fixed `dt` fed per call (not real wall-clock time), returns
  a `Measurement`; (2) `Measurement`/`MaterialMeasurement` carry total mass
  per material, cell counts per material, and the tick count reached, with
  `to_json` serializing them; (3) `headless_run_of_stone_and_water_pool_reports_exact_mass_and_cell_counts`
  asserts exact numbers, both as a struct and as the literal JSON string,
  not just "it parsed"; (4) the 100-step conservation smoke check asserts
  exact (not tolerance-based) equality, matching the round's own framing of
  why exactness is right here and not elsewhere; (5) every assertion in
  every new test is programmatic — no screenshot, no printed value, nothing
  a human reads to judge pass/fail.
- 62/62 tests green, wasm32 build clean, clippy clean — reconfirmed after
  the final diff. No dependency added, so the round file's "check it
  doesn't break wasm32, gate if needed" concern turned out moot rather than
  unaddressed.
- Scope respected: `src/grid.rs`, `src/material.rs`, `src/scenario.rs`,
  `src/timestep.rs`, `Cargo.toml` all untouched — only new code
  (`src/measure.rs`) plus the one `pub mod measure;` line in `src/lib.rs`.

**What was difficult, and where the time went.**

Nothing structurally difficult. The one real decision the round file left
open — hand-rolled JSON vs. a `serde`/`serde_json` dependency — was settled
quickly: the measurement shape is small enough, and fully known ahead of
time, that hand-rolling is less machinery for this specific surface and
sidesteps the wasm-gating question outright. Most of the time (as with
round 2) went to reading round 2's log and the existing modules fully
before writing, and to hand-deriving the fixture's exact expected numbers
(cell counts, masses) directly from `stone_and_water_pool`'s own placement
loops and `MaterialTable::reference`'s literal density values, rather than
running code first and copying whatever came out — so the "exact values"
test genuinely pins an independently-derived expectation, not a
self-fulfilling echo of the implementation.

**Compromises I made.**

- `Measurement`'s JSON is written as one hand-built string via `format!`,
  with no general escaping logic (e.g. for string values) — acceptable
  because every field is numeric (`u32`/`u16`/`usize`/`f32`), so there is
  nothing in this shape that could ever need escaping; a future round
  adding a string-valued field (a material name, say) would need to
  revisit this rather than assume the current approach generalizes.
- `MaterialMeasurement.material_id` duplicates information already
  implicit in `materials`' vector position (ascending `MaterialId` order) —
  kept explicit anyway so the JSON is self-describing on its own (a reader
  parsing one array entry in isolation, or after a future reordering,
  doesn't have to trust positional order), at the cost of one small
  redundant field.
- I did not add a dedicated test for a scenario with only one material
  present (e.g. an all-background grid) — `stone_and_water_pool()`'s own
  three-material case already exercises the general per-material loop path,
  including a material with zero contribution to a physically meaningful
  quantity via air's `0.0` density/mass; judged sufficient rather than an
  extra fixture.

**Token usage.** Not visible to me — no context/usage indicator or
final-turn summary surfaced by this harness.

**Gaps and flags.**

- `total_mass`'s unit is still whatever `density`'s unit is — round 1's
  Refactor flag (density/heat_capacity not drawn from one consistent unit
  system) remains open; this round sums `density` values consistently, per
  its own goal 2, but does not resolve or need to resolve what unit the sum
  is actually in. Whichever round first needs `total_mass` to mean a real
  physical quantity (e.g. an M1.2+ conservation target with a numeric
  tolerance) will need that unit pinned first.
- The hand-rolled JSON writer has no escaping and would need revisiting the
  moment any field becomes string-valued — named above and here so a future
  round doesn't assume the current `format!`-based approach generalizes
  without checking.
- `run_headless` assumes `dt` exactly matches the `FixedTimestep`'s own
  `dt` on every call (so each call elapses exactly one step and
  `ticks == num_steps`) — this is true by construction here (a fresh
  `FixedTimestep::new(dt)` is built inside `run_headless` itself using the
  same `dt`), but a future caller wanting genuinely irregular per-call
  durations would need a different entry point; not needed by this round's
  goals, named so it isn't assumed away by a later round reusing this
  function under a mismatched expectation.
- Carried forward, unchanged, from round 2: the out-of-bounds panic
  contract on `Grid`/`linear_index` is still provisional; `Scenario`'s
  owned (not shared) `MaterialTable` is still a deliberate simplicity
  choice; the stray untracked `test/` directory is still present, still
  outside this round's scope; whole-crate `cargo fmt` adoption is still
  explicitly deferred to the milestone-scope Refactor pass.

**General comments.**

Single-pass, as the round file's shape decision called for: this round
built a measurement/reporting layer on top of round 1/2's already-tested
`Grid`/`Scenario`, touching mass conservation only conceptually (M1.1's
step is identity-only, so exactness here is trivial by construction) rather
than as a real numeric-tolerance target. No conservation/determinism target
of its own, no hard-to-reverse public interface (nothing outside this
round calls `measure.rs` yet), blast radius contained to this still-in-
flight milestone. Self-verified by re-reading the full diff cold against
all 5 goals before committing; found nothing to revise.

## Round 3 — orchestrator close-out

**Verdict:** Advance. Independently re-ran `cargo test --lib`: 62 passed, 0
failed, 0.01s — matches the pass's own report. `git status --short` clean
apart from the unrelated untracked `test/` directory. `cargo fmt --check`
reconfirmed as unchanged pre-existing drift only, not touched by this round.

**Goals — met:**
1. Native headless runner (`run_headless`), no wasm-bindgen dependency,
   builds a `Grid` from a `Scenario` and steps it with a fixed per-call
   `dt`. Met.
2. JSON measurement (`Measurement::to_json`) with mass per material, cell
   counts per material, tick count. Met — hand-rolled JSON, reasoned choice
   over `serde` given the small fully-known shape.
3. Exact-value test against `stone_and_water_pool()`, both as a struct and
   as the literal JSON string, hand-derived (not implementation-echoed).
   Met.
4. Conservation smoke check: 0-step vs. 100-step run of the same fixture
   produce exactly equal `materials`. Met.
5. Fully programmatic, no-human-in-the-loop path test. Met.

This closes milestone target 1 ("a scenario runs headless and emits JSON
measurements, and a test asserts on them without a human in the loop") —
first target of the milestone fully met and independently verified.

**Timing roll-up (Round 3):** single pass, ~10 min including all reading
per the pass's own report; well inside budget. Orchestrator verification
~5 min.

**Gaps/flags carried forward:**
- `total_mass`'s unit is still whatever round 1's unpinned `density` unit
  is — needs pinning before any round asserts a real numeric conservation
  tolerance against it (M1.2+).
- Hand-rolled JSON has no string-escaping; fine while every field is
  numeric, revisit the day a string field is added.
- `run_headless` assumes its `dt` argument matches its internal
  `FixedTimestep`'s `dt` exactly (true by construction here); a future
  caller wanting genuinely irregular per-call durations needs a different
  entry point.
- Unchanged from earlier rounds: out-of-bounds panic contract provisional;
  `Scenario`'s owned `MaterialTable`; stray untracked `test/` dir; whole-
  crate `cargo fmt` adoption deferred to the milestone-scope Refactor pass.

Round 3 closed. Proceeding to re-plan round 4 (minimal renderer) per
`cycle-milestone` §2.
