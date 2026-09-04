# Round 3 — Fixed-timestep harness

**Milestone:** M0.4 — Mathematical foundations (`cycle-log/tranche-0/m0.4/plan.md`)

**Milestone intent:** the small, boring substrate every later tranche reaches
for — vector/grid primitives, a numeric type decision, a fixed-timestep
harness — got right once, here, rather than reinvented per-tranche.

**Round goals:**

1. Introduce an accumulator-style fixed-timestep stepping harness: given a
   fixed `dt` and a stream of variable, real (wall-clock or otherwise
   irregular) frame durations, it decides how many fixed steps of size `dt`
   have elapsed, carrying remainder time forward across calls.
2. The harness must be usable independent of rendering — it takes a duration
   and a step callback (or returns a step count), with no dependency on
   `web-sys`/canvas/wasm, so the same harness works in `cargo test` on the
   native target and later inside the wasm build.
3. Guard against runaway/spiral-of-death behaviour: an unusually large frame
   duration (e.g. a tab backgrounded for seconds) must not cause the harness
   to try to "catch up" an unbounded number of steps in one call — cap the
   number of steps taken per call (or an equivalent guard), and document the
   choice.
4. Unit tests exercising the harness directly (no browser, no wasm): feeding
   a sequence of frame durations and asserting the right number of fixed
   steps occur in total and per-call, including a case that would trigger
   the spiral-of-death guard.

**Round 1 & 2 carried forward:** `src/math.rs` now has `Scalar = f32`,
`Vec2` (add/sub/scale/dot, pinned y-up convention, `UP` constant), and
`GridIndex` (cell-center convention, `center(cell_size) -> Vec2`). Both
rounds advanced cleanly. This round's harness may use `Scalar` for its `dt`
type if convenient, but does not need `Vec2`/`GridIndex` directly — build it
as its own module/type; your call on file placement (a new `src/timestep.rs`
wired via `mod` in `lib.rs`, or elsewhere) as long as it does not touch
`src/lib.rs`'s existing hello-world logic (round 4 owns that retrofit).

**Push on vs. patch back:** pushing on. No carried-forward gaps block this
round.

**Refactor scope/focus:** round scope (the new harness module); focus on
the harness's determinism/robustness to variable frame durations — the
spiral-of-death guard specifically, since every later tranche's step loop
depends on this being right, and this is exactly the kind of latent bug an
isolated Green pass could get subtly wrong (off-by-one step counts, guard
that fires too early/late, remainder not carried correctly across calls).

## Round 03 — Red — 2026-09-05T03:14:00+12:00 → 2026-09-05T03:31:18+12:00 (~17m)

**What I did.**

Wrote `src/timestep.rs`: a new module implementing an accumulator-style
fixed-timestep harness (`FixedTimestep`), independent of `src/math.rs` and
`src/lib.rs`'s existing canvas/tick logic apart from reusing `Scalar` for
`dt`/durations. Wired it into `src/lib.rs` with a single `mod timestep;`
line (plus a short doc comment), mirroring exactly how `mod math;` was added
in round 1 — no other line of `lib.rs` touched. Did not restructure the
round's goal list; the four goals as handed to me formed one coherent slice
(the harness, its render-independence, the guard, and its tests) and none of
them depended on a mechanism that doesn't exist yet, so no exit ramp or
resequencing was needed.

**Design decisions made in Red (Green must not deviate without flagging back):**

- **Clamp the input duration, not just the reported step count.** Given a
  `frame_duration` far larger than `max_steps_per_call * dt`, `advance` must
  clamp the *time added to the accumulator* to that cap before extracting
  steps — not merely cap what's returned while still accumulating the full
  duration. The latter would only spread the same runaway catch-up across
  future calls instead of preventing it; the former means no future call
  ever inherits a backlog. Documented at length in the module doc comment,
  and pinned by two scenario tests
  (`a_huge_frame_duration_is_capped_not_allowed_to_catch_up_unbounded`,
  `the_guard_does_not_leave_a_debt_for_the_next_call_to_pay_off`) — the
  second specifically checks no debt survives into the next ordinary call,
  which is the property a lesser ("cap only what's returned") guard would
  fail.
