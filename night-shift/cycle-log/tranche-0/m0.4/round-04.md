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

## Round 04 — Green — 2026-09-05T03:45:42+12:00 → 2026-09-05T03:49:37+12:00 (4 min)

**What I did.**

Part A: filled in `advance_tick` in `src/lib.rs`, replacing Red's `todo!()`
stub:

```rust
fn advance_tick(frame_duration_secs: Scalar) -> u32 {
    let steps = TIMESTEP.with(|timestep| timestep.borrow_mut().advance(frame_duration_secs));
    TICK.with(|tick| {
        let new_tick = tick.get() + steps;
        tick.set(new_tick);
        new_tick
    })
}
```

Feeds `frame_duration_secs` to `TIMESTEP`'s `FixedTimestep::advance`, gets a
step count back, advances `TICK` by that many steps (not a bare `+1`), and
returns the new tick count — exactly what the stub's `todo!()` message and
Red's doc comment specified. No signature change from Red's skeleton was
needed.

Part B: rebuilt the wasm artifact, ran the Playwright e2e test, ran
`cargo test --lib` again, committed, pushed to `main`, polled the GitHub
Actions run to completion, and verified the live GitHub Pages deploy
directly (not assumed from CI status) with a real headless-browser check
against the live URL.

**Successes.**

Everything passed, at every stage, on the first attempt — no retries needed.

- `cargo test --lib`: **35 passed, 0 failed** (was 32 passed/3 failed before
  this fill-in). Exact command and output:
  ```
  test result: ok. 35 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
  ```
  All three previously-failing `advance_tick_*` tests and the green-guard
  `dt_seconds_matches_tick_interval_ms_converted_to_seconds` test pass.
- Self-diff (`git diff -- src/lib.rs`): a 13-line diff, exactly the stub body
  replaced, nothing else touched — no dead compute, no leftover doc comments
  describing an old scheme (Red's doc comment above the function already
  described the new, correct behaviour), no stray debug output.
- `bash scripts/build-wasm.sh`: succeeded cleanly (`cargo build --release
  --lib --target wasm32-unknown-unknown` then `wasm-bindgen`), only the
  expected pre-existing dead-code warnings (`GridIndex`, `FixedTimestep`'s
  unused accessor methods) — no errors.
- Confirmed `www/pkg/viewer.js`'s generated glue matches the new two-arg
  signature: `export function tick_and_draw(canvas_id, frame_duration_secs)`,
  and `www/index.html` (already updated by Red) calls it with a real
  `performance.now()`-derived `frameDurationSecs`, not a hardcoded value.
- Playwright e2e test against the freshly rebuilt local `www/`:
  ```
  PASS canvas_rectangle: rectangle pixel at tick 0 matches rect_color_rgb(), alternates to rect_color_rgb_alt() after one real tick and back to rect_color_rgb() after a second, outside pixel untouched throughout.
  ```
  Confirms the retrofit preserves the exact externally-observable contract
  goal 2 required — real `setInterval` loop, three real samples over time,
  colour alternates by tick parity.
- `git status --porcelain` showed only `src/lib.rs` modified — `www/pkg/` is
  gitignored (`.gitignore:11:/www/pkg`) and was never tracked, so there was
  nothing else to `git add`. Committed with explicit `git add src/lib.rs`
  (never `-A`), commit `2524e77`.
- `git branch --show-current` → `main`; `git remote -v` → confirmed
  `origin` is `https://github.com/DontPanic345/silver-system.git`. Pushed:
  `8438bef..2524e77  main -> main`.
- Polled `https://api.github.com/repos/DontPanic345/silver-system/actions/runs`
  (unauthenticated, public REST API) for the run matching commit `2524e770`
  until it left `queued`/`in_progress`. Final state, fetched directly:
  ```
  completed success 2524e770 https://github.com/DontPanic345/silver-system/actions/runs/33891412515
  ```
