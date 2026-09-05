# M1.1 Round 5 — Reference grid size and performance budget

**Planned:** 2026-09-05 (orchestrator)

## Status of milestone targets going in

Targets 1, 2, 4, 5 (headless JSON measurement, renderer, Vec2/GridIndex
exercised by real code, test tagging/fast path) are already closed — see
Rounds 1-4's close-outs. Only target 3 remains:

> **The reference grid steps within a stated per-step budget.** A grid
> size is chosen and recorded (with reasoning), a step of it is timed on
> this dev machine, and the number is written down in this milestone's
> closeout so M1.7 can hold later milestones to it.

This is also the first fixing of tranche-1 target 6's number (the tranche
plan defers the exact figure to M1.1).

## Goals

1. **Choose and record a reference grid size**, with reasoning: large
   enough to be a meaningful stand-in for "the large universe" (the north
   star's first half depends on the universe underneath being genuinely
   full — a toy-sized grid wouldn't test that), small enough to time
   sensibly and repeatably on this dev machine. Use `Grid`/`GridIndex` as
   they exist today; no new grid-construction machinery needed beyond
   what Rounds 1-2 already built.
2. **Time a step of that reference grid** using the existing
   `Grid::step`/`FixedTimestep` path (unmodified — this round measures,
   it does not change stepping behaviour). Native only; no wasm/browser
   timing (JS timers are not a reliable measurement instrument, and the
   existing native/PNG headless pattern already gives an honest,
   reproducible measurement path).
3. **Record the number and how it was measured** — machine context (dev
   container, not dedicated hardware — the number is a budget stand-in,
   not a marketing claim), methodology (warm-up iterations if used,
   number of steps averaged, wall-clock timer used), and the resulting
   per-step budget in milliseconds or microseconds. This measurement
   should end up in a form the milestone closeout can cite directly, and
   that M1.7 can later hold real per-milestone step costs against.
4. **A test or checked-in fixture makes the timing reproducible**, not a
   one-off number in a scratch script — e.g. a `#[test]` (likely
   `#[ignore]`d by default, matching the fast-path convention from Round
   1, since timing runs are not "fast" in the unit-test sense) or a small
   native binary/example that can be re-run later to re-derive the
   number, rather than the figure only existing as prose in a report.
5. **No production behaviour changes.** `Grid::step`'s identity/no-op
   semantics (correct and intentional per this milestone's "bones, not
   behaviour" intent) are not to be touched. This round is purely
   measurement/harness, additive only.

## Scope focus

Additive only: a way to build/size a reference grid, a timing harness
(native), and where the number gets recorded. No changes to `Grid`,
`Material`, `Scenario`, `measure.rs`, or `render.rs`'s existing logic
expected — if the agent finds a genuine need to touch any of them, that's
a flag for me to review at close-out, not a blocker to raise first.

## Shape decision: single-pass

This round is measurement plus a small reproducible harness around
existing, unmodified primitives (`Grid::step`, `FixedTimestep`) — no new
shared primitive, no blast radius past this round, no conservation/
determinism target at stake (this measures wall-clock cost, not physical
correctness), no interface/data-format change. Matches `cycle-plan`'s
single-pass criteria squarely. No prior exit ramp or cycle on this goal.

## Must-not-break condition

All existing tests (`cargo test --lib`, `cargo test`, wasm32 lib build)
must remain green throughout.

## Dispatch

Fresh cold `Agent` (`subagent_type: "claude"`), given: this round file,
`src/grid.rs`, `src/timestep.rs`, `src/scenario.rs`, `src/measure.rs`, the
milestone plan (`cycle-log/tranche-1/m1.1/plan.md`), and Round 1-4's
close-outs for context (grid representation, the identity step, existing
fast-path convention).

## Round 5 — single-pass — 2026-09-05T11:00:47+12:00 → 2026-09-05T11:04:30+12:00 (~4 min)

**What I did.**

Read `CLAUDE.md`, `cycle-contract`, `cycle-plan`, `cycle-round`, `cycle-green`,
this round file, the milestone plan, `src/grid.rs` (including all four prior
rounds' tests), `src/timestep.rs`, `src/scenario.rs`, `src/measure.rs`,
Round 1's full close-out (for the grid/material shape and the fast-path
convention's origin), and `README.md`'s "Test tagging / fast path" section.

Added one new, `#[ignore]`d test to `src/grid.rs`'s existing `#[cfg(test)]
mod tests` — `reference_grid_step_timing` — plus two `const`s
(`REFERENCE_GRID_WIDTH`, `REFERENCE_GRID_HEIGHT`) naming the reference grid
size. No other file touched; `Grid::step`, `Grid::step_once`, `FixedTimestep`,
`Scenario`, and `measure.rs` are all byte-for-byte unchanged — confirmed by
`git diff --stat` showing only `src/grid.rs`, and by the diff itself being
purely additive (new test + two consts appended at the end of the existing
`mod tests` block, nothing removed or reordered).

**Why an in-crate `#[ignore]`d test, not a new binary or an integration
test under `tests/`:** `FixedTimestep` (`src/lib.rs` line 58: `mod
timestep;`, not `pub mod`) is crate-private — an integration test in
`tests/` or a `src/bin/` binary is a separate crate from `viewer`'s
perspective and cannot name `FixedTimestep` to build one itself. Rather than
loosen that module's visibility (a real, if small, production-code change,
and out of this round's additive-only scope) just to relocate a test, I put
the timing test inside `src/grid.rs` itself, where `Grid::step`'s existing
tests already construct `FixedTimestep` values directly. This keeps the
round's diff to "add a test", not "add a test and change a visibility
boundary".

## Reference grid / performance budget

**Reference grid size: 1024 x 1024 cells (1,048,576 cells total).**

**Reasoning.** Large enough to be a meaningful stand-in for "the large
universe" the north star names — several orders of magnitude past a toy
grid (the milestone's own fixture, `stone_and_water_pool`, is 6x4 = 24
cells; a 1024x1024 grid is ~43,700x that), and past what any single on-screen
"terrarium" view plausibly renders 1:1 at once, so the number this round
records is honestly a budget for the *underlying* substrate, not for
whatever window onto it a later renderer actually draws. Small enough that:
(a) it is a round power-of-two, easy to reason about and to scale up/down
from later without arithmetic surprises; (b) even at this milestone's
identity-only step cost, timing it stays comfortably inside a fast dev-loop
budget (worst case observed below, ~15ms/step debug, ~2ms/step release —
both far under a single test's or a single interactive frame's budget); (c)
it is small enough that a future, much heavier per-cell physics step (M1.2+)
will not make this same reference size unusable as a benchmark — there is
headroom before 1024x1024 itself becomes the bottleneck rather than the
per-cell work.

**Methodology.** A native (no wasm/browser), in-crate `#[test]`,
`reference_grid_step_timing` in `src/grid.rs`, `#[ignore]`d by default
(matches Round 1's fast-path convention — a timing run is not "fast" in the
unit-test sense). Run explicitly with:

```sh
cargo test --lib -- --ignored reference_grid_step_timing --nocapture
# or, for the optimized-build number:
cargo test --release --lib -- --ignored reference_grid_step_timing --nocapture
```

Builds a `Grid::new(1024, 1024, AIR)`, a `FixedTimestep::new(1.0 / 60.0)`,
takes **5 warm-up steps** (untimed, via the existing, unmodified
`Grid::step`) to let allocator/cache effects settle, then times **50
measured steps** via `Grid::step` calls (each fed `dt = 1.0 / 60.0`, so each
call elapses exactly one step) wrapped in `std::time::Instant::now()` /
`.elapsed()`, and reports `elapsed / 50` as the average per-step time. The
stepping mechanism exercised is exactly `Grid::step` → `Grid::step_once`
(identity transform, unmodified this round) → `FixedTimestep::advance` — the
same path every other round in this milestone already tests, not a
special-cased fast loop.

**Machine context.** This dev container: 16 logical CPUs, Intel(R) Core(TM)
Ultra 9 285H, 15Gi RAM. A dev container, not dedicated/isolated benchmarking
hardware — this number is a budget stand-in for later milestones to hold
themselves to, not a marketing claim, and is expected to vary run-to-run and
machine-to-machine (see the numbers below, which already show ~10-15%
variance run-to-run on identical code).

**Measured numbers** (`viewer 0.1.0`, `1024x1024` = 1,048,576 cells, 50
measured steps after 5 warm-up steps):

| Build              | Total (50 steps) | Average per step |
|---------------------|-------------------|-------------------|
| debug (`cargo test --lib`)          | 742.87ms | 14.86ms |
| debug (`cargo test --lib`, rerun)   | 662.02ms | 13.24ms |
| release (`cargo test --release --lib`) | 109.46ms | 2.19ms |
| release (`cargo test --release --lib`, rerun) | 95.30ms | 1.91ms |

**Recorded per-step budget for M1.7 to cite: ~2.0ms/step (release build) at
the 1024x1024 reference grid, on this dev container**, with the debug-build
number (~14ms/step) recorded alongside for anyone iterating without
`--release`. Both numbers are for this milestone's **identity-only** step —
they measure the stepping *mechanism's* overhead (loop + double-buffer
swap + `FixedTimestep` accounting), not any real per-cell physics cost, since
M1.1 deliberately has none yet (see `src/grid.rs`'s `step_once` doc comment).
Real per-material behaviour arriving in M1.2+ will add to this baseline, not
replace it — this number is the floor, not the whole later budget.

**Test results.**

- `cargo test --lib`: 65 passed, 1 ignored (the new timing test, by design),
  0 failed, 0.01s.
- `cargo test`: all of the above plus `tests/native_fallback.rs` and
  `tests/render_native.rs` (both native-subprocess integration tests) pass;
  doc-tests: 0 (none exist), 0 failed.
- `cargo build --lib --target wasm32-unknown-unknown`: compiles clean, no
  errors, no warnings, in 0.34s (incremental).
- `cargo test --lib -- --ignored reference_grid_step_timing --nocapture`
  (both debug and `--release`): confirmed passing, output captured above.

**Per-goal — met/not-met.**

1. **Choose and record a reference grid size, with reasoning.** Met —
   1024x1024, reasoning above.
2. **Time a step of that reference grid using the existing
   `Grid::step`/`FixedTimestep` path (unmodified), native only.** Met —
   `reference_grid_step_timing` calls `Grid::step` exactly as every other
   round's tests do; no wasm/browser timing attempted; `Grid::step`,
   `Grid::step_once`, and `FixedTimestep` are byte-for-byte unchanged
   (`git diff` shows only additions).
3. **Record the number and how it was measured** (machine context,
   methodology, resulting per-step budget). Met — this section.
4. **A test or checked-in fixture makes the timing reproducible.** Met —
   `reference_grid_step_timing`, `#[ignore]`d, checked into `src/grid.rs`,
   re-runnable with the documented command; not a one-off number that only
   exists as prose.
5. **No production behaviour changes.** Met — the diff is one new
   `#[ignore]`d test plus two `const`s, nothing else; `Grid::step`'s
   identity/no-op semantics are untouched (confirmed both by reading the
   diff and by the identity-transform tests in the same file, already
   present before this round, still passing unchanged).

**Successes.**

- All five goals met, independently verifiable by anyone re-running the
  documented command — the number in this report is not the only place it
  exists.
- Found a real constraint (`FixedTimestep`'s crate-private visibility) that
  would have silently forced either a production-code visibility change or a
  broken external-test approach, and resolved it the additive-only way
  (in-crate test) rather than the path that touches `src/lib.rs`.
- Confirmed run-to-run variance directly (two runs each for debug and
  release) rather than reporting a single, possibly-lucky sample — the
  ~10-15% spread is now visible and honestly reported, not hidden behind one
  number.

**What was difficult, and where the time went.**

Nothing structurally difficult; this was a small, additive round. Most of
the time went to: (a) reading all four prior rounds' worth of `src/grid.rs`
content and this milestone's plan before writing anything, per "do one thing
at a time"; (b) working out the `FixedTimestep`-visibility constraint above
and deciding the in-crate-test resolution rather than reaching for the more
obvious-looking (but scope-violating) `pub mod timestep` fix; (c) running the
timing test four times (two builds x two runs each) to get a defensible,
non-single-sample number rather than reporting the first run seen.

**Compromises I made.**

- The timing test's only assertion is a generous sanity ceiling
  (`per_step.as_secs_f64() < 1.0`, i.e. under 1 full second) — it does not
  assert against the specific ~2ms/~15ms numbers recorded above. This is
  deliberate: hard-coding this dev container's exact number into the test
  itself would make the test fail on any other machine's ordinary,
  non-regressive variance (a slower CI runner, a different core count), which
  is exactly the kind of "loosen a threshold to make CI pass" trap
  `cycle-green` warns against in the other direction. The sanity ceiling
  still catches a genuine regression (e.g. an accidental O(n²) bug) while
  leaving the *specific* budget number to live in this report, where M1.7
  can cite it as a target to hold future work to, rather than as a test
  assertion this round has no authority to freeze for a machine it doesn't
  control.
- I recorded both a debug and a release number rather than picking one as
  "the" figure, and named release (~2.0ms/step) as the one for M1.7 to
  cite — release is the build that matters for an actual running
  simulation's frame budget; debug is recorded alongside only because it's
  what `cargo test --lib` runs by default and a future reader timing things
  informally via the fast path will see debug-speed numbers first.

**Token usage.** Not visible to me — no context/usage indicator or
final-turn summary surfaced by this harness.

**Gaps and flags.**

- This round's number describes only the identity-step's own overhead — the
  loop-and-swap mechanism, not any real physics. M1.2 onward will need to
  re-measure (or extend this same test) once real per-cell behaviour exists,
  and the "floor, not the whole budget" framing above should carry forward
  so nobody mistakes this milestone's number for a promise about a
  physics-bearing step.
- I did not add a release-mode variant of the `#[ignore]`d test itself (e.g.
  two separate tests, one implicitly meant for `--release`) — the single
  test's own doc comment tells a future runner to add `--release` for the
  optimized number, rather than the suite maintaining two near-duplicate
  tests for one measurement. If a later round wants both numbers asserted
  automatically rather than read from stdout, that's a genuine follow-up,
  not assumed done here.
- The stray, untracked `test/` directory (PNG output from ad-hoc native-binary
  runs) is still present in the working tree, as multiple prior rounds have
  already noted — untouched by this round, still worth someone's decision to
  `.gitignore` or delete it outside any single round's scope.
- I did not touch `src/scenario.rs` or `src/measure.rs` at all, despite both
  being listed in this round's dispatch context — read them for context
  (confirming `Scenario`/`run_headless` don't need to be involved in a raw
  `Grid`-level timing measurement) but found no genuine need to build the
  reference grid via a `Scenario` rather than `Grid::new` directly, since the
  round's goal is timing the stepping mechanism itself, not a specific named
  scenario's content. Flagging this choice explicitly in case a future
  reader expected the reference grid to be `Scenario`-shaped.

**General comments.**

Single-pass was the right shape call for this round, matching the plan's
own reasoning: no shared primitive changed, no conservation/determinism
target at stake (wall-clock cost, not physical correctness), no interface or
data-format change, and no prior exit ramp on this goal. The round closed in
well under the 30-minute budget with no need for a second pass.

**Elapsed:** ~4 minutes (2026-09-05T11:00:47+12:00 →
2026-09-05T11:04:30+12:00, per the two `date -Is` checks taken during this
phase), well inside the 30-minute decision point.

## Orchestrator close-out

**Independent re-verification performed** (cold, before trusting the
agent's report):

- `git log`/`git status --short` → commit `d598307` present as claimed;
  tree clean apart from the pre-existing, unrelated untracked `test/`
  directory.
- `cargo test --lib` → 65 passed, 0 failed, 1 ignored (the new
  `reference_grid_step_timing` test) — matches the report exactly.
- `cargo test` (full native suite) → all green, including
  `tests/native_fallback.rs` and `tests/render_native.rs`; doc-tests 0/0.
- `cargo build --lib --target wasm32-unknown-unknown` → clean.
- Ran the ignored timing test directly (`cargo test --lib -- --ignored
  reference_grid`) → passes, 0.86s wall time for the whole test (55 total
  `Grid::step` calls in an unoptimized debug build) — consistent with the
  reported ~14ms/step debug figure and confirms the test is real and
  reproducible, not a stub.

All of this matches the agent's own report; nothing found that
contradicts it.

**Verdict: Advance.** All 5 round goals met.

**Per-goal detail:**

1. Reference grid size chosen and recorded — 1024×1024 (1,048,576 cells),
   with stated reasoning (large enough to stand in for "the large
   universe," a clean power of two, small enough to time comfortably even
   unoptimized) — met.
2. Timed via the existing, unmodified native `Grid::step`/`FixedTimestep`
   path — met; confirmed `Grid::step`, `Grid::step_once`, and
   `FixedTimestep` are byte-for-byte untouched (`git diff --stat` on the
   commit shows only `src/grid.rs` additions plus the round log).
3. Number and methodology recorded in this file (grid size, machine
   context, warm-up/measured-step counts, debug vs. release numbers) —
   met. **Recorded reference budget: ~2.0ms/step (release build) at
   1024×1024 on this dev container** (2.19ms and 1.91ms across two runs),
   with the debug figures (14.86ms, 13.24ms) kept alongside for
   context — this is the number M1.7 should cite, understood explicitly
   as the identity-step's mechanism floor, not a promise about future
   physics cost.
4. Reproducible via a checked-in `#[ignore]`d test in `src/grid.rs` — met
   and independently re-run above.
5. No production behaviour changed — met, confirmed via diff.

This closes **milestone target 3** ("the reference grid steps within a
stated per-step budget") and is the first fixing of tranche-1 target 6's
number.

**Judgment call on the test-location deviation:** the round file
suggested a `#[test]` "or a small native binary/example"; the agent chose
neither a new binary nor a new `tests/` integration file, instead adding
the test inside `src/grid.rs`'s existing module, because `FixedTimestep`
lives behind a crate-private `mod timestep;` and an external test/binary
can't name it without a visibility change to production code (out of this
round's additive-only scope). This is the right call — it stays inside
the round's own constraints rather than quietly loosening
`timestep.rs`'s visibility to fit a binary shape nobody actually needed.

**Timing:** single-pass round, ~4 minutes wall time — well inside the
30-minute budget, no exit ramp taken.

**Gaps/flags carried forward:**

- The timing test's only automated assertion is a generous 1-second
  sanity ceiling, not the specific ~2ms figure — deliberate, to avoid
  flakiness across machines; the actual budget number lives in this
  report for M1.7 to cite by hand, not as a frozen, machine-specific test
  assertion. Worth keeping in mind: M1.7 will need to decide how it wants
  to *use* this number (a CI gate? a manual check?) — not this round's
  job to decide.
- The release-build number requires a manual `--release` re-run per the
  test's own doc comment (not automatically asserted) — a reasonable
  single-pass-scope choice, flagged in case a future round wants both
  numbers asserted automatically.
- Reference grid built directly via `Grid::new`, not through a
  `Scenario` — deliberate and correct (the round measures the stepping
  mechanism itself, not any named scenario's content), but flagged since
  the dispatch context included `scenario.rs`/`measure.rs` and a future
  reader might otherwise expect scenario-shaped timing.
- Carried forward unchanged: stray untracked `test/` dir (still harmless,
  `rm` still blocked by sandboxing, still nobody's single-round scope);
  whole-crate `cargo fmt` adoption still deferred to the milestone-scope
  Refactor pass; `paint_scenario`'s hardcoded fixture (Round 4); density/
  heat_capacity unit mismatch; `total_mass` unit unpinned; hand-rolled
  JSON has no string-escaping; `Grid`'s out-of-bounds panic contract
  provisional; `Scenario`'s owned (not shared) `MaterialTable`.

**All 5 milestone targets are now met** (targets 1, 2, 4, 5 closed in
Rounds 1-4; target 3 closed here). Next: the milestone-scope Refactor
pass per `cycle-milestone` §3, then the M1.1 closeout.