- **Two entry points, one goal.** Goal 2 says "a duration and a step
  callback (**or** returns a step count)" — implemented both:
  `advance(frame_duration) -> u32` (the primitive, easiest to test directly)
  and `step_with(frame_duration, on_step: F) -> u32` (a thin wrapper that
  also invokes a callback once per elapsed step, for a caller that wants to
  run the step itself rather than count it). Green should implement
  `step_with` in terms of `advance`, not duplicate its logic.
- **`dt`/duration units:** left as "seconds by convention, matching this
  crate's other `Scalar` uses" — not enforced by the type system (no
  newtype), matching `src/math.rs`'s existing philosophy of not validating
  scalar inputs (e.g. `GridIndex::center`'s unchecked `cell_size`).
- **`DEFAULT_MAX_STEPS_PER_CALL = 5`**, used by `FixedTimestep::new`;
  `with_max_steps_per_call` for a caller that wants a different cap. Reasoning
  for `5` documented on the constant itself: small enough to prevent a
  multi-second stall from freezing on catch-up, large enough to absorb
  ordinary frame jitter.

**Successes.**

- All four round goals have a concrete skeleton and at least one scenario
  test:
  1. Accumulator/remainder-carrying: `a_short_frame_produces_no_step_yet`,
     `two_short_frames_summing_to_one_dt_elapse_exactly_one_step`,
     `irregular_frame_durations_total_the_expected_step_count`,
     `same_total_time_yields_same_total_steps_regardless_of_chunking`,
     `leftover_remainder_is_visible_in_the_accumulator_between_calls`.
  2. Render-independence: the whole module has zero `web-sys`/`wasm-bindgen`
     imports — only `crate::math::Scalar`; both entry points
     (`advance`/`step_with`) are exercised directly in plain `cargo test`.
  3. Spiral-of-death guard:
     `a_huge_frame_duration_is_capped_not_allowed_to_catch_up_unbounded`,
     `the_guard_does_not_leave_a_debt_for_the_next_call_to_pay_off`, plus the
     module doc comment's explicit design-choice writeup.
  4. Unit tests exercising the harness directly, feeding sequences of
     durations and asserting per-call and total step counts, including the
     guard-triggering case — all of the above, run under plain
     `cargo test --lib`, no browser, no wasm.
- Confirmed red for the right reason: all 12 new tests panic on
  `not yet implemented: construct a FixedTimestep with a zeroed accumulator`
  — the stub in `with_max_steps_per_call`, which every other stubbed method
  transitively depends on (a single, clear failure signature, not 12
  different ones).
- All 18 pre-existing tests (9 in `math::tests`, 4 in `lib.rs`'s own
  `tests`... — actually 5 counting `render_frame_paints_rect_in_tick_colour`,
  see exact count below) remain green: this round's addition did not
  disturb rounds 1/2's work.
- `cargo build --lib` compiles clean apart from expected `dead_code`
  warnings on the new module's public surface (nothing outside its own
  tests calls it yet — same situation `math.rs` was in after rounds 1-2,
  and resolves the same way once a later round retrofits `lib.rs`'s tick
  loop to use it).
- `cargo clippy --lib` adds nothing new against `timestep.rs` beyond the
  same dead-code warnings; its one substantive finding
  (`tick % 2 == 0` → `.is_multiple_of(2)`) is pre-existing in `src/lib.rs`,
  outside this round's scope (already flagged by round 2's Refactor).

**What was difficult, and where the time went.**

Most of the time went into (a) reading rounds 1 and 2's logs and
`src/math.rs`/`src/lib.rs` in full to match established doc-comment and
test-naming conventions rather than inventing a new house style, and (b)
working out, on paper, which of the two spiral-of-death guard shapes
("clamp reported steps" vs. "clamp accumulated input time") actually
prevents future-call backlog rather than merely slowing it — then writing a
test (`the_guard_does_not_leave_a_debt_for_the_next_call_to_pay_off`) that
specifically distinguishes the two, so Green can't accidentally ship the
weaker guard and still pass the suite. One small compile hiccup: an
untyped `0.05` literal in a test needed an explicit `Scalar` annotation
(ambiguous-numeric-type error) — fixed immediately, not a design issue.

**Compromises I made.**

