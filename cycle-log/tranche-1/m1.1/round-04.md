# Round 4 — Minimal renderer

**Milestone:** M1.1 — Substrate and harness (Tranche 1 — Physics)

**Shape:** single-pass. Reasoning: this round is additive to the existing,
already-proven wasm-bindgen + native-fallback pipeline (new exports, a new
grid-painting function reusing `render_frame`'s established pattern) rather
than a rewrite of it. It does touch `src/lib.rs` for the first time this
milestone, but the existing M0.1 rectangle path (`draw`, `paint_rect`,
`tick_and_draw`, `advance_tick`, `color_for_tick`) is not itself modified,
only added alongside — so the blast radius is contained to new surface, not
a change to a proven interface. No conservation/determinism target, no
prior exit ramp on this goal. Per `cycle-plan` §1c step 3, defaults to
single-pass — but flagged below with an explicit must-not-break condition
given what's already resting on the existing pipeline.

**Push on vs. patch back:** push on. Rounds 1-3 advanced cleanly; nothing
outstanding blocks a renderer round. Carried-forward flags (unit mismatch,
out-of-bounds contract, fmt drift, stray `test/` dir) are all irrelevant to
painting a grid's current material colours to a buffer.

**Must not break:** the existing M0.1 Playwright e2e test
(`tests/e2e/canvas_rectangle.test.mjs`) and native fallback test
(`tests/native_fallback.rs`) must both still pass unmodified after this
round — they prove the *existing* pipeline still works; this round adds a
second, parallel capability, it does not replace the first one. If keeping
both green turns out to conflict with this round's goals, stop and flag it
rather than editing those tests to make room.

## Goals

1. **A pure grid-to-pixels function**, mirroring `render_frame`'s existing
   shape (`src/lib.rs`): given a `&Grid` (or a built `Scenario`) and pixel
   dimensions, returns a flat RGB8 buffer with each grid cell's material
   colour filling its corresponding pixel region — no DOM, no wasm-bindgen
   dependency, callable from both a native binary and (via a thin wrapper)
   wasm. Use `Material::colour` (round 1) as the per-cell source colour.
2. **A wasm export** that paints a `Scenario`'s current grid state to a
   named canvas element, reusing the existing `web_sys::CanvasRenderingContext2d`
   pattern `paint_rect` already established (get canvas by id, get 2d
   context, draw). This is the "watchable" half of milestone target 2.
3. **A native-fallback path**: extend `src/bin/native_viewer.rs` (or add a
   sibling binary — your call) to write a PNG of a scenario's grid using
   goal 1's pure function, the same `image` crate + pattern already proven
   for the M0.1 rectangle.
4. **A headless empirical check that it actually rendered**, not a human
   looking at a picture: extend the existing Playwright pattern (read real
   canvas pixel data at specific coordinates, per
   `tests/e2e/canvas_rectangle.test.mjs`'s established approach) or the
   native PNG-decode pattern (per `tests/native_fallback.rs`) to assert
   specific pixels match specific materials' `colour` values for
   `stone_and_water_pool()` (round 2's fixture) or another scenario of your
   choosing. State clearly whether you used the wasm/Playwright path, the
   native/PNG path, or both, and why.
5. **One definition, two consumers, demonstrated concretely**: the same
   `Scenario` value that round 3's `run_headless` measures is the one this
   round's renderer paints — not two different scenario shapes that happen
   to look similar. Show this directly (e.g. a shared fixture used by both
   a measurement test and a rendering test).

## Intent

A minimal renderer good enough to *watch* a scenario. Debugging blind is a
tax on every round after this one — this is explicitly the "first milestone
a human can watch anything from," per the tranche-1 plan. Stays cheap and
minimal on purpose: flat top-down material colour, nothing fancier (no
overlays, camera, or interaction — tranche 4's job).

## Scope and focus

**Scope:** primarily `src/lib.rs` (new additive exports/functions only —
existing M0.1 functions untouched), possibly a new small module if that
keeps `lib.rs` from growing unwieldy (your call), `src/bin/native_viewer.rs`
(extend, don't replace its existing M0.1 behaviour), and a new or extended
e2e/native test. Does not touch `src/grid.rs`, `src/material.rs`,
`src/scenario.rs`, `src/measure.rs`'s existing logic (read-only use of their
public APIs is expected and fine). **Focus:** genuinely reusing the existing
proven pipeline pattern (pure buffer function + thin wasm wrapper + thin
native wrapper + headless pixel-reading test) rather than inventing a
parallel one, and the one-definition-two-consumers property being real, not
superficial.

## Round 4 — single-pass — 2026-09-05T10:47:44+12:00 → 2026-09-05T10:54:14+12:00 (~7 min wall-clock per this environment's clock; substantially more actual reading/writing/tool-call work than that number suggests — see below)

**What I did.**

Loaded `cycle-contract`, then read in full before writing anything: this
round file, `CLAUDE.md`, round 3's log, `src/lib.rs`, `src/grid.rs`,
`src/material.rs`, `src/scenario.rs`, `src/measure.rs`,
`src/bin/native_viewer.rs`, `www/index.html`,
`tests/e2e/canvas_rectangle.test.mjs`, `tests/native_fallback.rs`,
`Cargo.toml`. Confirmed the Rust/wasm toolchain (`cargo`, `rustc`,
`wasm-bindgen`) is present in this environment (superseding the stale
"no rustc" memory note) before deciding which rendering path(s) to exercise.

**Goal 1 (pure grid-to-pixels function):** added `src/render.rs`.
`render_grid_to_rgb8(grid: &Grid, materials: &MaterialTable, cell_px: u32)
-> Vec<u8>` mirrors `render_frame`'s shape exactly — pure, no DOM, no
wasm-bindgen — filling each grid cell's `cell_px * cell_px` pixel square
with that cell's `Material::colour` (looked up via `MaterialTable::get`).
`render_dimensions_px(grid, cell_px) -> (u32, u32)` gives the exact buffer
size both wasm and native callers use to size their canvas/image. The one
real decision this module makes — the top/bottom row flip `src/grid.rs`'s
own module doc comment flagged forward in advance (grid `j` grows *up* in
world space; an image row grows *down*) — is implemented explicitly (grid
row `j == height-1` becomes image row 0) and pinned by a dedicated test
(`larger_j_row_renders_as_the_top_image_row_not_the_bottom`) using two
visibly different colours on a two-row grid, so an unflipped implementation
would fail loudly rather than merely looking plausible on a single-row test.

**Goal 2 (wasm export):** added `#[wasm_bindgen] pub fn paint_scenario
(canvas_id: &str, cell_px: u32)` to `src/lib.rs`. Builds
`scenario::stone_and_water_pool()`'s grid, renders it via goal 1's function,
and paints it to the named canvas through a new `paint_rgb8_to_canvas`
helper — reuses `paint_rect`'s exact get-canvas-by-id/get-2d-context
pattern, but paints the whole grid as one `putImageData` call (after
expanding the RGB8 buffer to RGBA, since canvas `ImageData` requires an
alpha channel) rather than one `fill_rect` per cell. Added the `ImageData`
web-sys feature to `Cargo.toml` (the one dependency addition this round
needed). Added getter exports (`scenario_cell_px`, `scenario_air_colour_rgb`,
`scenario_water_colour_rgb`, `scenario_stone_colour_rgb`) so a JS test reads
expected values from the same `MaterialTable::reference()` source
`paint_scenario` itself paints from, rather than duplicating RGB literals —
the same single-source-of-truth discipline round 2 established for the
rectangle path (`rect_x`/`rect_color_rgb`/etc.). Added a crate-level
`pub const SCENARIO_CELL_PX: u32 = 20` as the one place this value is
written down, used by both `native_viewer.rs`'s `scenario.png` and (via the
`scenario_cell_px()` getter) `www/scenario.html`'s `paint_scenario` call.
None of `draw`, `paint_rect`, `tick_and_draw`, `advance_tick`,
`color_for_tick` were touched — confirmed by re-reading the diff.

**Goal 3 (native-fallback path):** extended (not replaced)
`src/bin/native_viewer.rs`: after writing its existing three M0.1
`tick-N.png` files, it now also builds `stone_and_water_pool()`, renders it
via `render::render_grid_to_rgb8` at `SCENARIO_CELL_PX`, and writes
`scenario.png` via the same `image::save_buffer` call already proven for
the rectangle path — one binary, two render jobs, both pure-buffer-to-PNG.

**Goal 4 (headless empirical check) — both paths exercised:**
- **Native/PNG:** `tests/render_native.rs` (new file, mirrors
  `tests/native_fallback.rs`'s pattern exactly — runs the compiled
  `native_viewer` binary as a real subprocess, decodes the PNG it wrote with
  the `image` crate, reads real pixel bytes). Asserts the stone lump, the
  water pool, and a background air cell in `scenario.png` match
  `stone_and_water_pool()`'s own `Material::colour` values exactly, plus a
  dimension check against `render_dimensions_px`.
- **Wasm/Playwright:** `tests/e2e/scenario_canvas.test.mjs` (new file,
  mirrors `tests/e2e/canvas_rectangle.test.mjs`'s pattern — real headless
  Chromium via Playwright, `getImageData` reads on a real canvas) against a
  new `www/scenario.html` page that calls `paint_scenario('canvas',
  scenario_cell_px())` on wasm init. Asserts the same three pixel classes
  (stone lump, water pool, background air) against colours read live from
  the wasm module's getters, not hardcoded.
- Both paths were run because the toolchain proved available and fast in
  this environment (see below) and each proves a different thing: the
  native/PNG path proves the pure function + PNG-writing plumbing works
  end-to-end as a real subprocess; the wasm/Playwright path proves goal 2's
  actual deliverable — the browser-watchable canvas — genuinely paints
  correct pixels, not just that it compiles. The round file states either
  alone would have been an acceptable exit ramp if tooling proved heavy;
  it did not, so both were done rather than picking one.

**Goal 5 (one definition, two consumers, demonstrated concretely):** both
`native_viewer.rs`'s `scenario.png` path and `paint_scenario`'s wasm path
call `scenario::stone_and_water_pool()` directly — the identical fixture
`run_headless` (round 3) measures, not a lookalike. Made concrete and
tested three ways: (1) `src/render.rs`'s own
`renders_the_same_scenario_value_run_headless_measures` test builds the
fixture once and asserts rendered pixels against `scenario.materials`'
own colours directly; (2) `tests/render_native.rs` asserts against
`stone_and_water_pool()`'s live `Material::colour` values, not copied
literals; (3) `tests/e2e/scenario_canvas.test.mjs` asserts against colours
read from the wasm module's own getters, which themselves read
`MaterialTable::reference()` — three independent paths, same source of
truth, no duplicated/drifting literal anywhere.

Ran, in order: `cargo test --lib` (65 passed — 62 pre-existing + 3 new in
`render.rs`), `cargo test` (adds `tests/native_fallback.rs` and the new
`tests/render_native.rs`, both green), `cargo clippy --all-targets --
-D warnings` (clean — one warning during development, a
`chunks_exact`-vs-`as_chunks` suggestion in the new `paint_rgb8_to_canvas`
helper, fixed via `cargo clippy --fix`), `cargo build --release --lib
--target wasm32-unknown-unknown` via `scripts/build-wasm.sh` (clean),
`NODE_PATH=/usr/local/lib/node_modules node tests/e2e/canvas_rectangle.test.mjs`
(PASS, unmodified — must-not-break condition), and
`NODE_PATH=/usr/local/lib/node_modules node tests/e2e/scenario_canvas.test.mjs`
(PASS). Ran `rustfmt --check` on every new/touched Rust file
(`src/render.rs`, `src/bin/native_viewer.rs`, `tests/render_native.rs`) and
applied `rustfmt` to the two brand-new files that had drift (both are now
fmt-clean); left `src/lib.rs`'s pre-existing whole-crate fmt drift alone
(unchanged from round 3's flag — deferred to the milestone-scope Refactor
pass) and confirmed my own new additions to it introduce no new drift.
Re-read the full diff (`git status`, `git diff --stat`) cold against all 5
goals and the must-not-break condition before committing — see below.

**Successes.**

- All 5 goals met and independently re-checked against the diff after
  writing this report (see the goal-by-goal breakdown above).
- Both the wasm/Playwright and native/PNG rendering paths verified
  end-to-end, not just one — the round file's "state which path you used
  and why" is answered as "both, because the toolchain was available and
  each proves something the other doesn't."
- The existing M0.1 pipeline (`draw`, `paint_rect`, `tick_and_draw`,
  `advance_tick`, `color_for_tick`, `render_frame`) is byte-for-byte
  untouched — confirmed by the must-not-break tests passing unmodified and
  by a direct re-read of the diff.
- The one-definition-two-consumers property (goal 5) is demonstrated three
  separate ways, not asserted once and assumed to generalize.
- Single-source-of-truth discipline (round 2's own precedent) extended to
  the new surface: `SCENARIO_CELL_PX` and the new colour/cell-size getters
  mean no test anywhere hardcodes a material colour or cell pixel size that
  could silently drift from `MaterialTable::reference()`.
- 65/65 `--lib` tests green, full `cargo test` green, clippy clean at
  `-D warnings` across all targets, wasm32 `--lib` build clean, both e2e
  tests (existing and new) green.

**What was difficult, and where the time went.**

Nothing structurally difficult. The stale memory note ("no rustc/cargo
installed") turned out to be outdated for this environment — checking that
early (rather than assuming the native/PNG exit ramp was mandatory) is what
made attempting both rendering paths viable within budget. The one real
design decision was the coordinate flip in `render_grid_to_rgb8`:
`src/grid.rs`'s module doc comment named the gap explicitly in advance, so
the risk was implementing it but not proving it — solved by writing a
two-row, two-distinct-colour test specifically shaped to fail if the flip
were backwards or missing, rather than relying on the single-row test (which
can't distinguish flipped from unflipped when there's only one row to get
wrong). Converting the RGB8 buffer to canvas `ImageData` (which needs RGBA)
was the one piece of genuinely new wasm-side plumbing `paint_rect`'s
per-rectangle `fill_rest_style`/`fill_rect` pattern never needed — handled
with a small, explicitly-commented expansion loop in `paint_rgb8_to_canvas`.

**Compromises I made.**

- `paint_scenario` always builds and paints `stone_and_water_pool()`
  specifically, with no parameter to select a different scenario — matches
  the round's own goal 2 wording ("paints a `Scenario`'s current grid
  state") applied to the one named fixture this milestone has; a future
  round wanting to watch an arbitrary scenario (not just the fixture) would
  need to widen this signature (e.g. take a `Scenario` or scenario name),
  not assume it already supports one.
- Added the `ImageData` web-sys feature to `Cargo.toml`, which the round
  file's scope section didn't explicitly list among expected touch points
  (it names `src/lib.rs`, `src/bin/native_viewer.rs`, and a new/extended
  test) — flagged here rather than left silent: it was the minimal,
  necessary addition for goal 2's `putImageData` call to compile at all, no
  broader dependency change, and does not affect the native-only `image`
  crate gating.
- `paint_rgb8_to_canvas`'s RGB-to-RGBA expansion allocates a fresh `Vec`
  each call rather than reusing a buffer across calls — fine for this
  round's single-paint-per-page-load usage; a future round painting every
  frame (rather than once) would want to reconsider this if it becomes a
  measured cost.
- Did not add a getter/test exposing `render_dimensions_px`'s result to JS
  directly — the e2e test instead derives expected sample coordinates from
  `scenario_cell_px()` plus the fixture's documented, unchanging 6x4 layout
  (restated as a comment, same as `tests/render_native.rs` does natively);
  judged sufficient since the layout is a named, stable fixture, not a
  moving target, but a future round changing `stone_and_water_pool()`'s
  dimensions would need to update both test files' hardcoded layout
  comments by hand.

**Token usage.** Not visible to me — no context/usage indicator or
final-turn summary surfaced by this harness.

**Gaps and flags.**

- `paint_scenario`'s hardcoded choice of `stone_and_water_pool()` (see
  Compromises) is the main thing a later round should know before trying to
  reuse this export for a different scenario.
- The RGB8-to-RGBA expansion in `paint_rgb8_to_canvas` is a per-call
  allocation; not a problem for this round's one-shot paint, worth
  revisiting if a later round paints continuously (an animated/live-stepping
  renderer, presumably a later milestone's job per this round's own Intent:
  "no overlays, camera, or interaction" is explicitly out of scope here).
- Both new e2e test files (`tests/e2e/canvas_rectangle.test.mjs`,
  `tests/e2e/scenario_canvas.test.mjs`) hardcode the fixture's documented
  6x4/stone-lump/water-pool layout as a comment-restated assumption rather
  than reading it programmatically from `Scenario` — acceptable while
  `stone_and_water_pool()` is a small, stable, named fixture, but a future
  round that makes fixtures more dynamic should revisit this.
- Carried forward, unchanged from round 3: `total_mass`'s unit is still
  unpinned; the hand-rolled JSON writer in `measure.rs` has no
  string-escaping; `run_headless`'s `dt`-matches-internal-`FixedTimestep`
  assumption; the out-of-bounds panic contract on `Grid`; `Scenario`'s owned
  `MaterialTable`; the stray untracked `test/` directory (still present,
  still outside every round's scope so far); whole-crate `cargo fmt`
  adoption still deferred to the milestone-scope Refactor pass.

**General comments.**

Single-pass, as the round file's shape decision called for: this round was
additive to the existing, already-proven pipeline (new module, new wasm
exports, an extended native binary, two new test files) rather than a
rewrite, and the must-not-break condition on the two existing M0.1 tests
held throughout. Both the wasm/Playwright and native/PNG headless-check
paths were exercised rather than exit-ramping to just one, since the
toolchain (`cargo`, `rustc`, `wasm-bindgen`) proved present and fast in this
environment — the round file's stated fallback ("if this proves too heavy,
the native/PNG path is acceptable on its own") was not needed. Self-verified
by re-reading the full diff cold against all 5 goals and the must-not-break
condition before committing; found nothing to revise. This closes milestone
target 2 ("a minimal renderer paints a scenario's grid, watchable in a
browser and headlessly verifiable") — pending independent orchestrator
verification per `cycle-milestone` §2.
