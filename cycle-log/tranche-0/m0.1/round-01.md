# Round 01 — M0.1 Toolchain proving ground

## Round 01 — Red — 2026-09-05T02:35:57+12:00 → 2026-09-05T02:41:29+12:00 (~6 min)

**What I did.**

Built the Rust/wasm skeleton and a failing headless verification harness for
the round's four goals, and — since goals 1 and 2 are tooling-verification
goals rather than logic to defer to Green — actually exercised the real
toolchain chain end to end to confirm it holds on this machine, not just on
paper:

1. **Rendering approach decision (goal 1).** Chose canvas 2D via
   `wasm-bindgen`/`web-sys`, per the likely direction in the brief. Justified
   empirically, not on paper suitability: confirmed on this machine that
   `rustup target add wasm32-unknown-unknown` is already installed, that
   `cargo` can resolve and download `wasm-bindgen` + `web-sys` from crates.io,
   that a `wasm-bindgen`-using crate compiles for `wasm32-unknown-unknown`,
   and that `wasm-bindgen-cli` installs via `cargo install` and its version
   can be pinned to match the crate version resolved in `Cargo.lock`
   (0.2.127 both sides). No WebGL/WebGPU crate was trialled — canvas 2D
   worked on the first attempt with no surprises, so there was no forcing
   function to compare alternatives this round. If a later round needs more
   power than 2D canvas gives, that's a fresh decision against what builds
   then.

2. **wasm32 build (goal 2).** Created `Cargo.toml` (package `viewer`,
   `crate-type = ["cdylib", "rlib"]` — rlib added so `cargo test` also works
   on the host target for the disposable unit test) and `src/lib.rs` with:
   - `pub const RECT_COLOR_RGB/RECT_X/RECT_Y/RECT_W/RECT_H` — the pinned
     numbers Green's implementation and the JS test must agree on, plus a
     doc comment pinning the canvas coordinate convention (origin top-left,
     y grows down — opposite of math y-up) once, since this is exactly the
     kind of silent sign-convention bug the domain punishes expensively.
   - `pub fn draw(canvas_id: &str)`, `#[wasm_bindgen]`-exported, body
     `todo!("look up ... get_context(\"2d\") ... fill_rect ...")` — shape
     only, no drawing logic.
   - `#[wasm_bindgen(start)] pub fn main()` calling `draw("canvas")`, so the
     wasm module attempts the draw automatically when instantiated by the
     host page.
   - `cargo build --release --target wasm32-unknown-unknown` succeeds
     (verified — see commands below).

3. **Browser draw + headless verification (goal 3).** Built `www/index.html`
   with a `<canvas id="canvas" width="200" height="150">`, loading the
   `wasm-bindgen --target web` output as an ES module and recording
   `window.__viewerReady` / `window.__viewerError` from the `init()`
   promise so a headless test can observe success/failure without polling
   the DOM blindly. Wrote
   `tests/e2e/canvas_rectangle.test.mjs` (Node + Playwright): serves
   `www/` over a plain local HTTP server on an ephemeral port, drives
   headless Chromium, waits for the ready/error flag, then reads actual
   canvas pixel data with `getImageData` at a point inside the rectangle
   (expects `rgba(200,60,60,255)`) and a point outside it (expects alpha 0,
   i.e. untouched). This is a real pixel-data check, not a screenshot a
   human looks at, per the project's stated verification preference.

4. **Commands recorded (goal 4).** `scripts/build-wasm.sh` codifies the
   exact build + bindgen sequence for CI (M0.2) to reuse verbatim. Exact
   commands also listed below and in the script's own comments.

**Successes.**

- The entire toolchain chain works on this machine on the first attempt:
  `wasm32-unknown-unknown` target already installed via rustup;
  `cargo build --target wasm32-unknown-unknown` succeeds for a
  `wasm-bindgen`/`web-sys` crate; `cargo install wasm-bindgen-cli --version
  0.2.127` completes in ~51s and produces a CLI whose version matches the
  crate; `wasm-bindgen --target web` produces loadable JS + wasm; a plain
  static file server + headless Chromium loads the page and instantiates
  the module.
