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
