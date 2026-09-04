//! Fixed-timestep accumulator harness.
//!
//! This module is M0.4's third piece of "small, boring substrate" (see the
//! milestone's intent in `cycle-log/tranche-0/m0.4/plan.md`): every later
//! tranche that runs a simulation loop needs to decide how many fixed-size
//! physics steps have elapsed given an irregular stream of real frame
//! durations (wall-clock in a browser `requestAnimationFrame`, or anything
//! else irregular in a native/headless harness). Get the accumulator logic
//! and the spiral-of-death guard right once, here, rather than reinvented
//! per-tranche.
//!
//! Deliberately has no dependency on `web-sys`, `wasm-bindgen`, or any
//! rendering concept — it consumes a plain [`Scalar`] duration and either
//! returns a step count or drives a caller-supplied callback, so the exact
//! same type runs under `cargo test` on the native target and, later, inside
//! the wasm build. `src/lib.rs`'s existing tick counter is explicitly a
//! separate, throwaway thing (see its module doc comment) — this harness
//! does not replace it this round; a later round retrofits `lib.rs` to call
//! this instead.
//!
//! ## Design decision: clamp the *input duration*, not just the step count
//!
//! The classic "spiral of death" failure mode: a frame takes unusually long
//! (a backgrounded browser tab resumes after several seconds, a debugger
//! breakpoint pauses the process), the accumulator receives that whole
//! duration, and the harness tries to run enough fixed steps to "catch up" —
//! which takes real time, which produces a next frame duration that is
//! *also* huge, compounding forever.
//!
//! The guard here is [`FixedTimestep::max_steps_per_call`]: an unusually
//! large `frame_duration` is clamped, before it is added to the internal
//! accumulator, to at most `max_steps_per_call as Scalar * dt` worth of
//! simulated time. The excess is **discarded outright**, not deferred — the
//! simulation genuinely falls behind wall-clock time for that one call
//! rather than owing a debt of extra steps to future calls. This is the
//! stronger of the two guards this problem admits:
//!
//! - capping only the *steps returned* per call, while still accumulating
//!   the full (huge) duration, would spread the same unbounded catch-up
//!   across many subsequent calls instead of one — still a spiral, just a
//!   slower one;
//! - clamping the duration *before* accumulation, as done here, means the
//!   accumulator can never hold more than `max_steps_per_call * dt` of
//!   pending time, so no future call inherits a backlog at all.
//!
//! The trade this accepts: after a long stall, simulated time permanently
//! lags wall-clock time by however much was discarded. That is judged the
//! right trade for a "believable small world" — a visibly-consistent
//! simulation rate beats a simulation that either freezes the UI catching up
//! or drifts into a runaway loop.

use crate::math::Scalar;

/// The default cap on how many fixed steps [`FixedTimestep::advance`] (or
/// [`FixedTimestep::step_with`]) will report/run in a single call, used by
/// [`FixedTimestep::new`].
///
/// Chosen as a small, conservative number rather than tuned to any
/// particular `dt`: even at a fast simulation rate, 5 steps is enough to
/// absorb ordinary frame-time jitter (a slow frame here or there) without
/// visible stutter, while still being far below "enough steps to freeze the
/// UI catching up after a multi-second stall". A caller with different
/// needs (a coarser or finer `dt`, a different jitter tolerance) can pick
/// its own value via [`FixedTimestep::with_max_steps_per_call`].
pub const DEFAULT_MAX_STEPS_PER_CALL: u32 = 5;

/// An accumulator-style fixed-timestep stepping harness.
///
/// Feed it a stream of variable, real frame durations via [`advance`] (or
/// [`step_with`]); it tracks how much simulated time is owed in an internal
/// accumulator and reports how many fixed steps of size `dt` have elapsed,
/// carrying any leftover remainder forward to the next call. See the module
/// doc comment for the spiral-of-death guard this type enforces.
///
/// [`advance`]: FixedTimestep::advance
/// [`step_with`]: FixedTimestep::step_with
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedTimestep {
    dt: Scalar,
    max_steps_per_call: u32,
    accumulator: Scalar,
}