- No validation of `dt <= 0` or negative `frame_duration` — matches
  `src/math.rs`'s established "don't validate scalar inputs" philosophy
  (see round 2's refactor discussion of unvalidated `cell_size`). Not
  asked for by any round goal; flagging so Green doesn't feel obligated to
  add guards I didn't test for.
- `step_with` is specified (in its doc comment) as "call `advance`, then
  invoke `on_step` once per elapsed step" — I did not write a test pinning
  that it's implemented *in terms of* `advance` rather than duplicating the
  accumulator logic separately, since that's an implementation-structure
  preference, not externally observable behaviour. The externally-visible
  contract (callback fires exactly `advance`'s returned count of times) is
  pinned by `step_with_invokes_the_callback_once_per_elapsed_step`.
- Did not add a test for `dt() `/`max_steps_per_call()` changing after
  construction (they don't — there's no setter) — considered out of scope,
  plain accessors.

**Gaps and flags.**

- This round explicitly does **not** retrofit `src/lib.rs`'s `TICK`
  counter to use `FixedTimestep` — per the round framing, that's a later
  round's job. `lib.rs`'s own tick loop is untouched apart from the `mod`
  line.
- Dead-code warnings on all of `timestep.rs`'s public surface are expected
  right now (same pattern rounds 1-2 went through with `math.rs`) and will
  resolve once the retrofit round wires it in. Not a defect of this round.
- The module assumes `frame_duration`/`dt` are non-negative; nothing
  enforces or tests negative input. If a later round's retrofit could ever
  feed a negative duration (it shouldn't, from a wall-clock source), that's
  new ground, not covered here.
- I did not explore whether `FixedTimestep` should also expose an
  "interpolation alpha" (`accumulator() / dt()`) for render interpolation
  between fixed steps — a common companion feature to this exact pattern,
  but not asked for by any of the four round goals, so left out rather than
  guessed at. Flagging in case a future round's rendering retrofit wants
  smoothed motion between fixed steps rather than snapping to the latest
  one.

**General comments.**

Did not restructure the round's goal slice — the four goals as given formed
one coherent, landable-in-one-round unit, and none surfaced a hidden
ordering problem (no goal here depends on a mechanism a later round hasn't
built yet).

**Test file path:** `src/timestep.rs` (tests live in its own `#[cfg(test)]
mod tests` at the bottom, following `math.rs`'s and `lib.rs`'s existing
pattern — no separate `tests/` file).

**Currently-failing tests (all panic on the same named stub,
`with_max_steps_per_call`'s `todo!()`, since every other method depends on
constructing a `FixedTimestep` first):**

- `a_short_frame_produces_no_step_yet` — goal 1, sub-`dt` frame yields 0 steps.
- `two_short_frames_summing_to_one_dt_elapse_exactly_one_step` — goal 1, remainder across 2 calls.
- `irregular_frame_durations_total_the_expected_step_count` — goal 1, non-round-number durations.
- `same_total_time_yields_same_total_steps_regardless_of_chunking` — goal 1, call-granularity independence.
- `leftover_remainder_is_visible_in_the_accumulator_between_calls` — goal 1, remainder introspection.
- `a_huge_frame_duration_is_capped_not_allowed_to_catch_up_unbounded` — goal 3, guard fires.
- `the_guard_does_not_leave_a_debt_for_the_next_call_to_pay_off` — goal 3, guard doesn't defer.
- `step_with_invokes_the_callback_once_per_elapsed_step` — goal 2, callback entry point.
- `a_zero_duration_frame_produces_no_step_and_does_not_panic` — disposable, edge case.
- `an_exact_multiple_of_dt_leaves_a_zero_remainder` — disposable, edge case.
- `new_uses_the_documented_default_cap` — disposable, default wiring.
- `a_fresh_harness_starts_with_the_given_dt_cap_and_no_accumulated_time` — disposable, constructor/accessors.

**Skeleton created (every path/type/signature Green must fill in):**

- `src/timestep.rs` (new file), module-level:
  - `pub const DEFAULT_MAX_STEPS_PER_CALL: u32 = 5;` — real value, not a
    stub (a constant declaration, not an algorithm).
  - `pub struct FixedTimestep { dt: Scalar, max_steps_per_call: u32, accumulator: Scalar }`
  - `impl FixedTimestep`:
    - `pub fn new(dt: Scalar) -> Self` — real body (`Self::with_max_steps_per_call(dt, DEFAULT_MAX_STEPS_PER_CALL)`), not stubbed, since it's plumbing over the stub below.
    - `pub fn with_max_steps_per_call(dt: Scalar, max_steps_per_call: u32) -> Self` — `todo!()`.
    - `pub fn dt(&self) -> Scalar` — `todo!()`.
    - `pub fn max_steps_per_call(&self) -> u32` — `todo!()`.
    - `pub fn accumulator(&self) -> Scalar` — `todo!()`.
    - `pub fn advance(&mut self, frame_duration: Scalar) -> u32` — `todo!()`.
    - `pub fn step_with<F: FnMut()>(&mut self, frame_duration: Scalar, on_step: F) -> u32` — `todo!()`.
- `src/lib.rs`: one line added, `mod timestep;` (plus a 3-line doc comment
  above it) — no other line changed.

**Commands:**

- This round's tests only: `cargo test --lib timestep::`
- Full suite: `cargo test --lib`
- Build check: `cargo build --lib`
- Lint: `cargo clippy --lib`

**Durable scenarios vs. disposable unit tests:**

- Durable (restate the round's goals, should survive Green/Refactor
  reshaping the internals): `a_short_frame_produces_no_step_yet`,
  `two_short_frames_summing_to_one_dt_elapse_exactly_one_step`,
  `irregular_frame_durations_total_the_expected_step_count`,
  `same_total_time_yields_same_total_steps_regardless_of_chunking`,
  `leftover_remainder_is_visible_in_the_accumulator_between_calls`,
  `a_huge_frame_duration_is_capped_not_allowed_to_catch_up_unbounded`,
  `the_guard_does_not_leave_a_debt_for_the_next_call_to_pay_off`,
  `step_with_invokes_the_callback_once_per_elapsed_step`.
- Disposable (may be deleted if the implementation's shape changes):
  `a_zero_duration_frame_produces_no_step_and_does_not_panic`,
  `an_exact_multiple_of_dt_leaves_a_zero_remainder`,
  `new_uses_the_documented_default_cap`,
  `a_fresh_harness_starts_with_the_given_dt_cap_and_no_accumulated_time`.

**Confirmation the rest of the suite is green.** `cargo test --lib` (full
run, before the new module's stubs make it fail overall): 18 pre-existing
tests (9 `math::tests::*`, 4 `tests::*` non-timestep tests plus
`render_frame_paints_rect_in_tick_colour` and
`color_for_tick_alternates_by_parity` — 5 total in `lib.rs`'s own `tests`
module) all still pass; only the 12 new `timestep::tests::*` fail, all on
the same named stub. Suite runtime: 0.00s reported (30 tests total,
trivial arithmetic — no performance concern, nothing to tag as slow).

**Restructuring:** none. Goals taken as handed.

## Round 03 — Green — 2026-09-05T03:32:59+12:00 → 2026-09-05T03:38:00+12:00 (~5m)

**What I did.**

Filled in every `todo!()` stub in `src/timestep.rs` left by Red, in place —
no signature changed:

- `with_max_steps_per_call` — constructs `Self` with `accumulator: 0.0`.
- `dt`/`max_steps_per_call`/`accumulator` — plain field accessors.
- `advance` — computes `max_accumulate = max_steps_per_call as Scalar *
  dt`, clamps `frame_duration` to it with `.min(...)` *before* adding to
  `self.accumulator` (the module doc comment's documented design: clamp the
  input, not just the reported count), then extracts whole `dt`-sized steps
  in a `while` loop bounded by `max_steps_per_call`, subtracting `dt` from
  the accumulator per step and counting.
- `step_with` — calls `advance`, then loops `on_step()` that many times,
  returning the same count. Implemented in terms of `advance`, not a
  duplicate accumulator, per Red's stated preference.

One thing added beyond a literal stub-fill: a `const EPSILON: Scalar =
1e-5` tolerance in `advance`'s step-extraction comparison (`accumulator +
EPSILON >= dt` instead of `accumulator >= dt`). Without it,
`irregular_frame_durations_total_the_expected_step_count` and
`same_total_time_yields_same_total_steps_regardless_of_chunking`'s
"many small calls" side both under-counted by exactly one step. This is
`f32` rounding, not a logic bug — I isolated it with a standalone repro
(summing `[0.03, 0.12, 0.02, 0.11, 0.22]` against `dt = 0.1` in plain
`f32` arithmetic outside the crate): the true accumulator value after all
five adds is `0.0999999866`, a few ULPs short of `0.1`, so a strict `>=`
silently drops the fifth step. `1e-5` is far smaller than any `dt` this
harness is meant for (the smallest in the test suite is `0.02`) so it
never manufactures a step time didn't earn — it only recovers steps lost
to accumulated float error. This is the one place I "proposed a number"
rather than treating Red's skeleton as complete; measurement is above,
verifiable by rerunning that repro.

**Commands run and results.**

- `cargo test --lib timestep::` — 12 tests: 11 passed, 1 failed
  (`same_total_time_yields_same_total_steps_regardless_of_chunking`, see
  below).
- `cargo test --lib` (full suite) — 30 tests: 29 passed, 1 failed (the
  same one). All 18 pre-existing tests still green.

**Still failing: `same_total_time_yields_same_total_steps_regardless_of_chunking`.**

```
assertion `left == right` failed
  left: 5
 right: 17
```

This is not an implementation defect — it is the test's chosen inputs
directly contradicting the guard the round's own goal 3 requires, and I
did not weaken either side to force it green. Walkthrough:

- `dt = 0.1`, `total_time = 1.7`, both harnesses built via
  `FixedTimestep::new(dt)` → `max_steps_per_call = DEFAULT_MAX_STEPS_PER_CALL
  = 5`, so `max_accumulate = 5 * 0.1 = 0.5`.
- **One big call:** `advance(1.7)` clamps `1.7` down to `0.5` *before*
  accumulating (module doc comment, goal 3) — this is exactly the
  documented spiral-of-death guard doing its job on a single oversized
  duration. `0.5 / 0.1 = 5` steps. Result: `5`.
- **Many small calls:** chunked in steps of `0.05`, so no single `advance`
  call ever approaches the `0.5` clamp threshold — nothing is ever
  discarded, and the accumulator faithfully reaches the full `1.7` over
  many calls. `1.7 / 0.1 = 17` steps. Result: `17`.

Both numbers are correct outputs of the documented design; they disagree
because the test's `total_time` (`1.7`) exceeds what a *single* call can
ever honestly turn into steps under the default cap (`0.5`), so the
"same total, regardless of chunking" property this test asserts and the
"a single oversized call is capped, permanently losing simulated time"
property goal 3 asserts are mutually exclusive for this specific input.
Neither is a bug in `advance`; they're two round goals whose chosen test
inputs collide. I did not touch this test (out of Green's scope: `src/
timestep.rs`'s tests belong to Red, and the contract says propose a
number rather than loosen a threshold silently).

**Proposed fix (for Red/Refactor, not applied by me):** either (a) shrink
`total_time` in this test to something `<= max_steps_per_call * dt`
(`0.5` here) so a single call can honestly match the chunked total — e.g.
`0.35` (`3` and `3` steps for `dt = 0.1`, cap `5`) — or (b) build both
harnesses in this test via `with_max_steps_per_call(dt, some_larger_cap)`
so `1.7`'s single-call total (`17` steps) sits under the cap and the
guard never engages for either side. Either preserves the scenario's
actual intent (call-granularity independence) without touching goal 3's
guard tests, which are unaffected by this. I did not pick one and edit
the test myself — that's a design call for whoever owns Red's test intent
next (Refactor, or a follow-up Red patch), not an implementation
decision.

**Signature changes from Red's skeleton.** None. `step_with`'s parameter
was un-named (`on_step: F`, not `mut on_step: F`) in the skeleton
signature comment but that's a binding-mutability detail inside the body,
not part of the public signature — no external change.

**What I noticed but left for Refactor.**

- The `EPSILON` constant is a private `const` local to `advance`'s body,
  not a module-level item — fine for a single use site, but if a future
  round adds another comparison against `dt` it should probably move to
  module scope rather than be redeclared. Left as-is since only one
  comparison needs it right now and duplicating a one-line const isn't a
  live problem yet.
- Did not attempt to fix
  `same_total_time_yields_same_total_steps_regardless_of_chunking` myself
  per the above — flagging loudly rather than silently patching the test
  or loosening the guard.

**Files touched.** `src/timestep.rs` only, per scope. `cycle-log/tranche-0/m0.4/round-03.md` for this report.

**Commit.** `1addcc1` — "Round 3 Green: implement FixedTimestep accumulator harness".
