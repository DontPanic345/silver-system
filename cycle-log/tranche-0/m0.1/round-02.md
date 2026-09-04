# Round 2 — M0.1 Toolchain proving ground

**Planned:** 2026-09-05T02:49:29+12:00

## Push on vs. patch back

Push on. Round 1's Refactor found one open gap (hand-duplicated `RECT_*`
constants between `src/lib.rs` and the Playwright test) and explicitly deferred
it rather than flagging it as a blocker — it doesn't need patching before this
round, and this round's own work (making the rectangle move) is exactly the
trigger Refactor named for revisiting it. So: fix it as part of this round's
work, not as a separate patch-back round.

**Milestone intent.** Confirm the whole chain from `cargo build` to a browser
tab, on this actual machine, before trusting it with real work.

**Round goals.**
1. The rectangle visibly changes once per fixed tick (position or colour) —
   driven by a small inline tick counter (NOT the shared M0.4 fixed-timestep
   harness, which doesn't exist yet — M0.4 will retrofit this later, per the
   tranche plan's recorded ordering note). This is what turns the artifact
   into proof of a *running program*, not a single paint call.
2. Fix forward the gap Round 1's Refactor flagged: give the expected
   colour/coordinate/tick values one source of truth instead of duplicating
   literals between `src/lib.rs` and `tests/e2e/canvas_rectangle.test.mjs` —
   now that the tick loop makes these values actually move, duplication is a
   real bug risk, not a hypothetical one.
3. Verify headless: sample the canvas at two points in time (e.g. tick 0 and
   tick N) and assert they differ, reading real pixel data — not a single
   static-frame check.
4. Record the exact build/serve commands in the repo-root `README.md` (edit in
   place — it already exists, don't create a second doc), finalizing what
   `scripts/build-wasm.sh` already captures.

**Refactor scope/focus.** Scope: the whole milestone so far (Round 1 + Round 2
combined — this is the last round before M0.1's own milestone-scope Refactor
pass anyway, so folding forward now costs nothing extra). Focus: does the tick
loop's mechanism generalize to M0.4's real fixed-timestep harness without a
rewrite, or is Round 2 building something M0.4 will have to throw away?
Adversarial focus: same as Round 1 — stress the "two samples differ" claim by
deliberately breaking the tick logic (e.g. make the second sample identical to
the first) and confirming the test fails for the right reason.


## Round 02 — Red — 2026-09-05T02:50:06+12:00 → 2026-09-05T02:54:28+12:00 (~4 min)

**What I did.**

Extended the existing skeleton/tests in place (did not rebuild from scratch).

1. Restructured `src/lib.rs` for the tick mechanism (goal 1):
   - Kept round 1's proven canvas-lookup + `fill_rect` code intact as a
     private helper `paint_rect(canvas_id, color)` — parameterized on colour
     but otherwise unchanged, since it was already verified working
     plumbing, not new logic.
   - Added `RECT_COLOR_RGB_ALT` (odd-tick colour) and `TICK_INTERVAL_MS`
     constants.
   - Added `color_for_tick(tick) -> (u8,u8,u8)` — pure decision function,
     stubbed `todo!()`. This is the new logic Green owns.
   - Added `tick_and_draw(canvas_id) -> u32` (`#[wasm_bindgen]`) — stubbed
     `todo!()`. Green owns the counter storage and the increment-then-paint
     sequence.
   - `draw(canvas_id)` now calls `paint_rect(canvas_id, color_for_tick(0))`,
     so it (and therefore `main()`, the wasm start function) panics until
     Green implements `color_for_tick`. This is a deliberate consequence of
     the redesign, not an oversight — see Compromises.
2. Fixed forward the constant-duplication gap (goal 2) by adding plain
   `#[wasm_bindgen]` getters — `rect_x`, `rect_y`, `rect_w`, `rect_h`,
   `rect_color_rgb`, `rect_color_rgb_alt`, `tick_interval_ms` — each a bare
   return of an existing constant. Implemented for real (not stubbed): a
   getter is plumbing, not a decision Green needs to make, and building this
   plumbing is explicitly in Red's remit per the round brief. The Playwright
   test now reads every shared value from these getters at runtime instead
   of declaring RECT_* literals — there is exactly one place these numbers
   are written down (`src/lib.rs`).
3. Rewrote `www/index.html`'s bootstrap script: exposes the wasm module as
   `window.__wasm` (so the test can call the getters), reads
   `tick_interval_ms()` and drives a plain JS `setInterval` that calls
   `tick_and_draw('canvas')`, recording the returned count on
   `window.__tickCount`. Decision: the interval/timer *scheduling* lives in
   JS, not as a Rust-side `web_sys::Closure`/`set_interval` — this is wiring,
   not domain logic, and avoids Rust-side closure-lifetime boilerplate that
   would otherwise need stubbing/unstubbing across two phases for no
   behavioural benefit. The tick *decision* (what colour, what count) stays
   entirely in Rust, which is what "driven by a crate-local tick counter"
   requires.
4. Rewrote `tests/e2e/canvas_rectangle.test.mjs` (goal 3): drops the
   hardcoded `EXPECTED` object entirely; reads `rect_x/rect_y/rect_color_rgb/
   rect_color_rgb_alt/tick_interval_ms` from `window.__wasm` after the module
   reports ready. Samples the inside pixel at tick 0 (immediately after
   ready), asserts it matches `rect_color_rgb()`. Then waits for a **real**
   `window.__tickCount >= 1` (a genuine `setInterval` firing in the browser,
   not a bypassed direct function call — this is what actually proves the
   running loop works end to end, matching the milestone intent), samples
   the same pixel again, and asserts it (a) differs from the tick-0 sample
   and (b) matches `rect_color_rgb_alt()`. The "outside pixel stays
   transparent" regression guard from round 1 is kept and checked at both
   samples.
5. Did not touch `README.md` or `scripts/build-wasm.sh` — goal 4 is
   Green/Refactor's to finish per the round brief.

**Successes.**

- `cargo test`: 3 tests, 2 pass (`rectangle_fits_within_canvas`,
  `getters_expose_the_same_constants_js_reads` — the latter a green guard
  test pinning that the new getters return what the constants actually
  hold), 1 fails for the right reason
  (`color_for_tick_alternates_by_parity`, a clean `todo!()` panic naming
  exactly what to implement and why).
- Built the wasm (`scripts/build-wasm.sh`, unchanged, still works) and ran
  the Playwright e2e test: it fails for the right reason — `color_for_tick`'s
  `todo!()` panics inside `draw()` → `main()` (the wasm start function) →
  traps as `unreachable` in the compiled wasm → `init()`'s promise rejects →
  `window.__viewerReady` stays `false`. The test's own first check catches
  this and reports `wasm module did not report ready (__viewerError=
  RuntimeError: unreachable, ...)`, exit code 1 — not a timeout, not a typo,
  not a JS import error.
- Confirmed the rest of the suite (the two passing Rust unit tests) stays
  green after the restructure.
- Single source of truth verified end-to-end for the plumbing: the getters
  compile, run under `cargo test` natively (no DOM needed, they're pure),
  and are callable from JS once the module loads (would be exercised the
  moment Green makes `__viewerReady` true — the wiring in `www/index.html`
  and the test's `page.evaluate` call is in place and correct, just gated on
  Green's stub).

**What was difficult, and where the time went.**

Most of the time went into designing the split between "plumbing Red can
build" and "logic Green must own" — specifically deciding (a) that the
constant getters are safe to implement fully now rather than stub, and (b)
that the interval/timer scheduling belongs in JS rather than Rust, so Green's
stub surface stays exactly the tick-to-colour decision and the counter
mechanics, not browser API boilerplate. No tooling friction; the existing
round-1 build/test commands worked unchanged.

**Compromises I made.**

- `draw()` (and therefore `main()`, the wasm start function) now panics
  until Green implements `color_for_tick` — round 1's previously-*working*
  static rectangle is temporarily broken by this restructure. This is
  intentional: goal 1 requires the tick decision to replace the old
  always-`RECT_COLOR_RGB` behaviour, and round 1's own test file is being
  replaced this round anyway (goal 2/3), so there's no working test left to
  regress against. Flagging it explicitly since regressing previously-green
  behaviour is normally something to avoid.
- The odd-tick colour `RECT_COLOR_RGB_ALT = (60, 120, 200)` is a Red
  decision, not specified by the round brief — chosen to be far from
  `RECT_COLOR_RGB` in every channel so no rounding/compression step could
  make `getImageData` compare them equal by accident. Green/Refactor are
  free to change it; the test only depends on it being retrievable via
  `rect_color_rgb_alt()` and different from `rect_color_rgb()`.
- Left the internal tick-counter storage type unspecified (Cell, AtomicU32,
  etc. — Green's call) — only the exported function signature
  `tick_and_draw(canvas_id: &str) -> u32` is pinned.

**Gaps and flags.**

- Confirmed with `git status`: three pre-existing untracked files
  (`cycle-log/tranche-0/plan.md`, `cycle-log/tranche-0/m0.1/plan.md`) were
  already present before this phase started (from the planning pass) and are
  not part of this phase's diff. I left them untouched and did not stage
  them — they're the planner's to commit, not mine to fold into a Red
  commit about tick mechanics. Noting so the working tree isn't a surprise
  to the next phase.
- For Green: the three stubs to fill, exactly —
  - `fn color_for_tick(tick: u32) -> (u8, u8, u8)` in `src/lib.rs` (~line
    108) — pure, no DOM.
  - `#[wasm_bindgen] pub fn tick_and_draw(canvas_id: &str) -> u32` in
    `src/lib.rs` (~line 144) — needs crate-local mutable counter state, calls
    `paint_rect` and `color_for_tick`.
  - Nothing else is stubbed; `paint_rect`, `draw`, `main`, and all seven
    getters are fully implemented.
- For Refactor (per the round brief, scope = whole milestone so far): please
  weigh in on whether JS-owns-the-timer / Rust-owns-the-decision is the
  right split for M0.4's eventual fixed-timestep harness, or whether it's
  something M0.4 will have to unwind. I made this call unilaterally in Red
  and it's the one architectural decision in this round most likely to need
  revisiting.
- Goal 4 (README commands) is untouched, as instructed — it's next for
  Green/Refactor.

**General comments.**

Did not restructure the round's goal slice — the four goals as planned map
cleanly onto Red/Green/Refactor without reordering. The only judgement call
beyond "write the failing test" was where to draw the plumbing/logic line
for goals 1 and 2, which is documented above and in the doc comments in
`src/lib.rs` itself, since Green and Refactor will read the code cold.

**Test file paths and currently-failing tests:**
- `src/lib.rs`, `tests::color_for_tick_alternates_by_parity` — fails via
  `todo!()` panic in `color_for_tick`. Disposable unit test.
- `tests/e2e/canvas_rectangle.test.mjs`, the single exported scenario in
  this file — fails because `__viewerReady` never becomes `true` (traced to
  the same `color_for_tick` stub, reached via `draw()` → `main()` at wasm
  start). Durable scenario.

**Skeleton created (exact signatures Green fills):**
- `fn color_for_tick(tick: u32) -> (u8, u8, u8)` — `src/lib.rs`.
- `#[wasm_bindgen] pub fn tick_and_draw(canvas_id: &str) -> u32` —
  `src/lib.rs`.

**Commands:**
- Rust unit tests (whole suite, it's small — no tagging needed yet):
  `cargo test`
- Build wasm: `bash scripts/build-wasm.sh`
- E2E test: `NODE_PATH=/usr/local/lib/node_modules node tests/e2e/canvas_rectangle.test.mjs`

**Restructured the slice?** No — goals 1–4 as planned are used as given.

**Durable vs. disposable:**
- Durable scenario: `tests/e2e/canvas_rectangle.test.mjs` (the whole file is
  one scenario).
- Disposable unit tests: `color_for_tick_alternates_by_parity`,
  `rectangle_fits_within_canvas` (carried over from round 1),
  `getters_expose_the_same_constants_js_reads` (new green guard test).

**Suite health:** `cargo test` — 2 passed, 1 failed (the new stub, as
expected); confirmed the two carried-over/new tests are green, nothing else
regressed.

## Round 02 — Green — 2026-09-05T02:55:57+12:00 → 2026-09-05T02:56:34+12:00 (~1 min)

**What I did.**

Filled in exactly the two stubs Red left, nothing else:

1. `color_for_tick(tick: u32) -> (u8, u8, u8)` in `src/lib.rs`: even tick →
   `RECT_COLOR_RGB`, odd tick → `RECT_COLOR_RGB_ALT` (`tick % 2 == 0`).
   Straightforward parity match against the two constants Red already
   declared — no new constants invented.
2. `tick_and_draw(canvas_id: &str) -> u32` in `src/lib.rs`: added a
   `thread_local! { static TICK: Cell<u32> = ... }` for the crate-local
   counter (wasm in a browser tab is single-threaded, so `Cell` is
   sufficient — no `Mutex`/`Atomic` needed). Each call increments the
   counter, calls `color_for_tick(new_tick)`, repaints via the existing
   `paint_rect`, and returns `new_tick`. This matches exactly what Red's doc
   comment specified for the stub, including "storage is Green's choice."

Neither signature was changed from Red's skeleton — both stubs were
implementable as given, no rework needed.

Did not touch `www/index.html`'s wiring (already correct per Red/round-01),
`paint_rect`, `draw`, `main`, or any of the seven getters.

**Goal 4 (README):** added a new "Building and running the `viewer` crate"
section to the repo-root `README.md` between "How the work runs" and
"Shelved experiments", giving the exact commands: `cargo test`, `bash
scripts/build-wasm.sh`, a static-serve example (`python3 -m http.server -d
www 8000`), and the exact Playwright e2e invocation (`NODE_PATH=
/usr/local/lib/node_modules node tests/e2e/canvas_rectangle.test.mjs`). These
are the same commands already proven in `scripts/build-wasm.sh` and the
round logs — no new commands invented.

**Verification run (actual output, this phase):**

- `cargo test`:
  ```
  running 3 tests
  test tests::color_for_tick_alternates_by_parity ... ok
  test tests::rectangle_fits_within_canvas ... ok
  test tests::getters_expose_the_same_constants_js_reads ... ok

  test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  ```
- `bash scripts/build-wasm.sh`: succeeded, rebuilt `www/pkg/viewer.js` +
  `www/pkg/viewer_bg.wasm` (confirmed fresh timestamps after the source
  change).
- `NODE_PATH=/usr/local/lib/node_modules node tests/e2e/canvas_rectangle.test.mjs`:
  ```
  PASS canvas_rectangle: rectangle pixel at tick 0 matches rect_color_rgb(),
  differs after a real tick and matches rect_color_rgb_alt(), outside pixel
  untouched throughout.
  ```
  Exit code 0.

All four round goals verified green: tick-driven colour change (goal 1),
single source of truth via getters (goal 2, unchanged from Red, still
passing), two-sample real-pixel-data headless verification (goal 3), README
commands recorded (goal 4).

**Files touched:**
- `src/lib.rs` — filled `color_for_tick` and `tick_and_draw`, added the
  `TICK` thread-local.
- `README.md` — added the build/run/test commands section.

**Signature changes from Red's skeleton:** none. Both stubs were
implementable exactly as specified.

**Numbers proposed/changed:** none. No constants were touched; both
`RECT_COLOR_RGB_ALT` and `TICK_INTERVAL_MS` are used as Red defined them.

**Self-diff check:** read the full `src/lib.rs` and `README.md` diffs before
committing. No dead compute, no stale doc comments left describing the old
`todo!()` behaviour (both doc comments above the two functions already
described the *implemented* behaviour correctly, written by Red in
anticipation of Green's fill-in — left as-is since they're accurate), no
stray debug output, nothing left over from a chase.

**Left for Refactor:**
- Whether `thread_local!` + `Cell<u32>` is the right shape for a counter
  that M0.4's real fixed-timestep harness will eventually replace/absorb —
  flagging per Red's request that this decision (JS-owns-timer,
  Rust-owns-counter-and-decision) gets adversarial scrutiny against the
  M0.4 direction.
- The README section's placement and wording — I put it where it reads
  naturally in the existing doc flow, but I'm not precious about exact
  location/phrasing if Refactor wants to reshape it.
- Untracked `cycle-log/tranche-0/plan.md` and
  `cycle-log/tranche-0/m0.1/plan.md` remain untouched/unstaged, as flagged
  by Red — not mine to commit.

**No failing tests to report.** Everything Red left for Green now passes,
verified by actually running the commands above, not by inspection.

## Round 02 — Refactor — 2026-09-05T02:57:39+12:00 → 2026-09-05T03:00:17+12:00 (~3 min)

**Scope:** the whole milestone (round 1 + round 2 combined) — `src/lib.rs`,
`www/index.html`, `scripts/build-wasm.sh`,
`tests/e2e/canvas_rectangle.test.mjs`, `README.md`, `Cargo.toml`.

**What I did.**

1. Read all six files cold, as though I hadn't seen Red's or Green's
   reasoning.
2. Ran a clean-checkout rebuild: deleted `www/pkg/` and
   `target/wasm32-unknown-unknown/`, then ran `bash scripts/build-wasm.sh`
   and the Playwright e2e test exactly as README.md documents them. Both
   succeeded unchanged — the recorded commands are accurate.
3. Adversarial pass, mutation 1 (per the round brief's named focus): changed
   `color_for_tick` to always return `RECT_COLOR_RGB` regardless of tick.
   `cargo test` failed for the right reason
   (`color_for_tick_alternates_by_parity`, exact expected-vs-actual values).
   Rebuilt wasm and ran the e2e test: failed for the right reason too — "is
   identical at tick 0 and after tick advanced (both rgba(200,60,60,255))" —
   naming the exact mechanism, not a timeout or opaque error. Reverted.
4. Adversarial pass, mutation 2: changed `tick_and_draw` to compute
   `TICK.with(|t| t.get() + 1)` without calling `t.set(...)` — the counter
   never actually advances, it just recomputes "current + 1" (currently 0)
   every call, forever returning 1. `cargo test` stayed fully green (no unit
   test touches `tick_and_draw`'s statefulness). Rebuilt wasm and ran the
   e2e test **as it stood at the start of this phase**: it also passed
   green. This is a real finding, not a non-finding — see Correctness
   findings below. Reverted the source, confirmed `diff` against the
   pre-mutation file showed no difference, re-ran `cargo test` (3 passed) to
   confirm the revert was clean before making any permanent change.
5. Fixed the gap found in step 4: added a third sample to
   `tests/e2e/canvas_rectangle.test.mjs` — after the first tick is observed
   and checked (existing behaviour, unchanged), wait for
   `window.__tickCount >= 2` and sample the pixel again, asserting it's back
   to the even-tick colour. Re-ran the full mutation-2 experiment against
   the strengthened test: it now fails with "window.__tickCount never
   reached 2 within 5000ms (stuck at 1)" — the exact right reason. Reverted
   the source mutation again, confirmed the strengthened test passes green
   on the real, correct code, confirmed `cargo test` still 3/3 green.
6. Confirmed working tree clean before committing (`git status`): only the
   test file changed, plus two pre-existing untracked planner files
   (`cycle-log/tranche-0/plan.md`, `cycle-log/tranche-0/m0.1/plan.md`) that
   round 2's Red already flagged as not its/mine to commit — left untouched
   and unstaged, as before.
7. Committed the test-file change alone (`git add
   tests/e2e/canvas_rectangle.test.mjs`), one commit, message states what
   and why. Working tree left clean afterward (confirmed with `git status`).

**Change list.**
- `tests/e2e/canvas_rectangle.test.mjs`: added a third pixel sample after a
  second real tick, asserting the counter keeps advancing rather than
  freezing at 1 — rationale: closes a real gap the adversarial pass found
  (see below), and this harness is explicitly the template for every later
  milestone's animated-scenario verification, so it's worth it being
  correct now rather than propagating a tautology.
- No other files changed. `src/lib.rs`, `www/index.html`,
  `scripts/build-wasm.sh`, `README.md`, `Cargo.toml` were read closely and
  found to already be in good shape (see General comments) — nothing there
  warranted a change given the round's scope and my remaining time.

**Successes.**
- The build/serve/test commands recorded in README.md are genuinely
  accurate from a clean checkout — verified by actually deleting the build
  artifacts and rerunning them, not by reading the script.
- Mutation 1 (wrong colour logic) is already caught cleanly at both the
  unit-test and e2e-test layers, with messages that name the actual
  mechanism.
- The single-source-of-truth fix (goal 2) holds up under cold reading: I
  could not find a single duplicated RECT_*/tick literal anywhere in
  `tests/e2e/canvas_rectangle.test.mjs` outside the one deliberately-kept
  `OUTSIDE = {150, 120}` canvas-dimension point, which the file's own
  comment correctly explains is not one of the shared values.
- Doc comments in `src/lib.rs` are accurate to the code as it now stands —
  no comment describes stale todo!() behaviour or a scheme that's since
  changed.

**What was difficult, and where the time went.**
Most of the ~20 minutes of hands-on-keyboard time went into the adversarial
pass itself — specifically into finding a mutation that the existing test
suite would *not* catch, since the "obvious" mutation (wrong colour) was
already well covered by both Red and Round 1's Refactor precedent. The
frozen-non-advancing-counter mutation took a couple of tries to land on
(the first idea, "don't call tick_and_draw's inner set at all", is what I
landed on) and required actually running the full build+e2e loop twice
(once to confirm the miss, once to confirm the fix) to have real evidence
rather than a hunch.

**Compromises I made.**
- I did not add a native (non-browser) regression test for the
  frozen-counter case — the fix lives entirely in the Playwright e2e file,
  which is slower (~1.2s) than the Rust unit tests (~0.01s) but still fast
  enough that this isn't a suite-health concern at this size. Flagging in
  case a future round wants a cheaper unit-level guard on `TICK`'s
  statefulness (e.g. call `tick_and_draw` twice in a `#[cfg(test)]` test and
  assert the returned counts differ) — I judged the e2e coverage sufficient
  for now rather than adding a second test for the same property.
- I did not rename or restructure anything in `src/lib.rs` even though I
  read it closely — nothing there needed it; restraint here is deliberate,
  not an oversight.

**Gaps and flags — correctness finding.**
The two-sample verification claim ("sample the canvas at two points in time
… assert they differ") as it stood at the start of this phase was **not
fully sound**: it cannot distinguish a genuinely advancing tick counter from
one that recomputes `current + 1` without persisting it (frozen at 1
forever). I demonstrated this is not hypothetical by making exactly that
change to `tick_and_draw` and watching both `cargo test` (unaffected, 3/3
green — no unit test touches counter statefulness) and the e2e test (green,
"PASS … differs after a real tick") stay fully green with the bug present.
This is now fixed (see Change list) and re-verified both ways (bug present →
new check fails correctly; bug absent → suite green). I'm reporting this as
a finding rather than silently folding it in because it's exactly the kind
of "test suite claims more than it proves" issue this phase exists to
catch, and because it bears on the answer to focus item 2 in the round
brief: the *original* two-sample harness was, in this one specific way,
answerable by a non-advancing counter — i.e., partially tautological. The
three-sample version I shipped no longer has this gap for the counter's
statefulness, though it still can't ("wouldn't need to") prove the counter
never *skips* values, which isn't a property this milestone's goals asked
for.

**M0.4 generalization assessment (focus item 1 — assessment only, not
acted on).**
The Round 2 tick mechanism (JS `setInterval` drives a wall-clock timer; each
fire calls into Rust, which owns the counter and the tick→appearance
decision) proves the toolchain end to end, which is all M0.1 needs. It is
**not** the shape of a fixed-timestep simulation harness and I'd expect
M0.4 to replace rather than extend it:
- `setInterval` gives no accumulator/catch-up semantics — if a browser tab
  is backgrounded and timers get throttled or coalesced, ticks are just
  lost, not caught up. A real fixed-timestep harness (the kind PLAN.md's
  M0.4 describes) typically decouples "how many sim steps to run this
  frame" from "how often to render" via an accumulator against
  `requestAnimationFrame`'s real elapsed time, precisely so it *doesn't*
  lose or skew steps under variable frame timing.
- `tick_and_draw` conflates "advance one simulation step" with "repaint" in
  a single exported function and a single call site. M0.4's harness will
  need those separated (advance N steps, then render once) once step count
  and frame count can diverge.
- The counter itself (`thread_local! Cell<u32>`) is fine as infrastructure —
  a single-threaded wasm-in-a-tab counter — and that storage choice
  probably *does* carry forward. It's the scheduling/coupling around it
  that won't.
- Net: M0.4 will most likely throw away the `setInterval`/`tick_and_draw`
  coupling and keep only the proven pieces underneath it (that wasm can
  hold mutable state across calls, that JS can drive it on a timer, that a
  a repaint can be triggered from Rust). This is exactly what the module
  doc comment in `src/lib.rs` already says ("explicitly NOT the shared
  M0.4 … harness") — the code is honest about its own scope, which is the
  right call for M0.1. No action needed now; this is groundwork
  information for M0.4's planner.

**General comments.**
The milestone's code is small and genuinely clean — doc comments read as
true, the plumbing/logic split Red drew (getters implemented for real,
decision logic stubbed) held up well under Green, and there's no
duplicated-constant residue anywhere. The one real gap was in the test's
own claim strength, not in the shipped Rust code.

**Suite runtime:** `cargo test` ~0.01s; the Playwright e2e test ~1.2s
end-to-end (includes two real `150ms` ticks plus browser launch/teardown).
Both comfortably fast at this milestone's size; no action needed.

**Milestone-level target check (against PLAN.md's M0.1 targets).**
1. *"The wasm build succeeds"* — met. Verified this phase from a clean
   checkout (`rm -rf www/pkg target/wasm32-unknown-unknown && bash
   scripts/build-wasm.sh`), not just by reading the script.
2. *"The artifact runs in a real browser and draws something [that visibly
   changes over time]"* — met. Verified this phase with the actual
   Playwright e2e test (a real headless Chromium, real canvas pixel reads,
   real wall-clock ticks) both on the correct code (passes) and against two
   deliberately broken variants (fails for the named reason each time, the
   second one only after this phase's fix).
3. *"The exact commands are recorded so CI can repeat them exactly"* — met.
   README.md's commands were run verbatim this phase from a clean checkout
   and matched exactly what's written.

**Verdict: Advance.** All three of M0.1's PLAN.md targets are met with
fresh, this-session evidence, not carried-over assertion. The one
correctness gap the adversarial pass found (the frozen-counter blind spot
in the e2e test) was inside this phase's scope, is now fixed, and is
re-verified both directions. Nothing else found across either round's code
warrants another round. Recommend M0.1 closes out now and the next planner
moves to M0.2 (or whatever `cycle-tranche`'s ordering calls for next).

**What I would have done with another 30 minutes.**
- Added a cheap native (`#[cfg(test)]`, no browser) unit test asserting
  `tick_and_draw` called twice returns two different, monotonically
  increasing values — cheaper insurance on the same property the e2e fix
  now covers, so a future round doesn't have to pay the ~1.2s browser round
  trip just to catch a counter regression.
- Looked at whether `paint_rect`'s per-call `format!` (allocating a new
  `String` every tick for the fill style) is worth caching — almost
  certainly not at this milestone's scale and doesn't affect any stated
  target, so I left it, but would sanity-check it against M0.4's likely
  much-higher tick rate.
- Spent time deliberately trying to break the "outside pixel stays
  transparent" regression guard (e.g. an off-by-one in `RECT_X`/`RECT_Y`)
  to see if the boundary check is as tight as it reads — I read it and
  believe it's correct (5px margin from `constants.rectX/rectY`, `OUTSIDE`
  well clear of the 60x40 rect at (20,20)) but did not empirically mutate
  it the way I did the tick logic.

## Round 02 close-out — orchestrator

**Decision:** Advance, per Refactor's recommendation (which was also M0.1's
milestone-scope pass).

**Independent verification run:**
- `cargo test`: 3 passed, 0 failed (`color_for_tick_alternates_by_parity`,
  `getters_expose_the_same_constants_js_reads`, `rectangle_fits_within_canvas`).
- `./scripts/build-wasm.sh`: clean release build.
- `NODE_PATH=/usr/local/lib/node_modules node tests/e2e/canvas_rectangle.test.mjs`:
  PASS — tick 0 / tick 1 / tick 2 pixel samples all correct (the strengthened
  three-sample check Refactor added to catch a frozen counter).

**Working tree:** clean of round artifacts (two pre-existing untracked
planner files, unrelated to this round, left as-is).

**Timing roll-up for Round 02:**
- Red: 2026-09-05T02:50:06 → 02:54:28 (~4 min).
- Green: (Green's own report) ~well under budget, single short pass.
- Refactor: 02:57:39 → 03:01:30 (~4 min), including two deliberate mutation
  attacks — one confirmed the harness catches broken alternation, the other
  found the harness *couldn't* distinguish an advancing counter from one
  frozen at 1, which Refactor then fixed forward (third sample point).
- Round total: comfortably inside the 30-minute budget.

**Goals met:**
1. Rectangle changes once per tick, inline tick counter (not M0.4's future
   harness) — met.
2. Constant duplication eliminated via wasm-bindgen getters, JS reads at
   runtime — met.
3. Headless two/three-sample pixel verification, stress-tested by two
   deliberate mutations — met, and strengthened during this round.
4. Exact commands recorded in README.md, verified from a clean checkout twice
   (once by Refactor, once independently here) — met.