impl FixedTimestep {
    /// Builds a harness with the given fixed step size `dt` and the default
    /// spiral-of-death cap ([`DEFAULT_MAX_STEPS_PER_CALL`]).
    ///
    /// `dt` is assumed to be a positive step duration, in the same units as
    /// the `frame_duration` values later passed to [`advance`]/[`step_with`]
    /// (this crate uses seconds by convention, matching [`Scalar`]'s other
    /// uses). As with `src/math.rs`'s primitives, this is not validated at
    /// construction.
    ///
    /// [`advance`]: FixedTimestep::advance
    /// [`step_with`]: FixedTimestep::step_with
    pub fn new(dt: Scalar) -> Self {
        Self::with_max_steps_per_call(dt, DEFAULT_MAX_STEPS_PER_CALL)
    }

    /// Builds a harness with the given fixed step size `dt` and an explicit
    /// spiral-of-death cap: [`advance`]/[`step_with`] will never report (or
    /// run) more than `max_steps_per_call` steps from a single call, no
    /// matter how large the `frame_duration` passed in.
    ///
    /// [`advance`]: FixedTimestep::advance
    /// [`step_with`]: FixedTimestep::step_with
    pub fn with_max_steps_per_call(dt: Scalar, max_steps_per_call: u32) -> Self {
        let _ = (dt, max_steps_per_call);
        todo!("construct a FixedTimestep with a zeroed accumulator")
    }

    /// The fixed step size this harness was built with.
    pub fn dt(&self) -> Scalar {
        todo!("return the stored dt")
    }

    /// The spiral-of-death cap this harness was built with.
    pub fn max_steps_per_call(&self) -> u32 {
        todo!("return the stored max_steps_per_call")
    }

    /// The amount of simulated time currently owed but not yet turned into a
    /// fixed step — the remainder carried forward across calls to
    /// [`advance`]/[`step_with`]. Always in `[0, dt)` after a call completes
    /// normally (see the module doc comment for the one case where the
    /// spiral-of-death guard discards time instead of carrying it).
    ///
    /// [`advance`]: FixedTimestep::advance
    /// [`step_with`]: FixedTimestep::step_with
    pub fn accumulator(&self) -> Scalar {
        todo!("return the current accumulator value")
    }

    /// Records that `frame_duration` of real time has elapsed since the
    /// last call, and returns how many fixed steps of size `dt` have now
    /// elapsed as a result — carrying any leftover remainder in the internal
    /// accumulator forward to the next call.
    ///
    /// `frame_duration` is clamped, before being added to the accumulator,
    /// to at most `max_steps_per_call as Scalar * dt` worth of time — see
    /// the module doc comment for why this is the spiral-of-death guard and
    /// what it trades away (discarded simulated time after a long stall,
    /// rather than a deferred backlog of extra steps).
    ///
    /// Does not run any simulation step itself — it only counts them. Use
    /// [`step_with`] when a callback should be invoked once per elapsed
    /// step.
    ///
    /// [`step_with`]: FixedTimestep::step_with
    pub fn advance(&mut self, frame_duration: Scalar) -> u32 {
        let _ = frame_duration;
        todo!("clamp frame_duration, accumulate it, and extract whole dt-sized steps")
    }