- `cargo test` (host target, not wasm32) passes one disposable unit test
  (`rectangle_fits_within_canvas`) pinning that the rectangle geometry fits
  inside the declared canvas — a cheap tripwire against a silent-clip bug
  if either side's numbers drift.
- The verification harness fails **for the right reason**: the wasm module
  loads and instantiates, but `#[wasm_bindgen(start)] fn main()` calls
  `draw()`, whose body is `todo!()`. In wasm this compiles to an
  `unreachable` trap, so `init()` in the JS glue rejects with
  `RuntimeError: unreachable`, caught by `www/index.html` and surfaced as
  `window.__viewerError`. The test correctly reports
  `wasm module did not report ready (__viewerError=RuntimeError:
  unreachable, ...)` and exits 1. This is a real panic-from-a-named-stub
  signal, not a compile error or a typo.
- Discovered and worked around a real Node/Playwright interop issue (see
  Compromises) rather than leaving the harness broken.

**What was difficult, and where the time went.**

- Almost all the time went to actually exercising the toolchain (network
  fetch from crates.io, `cargo install wasm-bindgen-cli` taking ~51s,
  running the wasm-bindgen CLI, standing up and debugging the Playwright
  harness) rather than to indecision. No blocking difficulty was hit; total
  elapsed was well under the 30-minute checkpoint (~6 minutes).
- One real surprise: `NODE_PATH=/usr/local/lib/node_modules node
  tests/e2e/canvas_rectangle.test.mjs` initially failed with
  `ERR_MODULE_NOT_FOUND: Cannot find package 'playwright'` even though
  Playwright is confirmed installed and importable under that NODE_PATH.
  Root cause: Node's ESM loader (both static `import` and dynamic
  `import()`) does not consult `NODE_PATH` at all — confirmed with a
  minimal repro (`node --input-type=module -e "import('playwright')..."`
  fails, the CommonJS-style `require('playwright')` under the identical
  env succeeds). Fixed by using `createRequire(import.meta.url)` to get a
  CJS-style resolver inside the `.mjs` file, which does honour `NODE_PATH`.
  This is worth carrying forward to the README/memory: **any** future
  Playwright script in this repo needs the same `createRequire` workaround,
  not just `NODE_PATH` plus `import`.

**Compromises I made.**

- `EXPECTED.color` / sample coordinates in
  `tests/e2e/canvas_rectangle.test.mjs` are hand-duplicated against
  `RECT_COLOR_RGB` / `RECT_X..RECT_H` in `src/lib.rs`, with a comment on
  each side pointing at the other. There is no shared source of truth
  (e.g. Rust exporting the constants as JSON for the JS test to read) — for
  a four-number proving-ground round this seemed like the right amount of
  machinery, but it is a manual-sync risk Green or Refactor should notice
  if either set of numbers changes.
- I did not compare canvas 2D against a WebGL/WebGPU crate empirically —
  the round's likely-direction note said canvas 2D was probably right, and
  it worked cleanly on the first real attempt, so there was no observed
  failure to justify spending round time trialling an alternative. If a
  future milestone's rendering needs (many objects, shaders, GPU compute)
  outgrow 2D canvas, that comparison should happen then, against real
  requirements, not now against none.