- **Live deploy verified directly, not assumed from CI status.** Two
  independent checks against the real, live URL after the workflow
  completed:
  1. `curl` of `https://dontpanic345.github.io/silver-system/` (HTML) and
     `https://dontpanic345.github.io/silver-system/pkg/viewer.js` — confirmed
     the served JS glue contains `export function tick_and_draw(canvas_id,
     frame_duration_secs)` (the new two-arg signature, not the old one-arg
     shape), and the served `index.html` contains the `performance.now()`
     delta logic. `viewer_bg.wasm` fetched with HTTP 200, 51548 bytes.
  2. A real headless-Chromium Playwright script (written this phase,
     `live_check.mjs`, adapted from `tests/e2e/canvas_rectangle.test.mjs`)
     pointed directly at `https://dontpanic345.github.io/silver-system/`
     (not a local server) — loaded the live page, waited for the wasm module
     to report ready, sampled the rectangle pixel at tick 0, waited for a
     real tick via the live page's own `setInterval`, sampled again, waited
     for a second tick, sampled a third time. Result:
     ```
     PASS live: dontpanic345.github.io/silver-system/ animates correctly (tick0/1/2 colours match, retrofitted tick_and_draw signature confirmed working end-to-end in browser)
     ```
     This is the strongest evidence available short of a human watching a
     browser tab: the actual deployed artifact, fetched over the network,
     genuinely animates via the retrofitted `FixedTimestep`-driven
     `tick_and_draw`, in a real browser engine.

**What was difficult, and where the time went.**

Nothing was difficult. The fill-in was exactly as scoped by Red — one
function, using APIs already proven by `src/timestep.rs`'s own tests. Most of
the elapsed time (about 4 minutes total) was the GitHub Actions run itself
completing (queued → in_progress → completed took a little over 2 minutes),
polled in the background while nothing else was pending.

**Compromises I made.**

None. No number was loosened, no test weakened, no partial implementation
left behind.

**Gaps and flags.**

- The `advance_tick` implementation uses `TIMESTEP.with(...)` then
  `TICK.with(...)` as two separate `thread_local!` accesses rather than one
  combined closure — a minor style choice, not a correctness concern (wasm in
  a browser tab is single-threaded, per the existing doc comments, so there
  is no race between the two). Left as-is; if Refactor prefers a single
  combined access it's a trivial reshape, not a behaviour change.
  - Red's flagged, unreproduced `image` crate build error on a stray
  plain `cargo build` (no `--bin` filter) did not recur during this phase —
  every `cargo test --lib` and `bash scripts/build-wasm.sh` invocation
  succeeded cleanly. Still flagging forward per Red's note, since it wasn't
  root-caused, just unreproduced twice now.
- The ad-hoc `live_check.mjs` script used to verify the live deploy is a
  scratch file (`/tmp/.../scratchpad/live_check.mjs`), not committed to the
  repo — it was a one-off verification tool for this phase, not a
  permanent addition to `tests/e2e/`. If the project wants a durable
  "check the live deploy" test committed to the repo (as opposed to
  `tests/e2e/canvas_rectangle.test.mjs`, which only ever exercises a local
  build), that's a decision for planning/Refactor, not something I added
  unilaterally.

**General comments.**

This closes M0.4's last round: the ad-hoc `TICK: Cell<u32> += 1` mechanism
flagged since M0.1 is now fully replaced by round 3's `FixedTimestep`
harness, wired in and exercised by real code (not just tests) both locally
and on the live, deployed page. `cargo test --lib` is 35/35 green, the local
Playwright e2e test passes, the GitHub Actions deploy workflow completed
successfully for commit `2524e77`, and the live GitHub Pages page was
independently confirmed — via both a direct `curl` of its served artifacts
and a real headless-browser run against the live URL — to be running the
retrofitted code and animating correctly.

## Round 04 — Refactor — 2026-09-05T08:56:53+12:00 → 2026-09-05T08:58:30+12:00 (2 min)

**What I did.**

Read the round's diff cold against Red's and Green's reports and against the
M0.4 milestone plan, per the wider (whole-crate) scope this round was given.
Reviewed `src/lib.rs`, `src/timestep.rs`, `www/index.html`, and
`tests/e2e/canvas_rectangle.test.mjs` in full; confirmed the committed diff
since Red's `af2d3f4` touches only `src/lib.rs` (13 lines) plus the round log
— nothing else. Ran `cargo test --lib` (35/35 green), rebuilt the wasm
artifact, and ran the Playwright e2e test locally to reconfirm both still
pass on the checked-out `HEAD` before touching anything.