    /// Convenience wrapper over [`advance`]: records `frame_duration`,
    /// invokes `on_step` once for each fixed step that has elapsed, and
    /// returns the same step count `advance` would have.
    ///
    /// This is the render-independent "step callback" entry point: a native
    /// test harness or a future wasm render loop can pass a closure that
    /// runs one simulation tick, without either caller needing to know how
    /// the accumulator works.
    ///
    /// [`advance`]: FixedTimestep::advance
    pub fn step_with<F: FnMut()>(&mut self, frame_duration: Scalar, on_step: F) -> u32 {
        let _ = (frame_duration, on_step);
        todo!("call advance, then invoke on_step once per elapsed step")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Scenarios (durable: restate the round's goals) ---

    /// Scenario: a frame duration smaller than `dt` should not produce a
    /// step yet — the harness is meant to accumulate, not step on every
    /// call regardless of how little time passed.
    #[test]
    fn a_short_frame_produces_no_step_yet() {
        let mut ts = FixedTimestep::new(0.1);
        let steps = ts.advance(0.05);
        assert_eq!(steps, 0, "half a dt's worth of time should not yet elapse a step");
    }

    /// Scenario: feeding frames whose durations sum to exactly one `dt`,
    /// split across two calls, elapses exactly one step in total, and the
    /// remainder does not leak a spurious extra step — goal 1's "carries
    /// remainder time forward across calls" restated as the minimal case.
    #[test]
    fn two_short_frames_summing_to_one_dt_elapse_exactly_one_step() {
        let mut ts = FixedTimestep::new(0.1);
        let first = ts.advance(0.05);
        let second = ts.advance(0.05);
        assert_eq!(first, 0, "first half-dt frame should not step");
        assert_eq!(second, 1, "second half-dt frame should complete exactly one step");
    }

    /// Scenario: an irregular stream of frame durations (some longer, some
    /// shorter than `dt`, none a clean multiple) still yields, in total,
    /// the number of steps implied by the total elapsed time — the harness
    /// must work for genuinely variable, real (not synthetic, round-number)
    /// frame durations, per goal 1.
    #[test]
    fn irregular_frame_durations_total_the_expected_step_count() {
        let mut ts = FixedTimestep::new(0.1);
        // Durations: 0.03, 0.12, 0.02, 0.11, 0.22 -> total 0.50 -> 5 steps.
        let durations = [0.03, 0.12, 0.02, 0.11, 0.22];
        let total_steps: u32 = durations.iter().map(|&d| ts.advance(d)).sum();
        assert_eq!(total_steps, 5);
    }

    /// Scenario: however the same total real time is chopped up into calls
    /// (one huge call vs. many tiny ones), the harness reports the same
    /// total number of elapsed steps — the accumulator's job is to track
    /// total elapsed time regardless of call granularity.
    #[test]
    fn same_total_time_yields_same_total_steps_regardless_of_chunking() {
        let dt = 0.1;
        let total_time = 1.7_f32;

        let mut one_big_call = FixedTimestep::new(dt);
        let steps_one_call = one_big_call.advance(total_time);

        let mut many_small_calls = FixedTimestep::new(dt);
        let mut steps_many_calls = 0;
        let mut remaining = total_time;
        let chunk: Scalar = 0.05;
        while remaining > 0.0 {
            let this_chunk = chunk.min(remaining);
            steps_many_calls += many_small_calls.advance(this_chunk);
            remaining -= this_chunk;
        }

        assert_eq!(steps_one_call, steps_many_calls);
    }

    /// Scenario: the remainder left over after a partial step genuinely
    /// carries forward — after several sub-`dt` frames that together don't
    /// yet total a full `dt`, the accumulator reflects the leftover time,
    /// not zero.
    #[test]
    fn leftover_remainder_is_visible_in_the_accumulator_between_calls() {
        let mut ts = FixedTimestep::new(1.0);
        assert_eq!(ts.advance(0.3), 0);
        assert_eq!(ts.advance(0.3), 0);
        assert_eq!(ts.advance(0.3), 0);
        // 0.9 accumulated, still short of one full dt.
        assert!(
            (ts.accumulator() - 0.9).abs() < 1e-5,
            "expected ~0.9 owed, got {}",
            ts.accumulator()
        );
        // One more 0.3 pushes total to 1.2: exactly one step, 0.2 remainder.
        assert_eq!(ts.advance(0.3), 1);
        assert!(
            (ts.accumulator() - 0.2).abs() < 1e-5,
            "expected ~0.2 remainder after the step, got {}",
            ts.accumulator()
        );
    }

    /// Scenario (spiral-of-death guard, goal 3): an unusually large frame
    /// duration — e.g. a tab backgrounded for several seconds — must not
    /// make the harness try to run an unbounded number of catch-up steps in
    /// one call. With a small explicit cap, a huge duration reports at most
    /// the cap, never the "honest" (and much larger) step count.
    #[test]
    fn a_huge_frame_duration_is_capped_not_allowed_to_catch_up_unbounded() {
        let mut ts = FixedTimestep::with_max_steps_per_call(1.0 / 60.0, 5);
        // 10 seconds at 1/60s per step is ~600 steps if uncapped.
        let steps = ts.advance(10.0);
        assert_eq!(
            steps, 5,
            "a 10-second stall should be capped to max_steps_per_call, not ~600 honest steps"
        );
    }

    /// Scenario (spiral-of-death guard, continued): after the capped huge
    /// frame, the discarded time must not resurface as a backlog that the
    /// very next ordinary frame suddenly has to pay off — the guard clamps
    /// the input duration itself, not merely the steps reported for that
    /// one call, so no debt should be left pending.
    #[test]
    fn the_guard_does_not_leave_a_debt_for_the_next_call_to_pay_off() {
        let dt = 1.0 / 60.0;
        let mut ts = FixedTimestep::with_max_steps_per_call(dt, 5);
        ts.advance(10.0);
        // The accumulator after the capped call must not exceed one dt: no
        // leftover backlog beyond an ordinary remainder.
        assert!(
            ts.accumulator() < dt,
            "expected no more than one dt's worth of remainder after the \
             capped call, got {}",
            ts.accumulator()
        );
        // A normal, tiny next frame should not suddenly report a burst of
        // steps it didn't itself contain.
        let next = ts.advance(dt * 0.1);
        assert_eq!(
            next, 0,
            "a small ordinary frame right after the capped stall should not \
             pay off a hidden backlog"
        );
    }

    /// Scenario: the harness is usable via a step callback instead of a raw
    /// count — goal 2's "or returns a step count" alternative. The callback
    /// must run exactly once per elapsed step, matching the returned count.
    #[test]
    fn step_with_invokes_the_callback_once_per_elapsed_step() {
        let mut ts = FixedTimestep::new(0.1);
        let mut calls = 0;
        let reported = ts.step_with(0.35, || calls += 1);
        assert_eq!(reported, 3, "0.35 / 0.1 should elapse 3 whole steps");
        assert_eq!(calls, 3, "the callback should run exactly once per elapsed step");
    }

    // --- Disposable unit tests (may be deleted if the implementation changes shape) ---

    /// Disposable: a zero-duration frame is a legal no-op — no step, no
    /// panic, no change to the accumulator.
    #[test]
    fn a_zero_duration_frame_produces_no_step_and_does_not_panic() {
        let mut ts = FixedTimestep::new(0.1);
        assert_eq!(ts.advance(0.0), 0);
        assert_eq!(ts.accumulator(), 0.0);
    }

    /// Disposable: a frame duration that is an exact multiple of `dt`
    /// leaves nothing in the accumulator afterward.
    #[test]
    fn an_exact_multiple_of_dt_leaves_a_zero_remainder() {
        let mut ts = FixedTimestep::new(0.25);
        let steps = ts.advance(0.75);
        assert_eq!(steps, 3);
        assert!(
            ts.accumulator().abs() < 1e-5,
            "expected ~0 remainder, got {}",
            ts.accumulator()
        );
    }

    /// Disposable: `new` picks up [`DEFAULT_MAX_STEPS_PER_CALL`] so callers
    /// who don't care about the cap still get one.
    #[test]
    fn new_uses_the_documented_default_cap() {
        let ts = FixedTimestep::new(0.1);
        assert_eq!(ts.max_steps_per_call(), DEFAULT_MAX_STEPS_PER_CALL);
    }

    /// Disposable: the constructor and accessors round-trip the values they
    /// were given, and a freshly built harness starts with an empty
    /// accumulator.
    #[test]
    fn a_fresh_harness_starts_with_the_given_dt_cap_and_no_accumulated_time() {
        let ts = FixedTimestep::with_max_steps_per_call(0.02, 8);
        assert_eq!(ts.dt(), 0.02);
        assert_eq!(ts.max_steps_per_call(), 8);
        assert_eq!(ts.accumulator(), 0.0);
    }
}