- The `www/pkg/` directory (wasm-bindgen's generated JS + wasm output) is
  build output, gitignored rather than committed, since it's fully
  reproducible from `scripts/build-wasm.sh`. Green will need to re-run the
  build before running the test, same as I did.
- `wasm-bindgen-cli` is installed to `~/.cargo/bin/wasm-bindgen`, which is
  **not** on `$PATH` in this environment (confirmed: `which wasm-bindgen`
  found nothing, but the absolute path works). `scripts/build-wasm.sh`
  hardcodes `$HOME/.cargo/bin/wasm-bindgen` as a default, overridable via
  `$WASM_BINDGEN`. CI (M0.2) will need to either add `~/.cargo/bin` to
  `$PATH` or call the script as-is.

**Gaps and flags.**

- Green's task is exactly one function body: `draw()` in `src/lib.rs`. The
  doc comment on the stub states the intended `web_sys` call sequence
  (`get_element_by_id` → `dyn_into::<HtmlCanvasElement>` → `get_context("2d")`
  → `dyn_into::<CanvasRenderingContext2d>` → `set_fill_style` →
  `fill_rect`) as a *for*, not a *how* — Green should still make the actual
  API-shape decisions.
- `web-sys` features list in `Cargo.toml` includes `"console"`, which
  nothing currently uses — left in on the assumption Green will want
  `console::log_1` or similar for debugging the real implementation; harmless
  if unused, but Refactor should feel free to drop it if it stays unused.
- The Playwright NODE_PATH/ESM interop finding above should probably go
  into project memory or the README once M0.2 needs its own Playwright
  script, so it isn't rediscovered from scratch.
- I did not attempt GitHub Pages or CI in this round — that is explicitly
  M0.2's job, not this round's.

**General comments.**

The toolchain came out cleaner than the milestone intent's cautious framing
("Rust-to-web is the biggest unknown here") implied it might — every step
worked on the first real attempt with no fallback needed. The one genuine
gotcha (NODE_PATH + ESM) was environmental, not Rust/wasm-related, and is
now documented and worked around. Recommend Green proceed directly to
filling in `draw()`; I see no reason for an exit ramp back to planning.

---

**Test file(s) and currently-failing tests:**
- `tests/e2e/canvas_rectangle.test.mjs` — scenario "A person opens the built
  M0.1 page in a real browser and sees a coloured rectangle painted onto the
  canvas." Currently **fails**: wasm module's `#[wasm_bindgen(start)]` calls
  `draw()`, whose stub body is `todo!()`, which traps as `RuntimeError:
  unreachable` in the browser; the test reports
  `window.__viewerReady` never becomes `true` and exits 1. (Durable scenario.)
- `src/lib.rs` `tests::rectangle_fits_within_canvas` — **passes** already (a
  green guard/disposable unit test pinning that the rectangle geometry fits
  the declared canvas size; not part of this round's red signal).

**Skeleton created (for Green to fill):**
- `src/lib.rs`:
  - `pub const RECT_COLOR_RGB: (u8, u8, u8)`, `RECT_X/RECT_Y/RECT_W/RECT_H: u32`
    — already-final values, not stubs.
  - `#[wasm_bindgen] pub fn draw(canvas_id: &str)` — body is
    `todo!(...)`; **this is what Green implements.**
  - `#[wasm_bindgen(start)] pub fn main()` — already calls `draw("canvas")`,
    should not need to change.
- `www/index.html` — host page with `<canvas id="canvas">` and the module
  loader; should not need to change unless Green's API shape changes.
- `Cargo.toml` — dependencies (`wasm-bindgen`, `web-sys` with a features
  list) already declared; add more `web-sys` features here if Green's
  implementation needs a type not yet enabled.

**Commands for running this round's tests:**
```sh
# Build (from repo root):
./scripts/build-wasm.sh
# ...which runs, exactly:
#   cargo build --release --target wasm32-unknown-unknown
#   ~/.cargo/bin/wasm-bindgen --target web --out-dir www/pkg \
#     target/wasm32-unknown-unknown/release/viewer.wasm

# Headless verification (the scenario test):
NODE_PATH=/usr/local/lib/node_modules node tests/e2e/canvas_rectangle.test.mjs

# Host-side unit test (fast, no wasm/browser involved):
cargo test
```

**Full suite:** at this point in the project, `cargo test` (unit tests) and
the one Playwright scenario above **are** the full suite — there is nothing
else yet.

**Restructured the slice?** No. The four goals as given were already one
coherent, round-sized slice; I did not resequence or split them. I did
choose to actually *exercise* goals 1 and 2 (not just skeleton them) within
Red, since they are toolchain-verification goals rather than application
logic — deferring them to Green would have meant Green discovering a broken
toolchain while also trying to implement drawing logic, conflating two
different kinds of failure.

**Durable vs. disposable:**
- Durable (scenario): `tests/e2e/canvas_rectangle.test.mjs`.
- Disposable (unit test): `src/lib.rs::tests::rectangle_fits_within_canvas`.

**Rest of the suite green?** Yes — it's the only other test, and it passes
(see `cargo test` output above; 1 passed, 0 failed).

---

## Round 01 — Green — 2026-09-05T02:43:29+12:00 → 2026-09-05T02:44:04+12:00 (~1 min)

**What I did.**

Filled in `pub fn draw(canvas_id: &str)` in `src/lib.rs`, following Red's
doc-comment call sequence exactly: `web_sys::window()` →
`.document()` → `.get_element_by_id(canvas_id)` → `dyn_into::<HtmlCanvasElement>()`
→ `.get_context("2d")` → `dyn_into::<CanvasRenderingContext2d>()` →
`set_fill_style_str(...)` → `fill_rect(...)` using the pinned `RECT_*`
constants and `RECT_COLOR_RGB` (formatted as a CSS `rgb(r, g, b)` string, no
new colour representation invented). Nothing else in the crate, host page,
or build script needed to change.

**Successes.**

- `cargo test` (host target): 1 passed, 0 failed
  (`tests::rectangle_fits_within_canvas`), same as Red left it — untouched by
  this change.
- `./scripts/build-wasm.sh` (Red's script, unmodified): rebuilt
  `target/wasm32-unknown-unknown/release/viewer.wasm` and regenerated
  `www/pkg/` cleanly, no warnings.
- `NODE_PATH=/usr/local/lib/node_modules node
  tests/e2e/canvas_rectangle.test.mjs` against the freshly rebuilt output:
  `PASS canvas_rectangle: rectangle pixel matches expected colour, outside
  pixel untouched.` — a real `getImageData` pixel check in headless
  Chromium, not a screenshot, confirming the rectangle actually painted at
  the pinned coordinates in the pinned colour.

**What was difficult, and where the time went.**

- None. This round's goal-4 groundwork (constants pinned, `web-sys` features
  already enabled in `Cargo.toml`, build script already correct) meant the
  actual code change was the one function body Red's doc comment already
  specified move-for-move. Total elapsed ~1 minute, no blockers, nowhere
  near the 30-minute checkpoint.

**Compromises I made.**

- None beyond what Red already flagged (the hand-duplicated
  `EXPECTED.color`/coordinates between `src/lib.rs` and
  `tests/e2e/canvas_rectangle.test.mjs`, and `www/pkg/` staying gitignored
  build output). I did not introduce any new ones.

**Gaps and flags.**

- `set_fill_style_str` (rather than the older `set_fill_style(&JsValue)`) is
  available on the resolved `web-sys` 0.3.104 — confirmed by the successful
  build, not just assumed. No `Cargo.toml` changes were needed; the
  `"console"` feature Red left in remains unused (Red already flagged this
  for Refactor to drop if it stays that way).
- No signature changes from Red's skeleton — `draw(canvas_id: &str)`, its
  `#[wasm_bindgen]` export, and `main()` calling `draw("canvas")` are exactly
  as Red left them.
- I did not add error-reporting beyond the `.expect(...)`/`panic!(...)`
  messages already implied by the stub's doc comment; a panic here traps to
  `RuntimeError: unreachable` in the browser exactly as the `todo!()` did
  when something is wrong, which is what surfaced Red's intentionally-red
  state clearly. Nothing round-scoped calls for softer error handling, so I
  left it as-is for Refactor to weigh in on if it disagrees.

**General comments.**

Straight fill-in, no surprises. All four of the round's goals are now
demonstrated pass/fail-checked end to end: rendering approach decided
(Red), wasm32 build succeeds (Red + reconfirmed here after the `draw()`
change), a real browser paints and a headless Playwright test reads the
actual canvas pixels to confirm it (this phase), and the exact commands are
recorded in `scripts/build-wasm.sh` and this log. Recommend proceeding to
Refactor.

**Files touched:** `src/lib.rs` only (the `draw()` body, plus the two `use`
imports it needed — `wasm_bindgen::JsCast` and `web_sys::CanvasRenderingContext2d`).

**Exact commands run, in order, with results:**
```sh
cargo test
# → running 1 test; test tests::rectangle_fits_within_canvas ... ok
# → test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

./scripts/build-wasm.sh
# → Compiling viewer v0.1.0 (...)
# → Finished `release` profile [optimized] target(s)
# → Built www/pkg/.

NODE_PATH=/usr/local/lib/node_modules node tests/e2e/canvas_rectangle.test.mjs
# → PASS canvas_rectangle: rectangle pixel matches expected colour, outside pixel untouched.
```

**Signature changes from Red's skeleton:** none.

**Numbers proposed changing:** none — `RECT_COLOR_RGB`, `RECT_X/Y/W/H` used
as pinned, unmodified.

**Left for Refactor:** the unused `"console"` web-sys feature in
`Cargo.toml` (Red already flagged it); the hand-duplicated expected-colour/
coordinate constants between `src/lib.rs` and the Playwright test (Red
already flagged it, no shared-source-of-truth mechanism built this round);
whether the `.expect(...)`/`panic!(...)` error messages in `draw()` are the
right shape long-term versus something more structured — round-appropriate
as a proving-ground implementation, not evaluated for anything beyond that.

---

## Round 01 — Refactor — 2026-09-05T02:45:09+12:00 → 2026-09-05T02:47:33+12:00 (~3 min)

**What I did.**

Read the whole round's diff cold (Red `7b779ae` + Green `b8a8776`): `Cargo.toml`,
`src/lib.rs`, `www/index.html`, `scripts/build-wasm.sh`,
`tests/e2e/canvas_rectangle.test.mjs`. Then, per the round's explicit adversarial
focus, independently re-verified the headless harness from a fully clean state
rather than trusting Red's/Green's reported output:

1. `rm -rf target www/pkg` (clean slate — no cached build artifacts).
2. `cargo test` — passes (1/1), no warnings.
3. `./scripts/build-wasm.sh` from clean — succeeds, produces `www/pkg/`.
4. `NODE_PATH=/usr/local/lib/node_modules node tests/e2e/canvas_rectangle.test.mjs`
   against that fresh build — **PASS**, real `getImageData` pixel check.
5. **Fault injection, attempt 1** — changed the fill colour in `draw()` to
   `rgb(0,0,0)` (kept `fill_rect` itself intact, so this isolates "wrong
   colour" specifically), rebuilt via the same script, reran the e2e test:
   **FAIL**, reporting the actual observed wrong pixel —
   `rgba(0,0,0,255), expected rgba(200,60,60,255)`. Correct failure, real data,
   not a stale pass.
6. Restored, then **fault injection, attempt 2** — commented out the entire
   `fill_rect(...)` call (nothing drawn at all), rebuilt, reran: **FAIL**,
   `rgba(0,0,0,0), expected rgba(200,60,60,255)` (alpha 0 = untouched canvas
   backing). Correct failure, different and correctly-distinguishable signature
   from attempt 1.
7. Restored `src/lib.rs` to the committed version (`diff` against a backup
   confirmed byte-identical restoration), rebuilt clean, reran the full suite
   (`cargo test` + e2e): both green again.
8. Checked for compiler warnings on the real committed code with
   `cargo build --release --target wasm32-unknown-unknown`: none. (The 3
   warnings seen during fault-injection step 6 were from my own dead-code
   edit, not present in the shipped code — confirmed by rebuilding the
   restored file cleanly afterward with zero warnings.)
9. Reviewed `src/lib.rs`, `Cargo.toml`, `www/index.html`,
   `scripts/build-wasm.sh` for fold-in issues: naming, conventions, doc-comment
   accuracy, duplication. Found the code idiomatic and the doc comments
   accurate (checked the coordinate-convention comment against the actual
   `fill_rect` call — consistent; checked the "kept in sync by hand" comment
   on `RECT_COLOR_RGB` against the actual test file — still true).
10. Acted on one of the two flagged gaps: removed the unused `"console"`
    `web-sys` feature from `Cargo.toml` (confirmed unused by grep and by a
    clean rebuild with it removed — still builds, zero warnings, full suite
    still green). Left the hand-duplicated constants gap as a deferral — see
    Compromises.

**Successes.**

- **The headless harness is real**, not theater. Verified independently, not
  just re-reading Green's report:
  - It builds from a genuinely clean state each time (no `target/`, no
    `www/pkg/` reused across a pass/fail pair) — ruling out a stale-cache
    false positive.
  - It reads live `getImageData` pixel values from a real headless Chromium
    page, not a hardcoded assertion — proven by making the *actual rendered
    pixel* wrong two different ways and watching the failure message report
    the *actual wrong value observed*, not a canned string.
  - The two fault-injection failures are distinguishable from each other
    (wrong-but-opaque colour vs. fully untouched/transparent), which is exactly
    what you'd expect from a harness that is genuinely reading pixel state
    rather than a boolean gate.
  - Restoring the original code reproduces the original PASS, confirming
    nothing about my probing left residue.
- `cargo test`, the wasm32 release build, and the e2e scenario all reproduce
  cleanly from a fresh checkout state using exactly the commands
  `scripts/build-wasm.sh` and the round log record — goal 4 (reproducible
  commands) holds up under an independent rerun, not just Green's own report.
- `src/lib.rs` is straightforward, idiomatic Rust for what it needs to do:
  the `window → document → get_element_by_id → dyn_into → get_context →
  dyn_into → set_fill_style_str → fill_rect` chain is the standard `web-sys`
  canvas 2D pattern, error messages via `.expect()`/`panic!()` are
  descriptive, and the constants are named and documented rather than
  inlined. No sign-flip or off-by-one found in `RECT_X/Y/W/H` — cross-checked
  against the disposable unit test's canvas bounds and the actual pixel
  sample points in the JS test; all consistent.
- Canvas 2D via `wasm-bindgen`/`web-sys` is sound to build a per-tick
  animation loop on: nothing about this round's approach (module
  instantiation, `#[wasm_bindgen(start)]`, the context-lookup chain) forecloses
  calling `draw()` (or a renamed equivalent) repeatedly from a `requestAnimationFrame`
  loop later — the context lookup is cheap and there's no per-call setup this
  round did that would need to be hoisted out for a hot loop to work correctly.
  The one thing Round 2 should watch: `get_context("2d")` currently re-resolves
  the canvas element and context from scratch inside `draw()` every call. That's
  fine once; if Round 2 calls this every frame, it's wasted DOM lookup work per
  tick — not a correctness bug, a performance-hygiene note for whoever writes
  Round 2's Red skeleton.

**What was difficult, and where the time went.**

Nothing was difficult. Almost all the ~3 minutes of wall-clock time went into
the mechanical rebuild-test-fault-inject-restore-rebuild-test cycle (which is
fast on this project's tiny build), not into indecision or debugging. No
tooling snags — the recorded commands worked exactly as documented on the
first try, every time I ran them.

**Compromises I made.**

- Deliberately **deferred** the hand-duplicated `RECT_COLOR_RGB`/`RECT_X..H`
  vs. `EXPECTED`/sample-coordinate constants between `src/lib.rs` and
  `tests/e2e/canvas_rectangle.test.mjs`, rather than building a shared
  source of truth (e.g. exporting the constants via `#[wasm_bindgen]` and
  reading them from the page in the test). Reasoning: building that
  machinery now would mean engineering a synchronization mechanism for four
  numbers that are not expected to change again this round — this is a
  proving-ground milestone, not the round where colour/geometry becomes
  dynamic. It becomes worth the engineering the moment Round 2 (a per-tick
  animation loop) starts actually varying position/colour per frame, at
  which point the two sides diverging would be a live risk instead of a
  theoretical one, and the shared-source-of-truth shape can be designed
  around what Round 2 actually needs (e.g. exporting current draw-state to
  JS) rather than speculatively now. Flagging this explicitly rather than
  silently dropping it: **Round 2's planner/Red should decide whether to
  build that mechanism as part of adding per-tick state**, since duplicated
  constants will get materially riskier the moment they stop being constant.
- I did act on the other flagged gap (unused `"console"` feature) rather
  than deferring it, since it was a zero-risk, fully-verified deletion with
  no design decision attached — not really a "compromise," just a same-round
  cleanup.

**Gaps and flags.**

- (Carried forward, not new) The hand-duplicated constants — see Compromises
  above; explicit recommendation for Round 2 planning.
- (Carried forward, not new) `www/pkg/` stays gitignored build output,
  reproducible via `scripts/build-wasm.sh`; confirmed again this phase that
  this reproduction is reliable from a fully clean `target/`+`www/pkg/` wipe,
  not just from an incremental rebuild.
- Minor, non-blocking: `draw()` re-resolves the canvas/context from the DOM
  on every call. Not a problem for a single call-on-load; worth hoisting or
  caching if Round 2 calls it per animation frame. Not acted on now since
  there's no per-tick loop yet to optimize for, and doing so speculatively
  risks guessing wrong about Round 2's actual shape.
- Untracked `cycle-log/tranche-0/m0.1/plan.md` and `cycle-log/tranche-0/plan.md`
  exist in the working tree but are outside this round's file list (not part
  of Red's or Green's commits) — left untouched as out of scope for this
  phase.

**General comments.**

This is a clean first round for a proving-ground milestone. The most
important thing to check — whether the headless verification harness can be
trusted for every round that follows in this tranche — held up under direct
attack: two different, deliberately-injected wrong-`draw()` states each
produced a distinct, correctly-diagnosed failure with real observed pixel
data, and restoring the original code reproduced the original pass. Nothing
found here calls the milestone's or round's goals into question.

**Change list (this phase):**
- `Cargo.toml`: removed the unused `"console"` `web-sys` feature — confirmed
  unused (grep, and a clean rebuild without it still passes the full suite
  with zero warnings). Rationale: dead weight in the build, flagged by both
  Red and Green as safe to drop once confirmed unused.

**Adversarial pass — what I tried:**
- Full clean rebuild (`target/` + `www/pkg/` wiped) before every test run, to
  rule out stale-cache false positives. Found nothing wrong — reproduced
  cleanly every time.
- Fault injection #1: wrong fill colour (`rgb(0,0,0)` instead of the pinned
  colour), `fill_rect` call otherwise intact. Result: e2e test failed
  correctly, reporting the real wrong pixel value.
- Fault injection #2: `fill_rect` call fully commented out (nothing painted).
  Result: e2e test failed correctly, reporting alpha 0 (untouched canvas),
  correctly distinguished from fault #1's failure message.
- Reviewed `RECT_X/Y/W/H` against canvas bounds (200x150) and the JS test's
  sample points (40,40 inside; 150,120 outside) for an off-by-one or
  sign-convention error. Found none — geometry and sample points are
  internally consistent on both sides.
- Checked for compiler warnings on the real committed code (both host and
  wasm32 targets). None found.
- Considered whether canvas 2D forecloses anything Round 2 (per-tick
  animation) will need. Found no blocker; flagged one minor performance-
  hygiene note (re-resolving canvas/context every call) for Round 2 to be
  aware of, not act on now.

**Correctness findings:** none. No wrong sign, no wrong ordering, no
incorrect conserved-quantity issue — nothing to report at the "shipped code
is actually wrong" level.

**Current suite runtime:** `cargo test` ~9s (mostly compile, from clean;
near-instant incrementally). Full wasm32 release build ~13s from clean,
~0.1s incrementally. Playwright e2e scenario: a few seconds (browser launch
dominates). Entirely acceptable for a suite this size — nothing to trim yet.

**Verdict: Advance.**

All four of this round's goals are met and independently re-verified, not
just trusted from Green's report:
1. Rendering approach (canvas 2D via `wasm-bindgen`/`web-sys`) is decided,
   justified with actual toolchain evidence, and confirmed sound to build
   Round 2's animation loop on.
2. The wasm32 build succeeds, reproducibly, from a clean checkout.
3. The build, served locally, draws the rectangle and a headless Playwright
   test reads real canvas pixel data to confirm it — and that harness
   survived direct adversarial attack (two independent fault injections,
   both correctly caught).
4. The exact build/serve commands in `scripts/build-wasm.sh` and the round
   log are accurate and reproducible — confirmed by rerunning them verbatim
   from a fully clean state.

The one open item (hand-duplicated constants) is a real but low-urgency gap,
explicitly flagged forward to Round 2's planning rather than blocking this
round — it does not call this round's goals into question, only a future
one's design.

**What I would have done with another 30 minutes:** written a small
`#[wasm_bindgen]`-exported accessor (or a JSON blob) for the `RECT_*`
constants and updated the JS test to read them live, eliminating the
hand-duplication gap entirely rather than deferring it — but only if Round
2's actual shape (what becomes dynamic) were known, so the mechanism is
built for the real need rather than guessed at.