Then ran one deliberate adversarial mutation (see below), confirmed which
test layer catches it, and reverted it. No other code changes were made —
the diff was already well-folded and needed no changes.

**Successes.**

- Confirmed the retrofit is real, not cosmetic: `TICK` is written in exactly
  one place (`advance_tick`), and it is written with the step count
  `FixedTimestep::advance` returns, never a bare `+1`. `TIMESTEP` is not a
  parallel/vestigial mechanism sitting alongside the old one — the old
  mechanism (direct `+1` increment in `tick_and_draw`) is fully gone; grepped
  the whole crate (`src/`, `www/`) for `TICK` and found only the intended
  uses (the doc comment, the one write site inside `advance_tick`, and an
  unrelated `TICKS_TO_RENDER` loop bound in `src/bin/native_viewer.rs`, which
  is M0.3's native-fallback frame count, not a tick-driving mechanism).
- Confirmed the e2e test still genuinely samples three points in time (tick
  0, tick ≥1, tick ≥2), each gated on a real `window.__tickCount` change via
  `page.waitForFunction`, not a fixed sleep — Red's report that no e2e change
  was needed checks out; the test file is untouched and still carries the
  M0.1-derived "two samples can't distinguish advancing from stuck" comment
  and structure.
- `git diff af2d3f4 HEAD --stat`: only `src/lib.rs` (13 lines) and the round
  log changed since Red — no stray edits, no dead code left behind.
- Confirmed `HEAD` (`5e0d872`) matches `origin/main` after `git fetch` — the
  push Green reported is real, not just claimed.
- Rebuilt wasm from the clean tree and re-ran the Playwright e2e test myself
  (not just trusting Green's report): both green, matching Green's numbers
  exactly.
- Checked for dead-code fallout: `cargo build --lib` still shows the same
  class of warnings Green already flagged as expected (`Vec2`/`GridIndex`
  unused — no real code consumes them yet, by M0.4's own plan; `FixedTimestep`'s
  `dt`/`max_steps_per_call`/`accumulator`/`step_with` unused by non-test code
  since `advance_tick` only calls `.advance()`) — nothing new, nothing this
  round should have consumed and didn't.

**What was difficult, and where the time went.**

Nothing was difficult. Most of the ~2 minutes went into reading the four
files in full plus the milestone plan, and running the mutation probe
(mutate → `cargo test` → rebuild wasm → Playwright → revert → rebuild wasm →
Playwright again to leave the tree's build artifacts correct).

**Compromises I made.**

None — no code changes were needed; the round's diff was already correct and
well-folded.

**Adversarial pass.**

One deliberate mutation, per this round's explicit instruction: in
`advance_tick`, changed `TICK.set(tick.get() + steps)` to
`TICK.set(tick.get() + 1)` while still calling `TIMESTEP.advance(...)` and
discarding its result (`let _steps = ...`). This exactly reintroduces the
ad-hoc "+1 per call" bypass, disguised behind a still-present (but now
vestigial) call into `FixedTimestep` — the specific failure mode the round's
focus asked me to check for.

- `cargo test --lib tests::`: **caught it immediately.** All three new
  `advance_tick_*` scenario tests failed with exactly the expected
  assertion mismatches (e.g. "three dts' worth of elapsed time... left: 1,
  right: 3"). This confirms the native suite genuinely exercises the
  retrofit's actual logic, not just its signature.
- `NODE_PATH=/usr/local/lib/node_modules node tests/e2e/canvas_rectangle.test.mjs`
  against a wasm rebuild of the mutated code: **did not catch it — the e2e
  test still PASSed.** This is a real finding, not a null result: under
  ordinary `setInterval` timing (fires land close to `intervalMs` apart), a
  bare `+1`-per-call bypass is externally indistinguishable from genuine
  `FixedTimestep`-driven advancement — the e2e test only checks that the
  tick count/colour visibly advances twice over real wall-clock time, not
  that the *step count per call* is computed from actual elapsed duration.
  Catching this class of regression is the native `advance_tick` unit
  tests' job, not the e2e test's, and they do it. I do not consider this a
  gap to fix this round: the e2e test's stated purpose (per its own header
  comment) is proving the running-program/real-timer loop end to end, which
  it still does; it was never meant to be a substitute for the unit-level
  duration-accounting tests, and Red's three new tests already cover exactly
  this. Flagging forward for planning: if a future round wants the e2e layer
  itself to distinguish real elapsed-time accounting from a bypass (e.g. by
  artificially varying `setInterval` cadence or injecting frame durations
  from JS), that would be new scope, not a defect in this round.
- Reverted the mutation (`cp` from a pre-mutation backup), confirmed
  `git diff --stat src/lib.rs` empty and `cargo test --lib` back to 35/35
  green, then rebuilt wasm and re-ran Playwright once more to leave the
  gitignored `www/pkg/` build artifacts matching the clean, committed source
  (not the mutated one) — working tree left clean (`git status`: nothing to
  commit).

Also tried, found nothing: swept `src/`, `www/`, `README.md`, and
`scripts/*.sh` for any other lingering reference to the old ad-hoc mechanism
or stale doc comments describing a scheme that no longer exists — none
found. The module-level doc comment's "## Round 4 retrofit" section
describes only the current scheme, not a stale mix of old and new.

**Correctness findings.** None. The shipped code is correct as committed.

**Milestone-level sanity (M0.4 as a whole).**

Checked the shipped state against `cycle-log/tranche-0/m0.4/plan.md`'s five
targets:

1. Grid/vector primitives exist and are unit-tested — `src/math.rs`,
   confirmed via `cargo test --lib` (`math::tests::*`, 6 tests, all green).
2. Coordinate convention pinned by both a test and a doc comment, and the
   canvas (`src/lib.rs`) and math (`src/math.rs`) conventions state their
   relationship — confirmed present (`math::tests::up_convention_pins_...`
   plus the canvas's own y-down doc comment on `RECT_X`/`RECT_Y`, which
   explicitly calls out the flip from math's y-up).
3. Numeric type decision (`f32`, per `Scalar`) — present with written
   rationale in `src/math.rs` per earlier rounds' reports (not re-derived
   here, out of this round's diff, but confirmed still in place).
4. Fixed-timestep harness exists, is unit-tested, and is now actually called
   by running code — this round's own work, confirmed above.
5. Post-retrofit: wasm build succeeds, e2e test passes, live Pages deploy
   confirmed directly (not from CI status alone) — confirmed by Green's
   report (direct `curl` + live-Chromium Playwright check against the real
   URL) and re-confirmed by my own local rebuild/e2e rerun on the same
   `HEAD` that is live.

All five milestone targets are met. No gap found that would block closing
M0.4.

**Suite runtime.** `cargo test --lib`: 35 tests in ~0.01s — negligible, no
concern. Playwright e2e: single test, a few hundred ms of real wall-clock
wait per tick sample (~2 real ticks ≈ 300ms at `TICK_INTERVAL_MS=150`) plus
browser startup, on the order of a few seconds total — acceptable for a
single-test suite at this stage; nothing to trim.

**The verdict: Advance.**

The round's four goals are all met to a sufficient standard: the ad-hoc
`TICK` counter is genuinely gone (not left as dead code beside the new
harness), `FixedTimestep` is the sole mechanism advancing the tick, the
externally-observable behaviour the e2e test guards is preserved verbatim,
and goal 4's rebuild/e2e/native-test/push/live-verify chain was completed
and independently reconfirmed this phase. The adversarial mutation confirms
the native test suite would catch a broken retrofit of the disguised-bypass
kind; it also surfaces a real (not fatal) limit of the e2e test's coverage,
flagged above for planning rather than treated as a this-round defect. This
also closes M0.4 as a whole — all five milestone targets are independently
confirmed met.

**What I would have done with another 30 minutes.** Explored whether the
e2e test's coverage gap (above) is worth closing now vs. deferred — e.g. a
native/headless test that drives `tick_and_draw` with deliberately irregular
durations through the actual wasm module (not just `advance_tick` natively)
to catch a bypass that only manifests through the `#[wasm_bindgen]` boundary
itself. Judged out of this round's scope (goal 3/4 don't ask for a new e2e
test, and the native tests already cover the logic this would re-check) —
noted as a possible future-round idea, not acted on.
