# Round 4 — Retrofit M0.1's hello-world onto the shared harness

**Milestone:** M0.4 — Mathematical foundations (`cycle-log/tranche-0/m0.4/plan.md`)

**Milestone intent:** the small, boring substrate every later tranche reaches
for — vector/grid primitives, a numeric type decision, a fixed-timestep
harness — got right once, here, rather than reinvented per-tranche.

**Why this round exists:** flagged since M0.1 (see M0.1's closeout, and the
tranche plan §2): M0.1's hello-world drives its animation with an inline,
ad-hoc `TICK: Cell<u32>` counter incremented directly by `tick_and_draw`, on
a bare JS `setInterval`. M0.4's target ("the fixed-timestep loop is
exercised by the M0.1 hello-world so it isn't a paper exercise nobody
actually calls") is not honestly met until that ad-hoc counter is replaced
by round 3's real `FixedTimestep` harness (`src/timestep.rs`). This is the
last round of the milestone.

**Round goals:**

1. Replace `src/lib.rs`'s `TICK: Cell<u32>` counter and its direct increment
   in `tick_and_draw` with round 3's `FixedTimestep` — JS continues to call
   into Rust once per wall-clock timer fire (or via `requestAnimationFrame`,
   implementer's call), passing a frame duration; Rust's `FixedTimestep`
   decides how many fixed steps have elapsed and drives the tick
   count/colour from that, not from a bare per-call increment.
2. Preserve the externally-observable behaviour the existing Playwright e2e
   test (`tests/e2e/canvas_rectangle.test.mjs`) checks: the rectangle
   alternates colour by tick parity on (approximately) the existing cadence.
   Update the e2e test only if the retrofit changes what it needs to poll
   (e.g. reading elapsed real time instead of a fixed interval) — but the
   test's actual guarantee (three real samples over time, not two — per
   M0.1's own carried-forward lesson) must not weaken.
3. Remove the ad-hoc `TICK` counter entirely — no dead parallel mechanism
   left behind.
4. After the retrofit: rebuild the wasm artifact
   (`bash scripts/build-wasm.sh`), re-run the Playwright e2e test, run
   `cargo test --lib` (native tests), push to `main`, and verify the live
   GitHub Pages deploy (`https://dontpanic345.github.io/silver-system/`)
   still loads and animates correctly afterward — checked directly (e.g.
   `curl` for the page content plus, if feasible, a Playwright check against
   the live URL, or at minimum polling the GitHub Actions run to completion
   and fetching the live page's HTML/wasm to confirm they match what was
   pushed), not assumed from CI status alone.

**Rounds 1-3 carried forward:** `src/math.rs` (`Scalar`, `Vec2`, `GridIndex`)
and `src/timestep.rs` (`FixedTimestep`, accumulator with a documented,
tested spiral-of-death guard) both exist, tested, and advanced cleanly (31/31
tests passing, working tree clean as of round 3's close). This round is the
first to actually call `FixedTimestep` from real (non-test) code.

**Push on vs. patch back:** pushing on — no carried-forward gaps from
rounds 1-3 block this round. This IS the retrofit round itself, the last
piece of the milestone's target.

**Refactor scope/focus:** wider scope this round — the whole crate
(`src/lib.rs`, `src/timestep.rs`, `www/index.html`), since this round
touches previously-shipped M0.1 surface, not just new material. Focus:
regression — does the retrofit genuinely preserve the animated behaviour
the existing e2e test guards, is the ad-hoc tick counter fully gone (not
left as dead code alongside the new harness), and does the wasm build/live
Pages deploy actually still work (this is the one point in the milestone
where "exercised by real code" has external, user-visible consequences if
gotten wrong).

## Round 04 — Red — 2026-09-05T03:40:27+12:00 → 2026-09-05T03:44:02+12:00 (4 min)

**What I did.**

Retrofitted `src/lib.rs`'s tick mechanism onto round 3's `FixedTimestep`, and
wrote the Rust-level test surface for it (no browser/DOM involved), per the
round's goals.

- Removed the ad-hoc `TICK: Cell<u32> += 1`-per-call mechanism entirely.
  `TICK: Cell<u32>` still exists as the storage for the tick count, but it is
  no longer incremented directly — it is now only ever advanced by whatever
  step count `FixedTimestep::advance` reports (0, 1, or more per call).
- Added a new `thread_local!` `TIMESTEP: RefCell<FixedTimestep>`, built with
  `dt = DT_SECONDS` (a new `pub const DT_SECONDS: Scalar = TICK_INTERVAL_MS as
  Scalar / 1000.0`), so the harness's fixed-step cadence matches the
  pre-retrofit animation cadence.
- Added `fn advance_tick(frame_duration_secs: Scalar) -> u32` (private, no
  `web-sys`/DOM dependency): feeds a duration to `TIMESTEP.advance(...)` and
  is expected to advance `TICK` by the returned step count and return the new
  tick count. Left as a `todo!()` stub for Green — this is the round's actual
  new decision logic.
- Changed `tick_and_draw`'s signature from `tick_and_draw(canvas_id: &str) ->
  u32` to `tick_and_draw(canvas_id: &str, frame_duration_secs: f32) -> u32`.
  Implemented for real (not stubbed) — it is thin plumbing wiring
  `advance_tick`'s result into the existing `paint_rect`/`color_for_tick`
  calls, the same "plumbing vs. decision" split round 2 already used for
  `draw`.
- Updated `www/index.html`: the `setInterval` callback now measures real
  elapsed time via `performance.now()` deltas (`lastFireMs`/`nowMs`) and
  passes `frame_duration_secs = (nowMs - lastFireMs) / 1000` to
  `wasm.tick_and_draw('canvas', frameDurationSecs)`, instead of calling it
  with no argument. This is an honest, variable duration (setInterval fires
  are not perfectly regular), not a value hardcoded to always equal exactly
  one `dt`.
- `tests/e2e/canvas_rectangle.test.mjs`: **no change needed.** It never calls
  `wasm.tick_and_draw` itself — only `www/index.html`'s `setInterval`
  callback does. The test only reads `wasm.tick_interval_ms()` and polls
  `window.__tickCount`, both interfaces unchanged by the retrofit. Its
  existing 3-sample-over-real-time structure (tick 0 → wait for tick ≥1 →
  wait for tick ≥2) is preserved verbatim and still exercises the real
  `setInterval` loop end to end.
- Added four new Rust-level tests in `src/lib.rs`'s `tests` module (see
  below), all currently red against the `advance_tick` stub, plus confirmed
  the pre-existing 32 tests are still green.

**Successes.**

`cargo test --lib` shows exactly the intended signature: 32 passed, 3 failed,
each failure a panic from the named `todo!()` in `advance_tick`, not a
compile error or a stray typo. The crate still compiles cleanly under `cargo
build` (default target) with only the expected new dead-code warnings on
`FixedTimestep`'s methods (unused because `advance_tick`'s body is currently
`todo!()` and never calls them — these clear once Green fills the stub in).

**What was difficult, and where the time went.**

Straightforward retrofit; most of the time went into reading the four
context files in full and deciding the plumbing/decision split (what to
implement for real vs. leave as a stub) consistently with round 2's existing
convention, rather than any actual difficulty. One transient hiccup: a stray
`cargo build` (default target, no `--bin` filter) once reported an unrelated
`E0433 cannot find crate image` error from `src/bin/native_viewer.rs`; it did
not reproduce on a second run (`cargo build --bin native_viewer` and then
plain `cargo build` both succeeded cleanly), and `git stash` confirmed it is
unrelated to anything in this round's diff — noted under Gaps, not
investigated further since it's out of this round's scope and not
reproducible.

**Compromises I made.**

`advance_tick`'s body is a single `todo!()` rather than partially wired
(e.g. pre-calling `TIMESTEP.with(...)` and leaving only the `TICK` update as
`todo!()`) — kept it shape-only per Red's charter ("no arithmetic, no logic,
no partial implementation") even though calling `FixedTimestep::advance`
itself is arguably just wiring. Green has full latitude on how it structures
the body, as long as the tests pass.

**Gaps and flags.**

- The transient `image` crate build error described above: not reproduced on
  retry, `git stash` confirmed it predates/is unrelated to this round's
  changes. Flagging in case it recurs for Green/Refactor — if it does, it is
  pre-existing and out of this round's scope (`src/bin/native_viewer.rs` /
  `Cargo.toml`, neither touched here).
- Per this round's explicit instructions, I did **not** rebuild the wasm
  artifact, run Playwright, or push to remote — that is goal 4, explicitly
  Green/Refactor's job later in this round. `cargo build --target
  wasm32-unknown-unknown` was run once during investigation (a plain compile
  check, not a `wasm-bindgen`/artifact rebuild) and succeeded; I did not
  repeat it.
- `FixedTimestep`'s public API (`new`, `with_max_steps_per_call`, `advance`,
  `step_with`, `dt`, `accumulator`, `max_steps_per_call`) was sufficient for
  this retrofit as-is — no gap to report there, and `src/timestep.rs` itself
  was not touched.

**General comments.**

The skeleton is minimal and single-point: Green has exactly one function
(`advance_tick`) to fill in, with everything else (constants, thread-locals,
`tick_and_draw`'s wiring, the JS side) already real and in place. The three
new scenario tests plus the one green-guard test give Green (and Refactor)
concrete, runnable pass/fail checks with no browser involved, mirroring
`src/timestep.rs`'s own test style.

---

**Test file:** `src/lib.rs` (`#[cfg(test)] mod tests`, unchanged path).

**Currently-failing tests** (all in `src/lib.rs`'s `tests` module), each a
panic from `advance_tick`'s named `todo!()` stub:

- `tests::advance_tick_with_duration_under_one_dt_does_not_advance_the_tick`
- `tests::advance_tick_with_several_dts_advances_by_that_many_steps_and_colour_matches_parity`
- `tests::advance_tick_accumulates_partial_durations_across_calls_until_a_full_dt_elapses`

All three are **durable scenarios** — each restates a clause of round 4 goal
1 verbatim (duration-under-dt doesn't step; several-dts-in-one-call advances
by that many and colour matches parity; partial durations across multiple
calls accumulate correctly — the exact "0, 1, or occasionally more than 1
step per call" pattern `www/index.html`'s `setInterval` produces).

Also added, already green (**green guard test**, pins a relationship so a
later round can't quietly break it): `tests::dt_seconds_matches_tick_interval_ms_converted_to_seconds`.

**Skeleton for Green to fill:**

- `fn advance_tick(_frame_duration_secs: Scalar) -> u32` in `src/lib.rs`
  (private, no `#[wasm_bindgen]`, no DOM dependency) — body is
  `todo!("feed frame_duration_secs to TIMESTEP.advance(...) to get a step
  count, advance TICK by that many steps (not by a bare +1), and return the
  new tick count")`. Everything else Green needs already exists and is real:
  `TICK: Cell<u32>`, `TIMESTEP: RefCell<FixedTimestep>` (both `thread_local!`),
  `DT_SECONDS: Scalar` const, and `tick_and_draw` which already calls
  `advance_tick` and repaints.

**Commands:**

- This round's/module's tests: `cargo test --lib tests::` (matches only
  `src/lib.rs`'s `tests` module, e.g. the 4 new tests plus the pre-existing
  `color_for_tick`/`render_frame`/getters/canvas-fit ones — excludes
  `math::tests::*` and `timestep::tests::*`).
- Just the new retrofit tests: `cargo test --lib advance_tick`.
- Full native test suite: `cargo test --lib` (35 tests total: 32 passing + 3
  new, currently red).
- Not run this phase (explicitly deferred to Green/Refactor per the round's
  instructions): `bash scripts/build-wasm.sh`, the Playwright e2e test
  (`NODE_PATH=/usr/local/lib/node_modules node tests/e2e/canvas_rectangle.test.mjs`),
  and anything touching the live GitHub Pages deploy.

**Did I restructure the slice?** No. The four pre-baked goals held up as
given; goal 4 (rebuild/deploy/verify) was already explicitly out of Red's
scope per the round dispatch instructions, so nothing needed splitting or
resequencing.

**Durable vs. disposable:** the three new `advance_tick` tests are durable
scenarios (see above). `dt_seconds_matches_tick_interval_ms_converted_to_seconds`
is a green-guard pin, not a scenario restating a goal, but also durable in
the sense it shouldn't be deleted without cause. No new disposable unit tests
were added this round; all pre-existing disposable unit tests
(`rectangle_fits_within_canvas`, `render_frame_paints_rect_in_tick_colour`,
`color_for_tick_alternates_by_parity`) are untouched and still green.

**Rest of the suite:** confirmed green — `cargo test --lib` reports 32 passed
(all of `math::tests::*`, `timestep::tests::*`, and every pre-existing
`tests::*` test), 3 failed (the new ones, for the intended reason), 0 other
failures.
