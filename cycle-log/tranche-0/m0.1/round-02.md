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
