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
