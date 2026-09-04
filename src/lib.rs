//! M0.1 toolchain proving ground.
//!
//! Round 1 proved the chain from `cargo build --target wasm32-unknown-unknown`
//! through `wasm-bindgen` to a real browser canvas: a single static coloured
//! rectangle. Round 2 proved the chain also carries a *running* program — the
//! rectangle visibly changes once per fixed tick — and removed the
//! hand-duplicated constants Round 1's Refactor flagged forward.
//!
//! Rendering approach unchanged from round 1: canvas 2D via
//! `wasm-bindgen`/`web-sys`. See `cycle-log/tranche-0/m0.1/round-01.md` for
//! why.
//!
//! ## Round 4 retrofit: the tick is now driven by `FixedTimestep`
//!
//! Round 2 shipped this with an ad-hoc `TICK: Cell<u32>` counter, incremented
//! by exactly one on every call to [`tick_and_draw`] — a known, flagged
//! shortcut (see the milestone's round-04 log for why). M0.4 round 3 built
//! the shared [`timestep::FixedTimestep`] accumulator harness precisely to
//! replace throwaway counters like that one; this round wires it in.
//!
//! JS still owns the wall-clock timer (`setInterval` in `www/index.html`),
//! but it now measures the real elapsed time between fires (via
//! `performance.now()` deltas) and passes that duration, in seconds, to
//! [`tick_and_draw`]. Rust feeds the duration to a crate-local
//! [`timestep::FixedTimestep`] via [`advance_tick`], which decides — per the
//! harness's own accumulator semantics — how many fixed steps (0, 1, or more)
//! have elapsed since the last call, and advances the tick count by that many
//! (not by a bare `+1`). [`color_for_tick`] and [`paint_rect`] are unchanged;
//! only the mechanism deciding *how far* the tick count moves per call is
//! new.
//!
//! [`advance_tick`] is deliberately free of any DOM/`web-sys` dependency —
//! same reasoning as `timestep.rs` itself — so it is unit-testable directly
//! under `cargo test --lib`, with no browser involved. [`tick_and_draw`] is
//! the thin `#[wasm_bindgen]` wrapper that calls it and then repaints.
//!
//! ## Single source of truth for shared constants (round 2, goal 2)
//!
//! Round 1 left the expected colour/coordinate values hand-duplicated as
//! literals in both this file and `tests/e2e/canvas_rectangle.test.mjs`. Fixed
//! by exporting plain `#[wasm_bindgen]` getter functions for every value the
//! JS test needs (`rect_x`, `rect_y`, `rect_w`, `rect_h`, `rect_color_rgb`,
//! `rect_color_rgb_alt`, `tick_interval_ms`). The JS test calls these on the
//! already-loaded wasm module at runtime instead of re-declaring the
//! numbers — there is exactly one place these values are written down.

use std::cell::{Cell, RefCell};

// M0.4: shared math primitives (Scalar, Vec2, GridIndex). Not used by this
// file's own canvas logic (M0.1's rectangle) — see src/math.rs for what it
// is and why. M1.1 round 1's grid module below is GridIndex's/Vec2's first
// real caller.
mod math;

// M0.4 round 3: fixed-timestep accumulator harness (Scalar dt in, step
// count out). Round 4 wires this into tick_and_draw via advance_tick below
// — see src/timestep.rs for what it is and why.
mod timestep;

// M1.1 round 1: material representation (Material, MaterialTable) and the
// grid (Grid) they're stored in — see src/material.rs and src/grid.rs.
// `pub` so later rounds/milestones and integration tests (tests/*.rs) can
// build on them, the same way RECT_COLOR_RGB etc. below are `pub` for
// tests/native_fallback.rs.
pub mod material;
pub mod grid;

// M1.1 round 2: `Scenario`, the "one definition, two consumers" type round
// 3 (headless runner) and round 4 (renderer) will both build a Grid from —
// see src/scenario.rs.
pub mod scenario;

// M1.1 round 3: the headless runner, the first of `Scenario`'s two
// consumers — builds a Grid from a Scenario, steps it, and measures it to
// JSON with no browser/DOM involved. See src/measure.rs. `pub` for the same
// reason material/grid/scenario are: later rounds and integration tests
// build on it directly.
pub mod measure;

// M1.1 round 4: the renderer, the second of `Scenario`'s two consumers —
// paints the same Scenario/Grid value's current state to pixels, no
// browser/DOM dependency, mirroring this file's own M0.1 render_frame
// shape. See src/render.rs. `pub` for the same reason measure/scenario/
// grid/material are: `src/bin/native_viewer.rs` and integration tests build
// on it directly.
pub mod render;

use math::Scalar;
use timestep::FixedTimestep;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::CanvasRenderingContext2d;

thread_local! {
    /// The crate-local tick count driving [`tick_and_draw`]'s colour
    /// decision. Wasm in a browser tab is single-threaded, so a
    /// `thread_local!` `Cell` is sufficient. Unlike round 2's ad-hoc
    /// counter, this is never incremented directly by a call —
    /// [`advance_tick`] advances it by whatever step count
    /// [`FixedTimestep::advance`] reports (0, 1, or occasionally more),
    /// never by a bare `+1` per call.
    static TICK: Cell<u32> = const { Cell::new(0) };

    /// The shared M0.4 fixed-timestep accumulator driving [`advance_tick`].
    /// Built with `dt` equal to [`TICK_INTERVAL_MS`] converted to seconds, so
    /// the harness's fixed step cadence matches the cadence the rectangle was
    /// already animating at pre-retrofit — the externally-observable
    /// behaviour (colour alternates roughly every `TICK_INTERVAL_MS`) is
    /// preserved, even though the *mechanism* deciding when a step elapses is
    /// now real elapsed-time accounting rather than "one step per call".
    static TIMESTEP: RefCell<FixedTimestep> =
        RefCell::new(FixedTimestep::new(DT_SECONDS));
}

/// [`TICK_INTERVAL_MS`] expressed in seconds — the fixed step size `dt` fed
/// to [`TIMESTEP`]'s [`FixedTimestep`]. `Scalar` (`f32`) division of small
/// constants like this is exact enough that no tolerance is needed when
/// comparing against it directly.
pub const DT_SECONDS: Scalar = TICK_INTERVAL_MS as Scalar / 1000.0;

/// The colour painted on even ticks (including tick 0), as 8-bit sRGB.
pub const RECT_COLOR_RGB: (u8, u8, u8) = (200, 60, 60);

/// The colour painted on odd ticks, as 8-bit sRGB. New this round: this is
/// what makes the tick visible as a colour change. Chosen far enough from
/// `RECT_COLOR_RGB` in every channel that no compression/rounding step
/// between here and a browser's canvas backing store could plausibly make
/// them compare equal by accident.
pub const RECT_COLOR_RGB_ALT: (u8, u8, u8) = (60, 120, 200);

/// How often, in milliseconds, the rectangle is expected to advance one
/// tick. Read by `www/index.html` via [`tick_interval_ms`] to drive its
/// `setInterval`, and by the Playwright test to size its wait timeout — so
/// changing this one constant changes both the running page and the test
/// that watches it.
pub const TICK_INTERVAL_MS: u32 = 150;

/// Rectangle geometry, in canvas pixel coordinates.
///
/// Coordinate convention (pinned once, applies to every later use of this
/// canvas): origin `(0, 0)` is the canvas's top-left corner, `x` grows
/// rightward, `y` grows *downward* — this is the HTML canvas convention, and
/// it is the opposite of a math/physics y-up convention. Anything that maps
/// simulation coordinates onto this canvas later must account for that flip
/// explicitly rather than assume it away.
pub const RECT_X: u32 = 20;
pub const RECT_Y: u32 = 20;
pub const RECT_W: u32 = 60;
pub const RECT_H: u32 = 40;

/// The on-screen pixel size of one grid cell used by both
/// `src/bin/native_viewer.rs`'s `scenario.png` and `www/scenario.html`'s
/// `paint_scenario` call — single source of truth (round 2's own naming for
/// this exact pattern) so the native and wasm rendering paths, and any test
/// reading pixels back, never disagree on cell size. See `src/render.rs`'s
/// module doc comment for why this is a plain screen-pixel integer, not
/// `Scenario::cell_size`'s world-space unit.
pub const SCENARIO_CELL_PX: u32 = 20;

/// Looks up the canvas element by `canvas_id`, gets its 2D rendering context,
/// and fills a `RECT_W` x `RECT_H` rectangle at `(RECT_X, RECT_Y)` in
/// `color`.
///
/// This is round 1's proven `draw` body, unchanged apart from taking the
/// colour as a parameter instead of always using `RECT_COLOR_RGB` — it is
/// existing, already-verified plumbing (see round-01 log), not new logic, so
/// it is implemented here rather than stubbed.
fn paint_rect(canvas_id: &str, color: (u8, u8, u8)) {
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("window has no document");
    let canvas = document
        .get_element_by_id(canvas_id)
        .unwrap_or_else(|| panic!("no element with id `{canvas_id}`"))
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .expect("element is not an HtmlCanvasElement");
    let ctx = canvas
        .get_context("2d")
        .expect("get_context(\"2d\") failed")
        .expect("canvas has no 2d context")
        .dyn_into::<CanvasRenderingContext2d>()
        .expect("context is not a CanvasRenderingContext2d");

    let (r, g, b) = color;
    ctx.set_fill_style_str(&format!("rgb({r}, {g}, {b})"));
    ctx.fill_rect(RECT_X as f64, RECT_Y as f64, RECT_W as f64, RECT_H as f64);
}

/// Decides which colour the rectangle should be painted at a given tick:
/// `RECT_COLOR_RGB` on even ticks (including tick 0), `RECT_COLOR_RGB_ALT` on
/// odd ticks. Pure — no drawing, no shared state — so it can be unit tested
/// directly without a DOM.
///
/// This is the round's new *decision*, not plumbing: left as a stub for
/// Green.
fn color_for_tick(tick: u32) -> (u8, u8, u8) {
    if tick.is_multiple_of(2) {
        RECT_COLOR_RGB
    } else {
        RECT_COLOR_RGB_ALT
    }
}

/// Paints the rectangle at tick 0. Called once, synchronously, when the wasm
/// module starts (see [`main`]).
#[wasm_bindgen]
pub fn draw(canvas_id: &str) {
    paint_rect(canvas_id, color_for_tick(0));
}

/// Feeds `frame_duration_secs` of real elapsed time to the crate-local
/// [`FixedTimestep`] (`TIMESTEP`) and advances the crate-local tick count
/// (`TICK`) by however many fixed steps the harness reports elapsed — zero,
/// one, or (per the harness's own spiral-of-death-capped accumulator
/// semantics from round 3) occasionally more than one. Returns the tick
/// count *after* advancing.
///
/// This is the round 4 retrofit's core new decision: unlike round 2's ad-hoc
/// counter (which advanced by exactly one on every call regardless of how
/// much time had actually passed), the tick count here only moves as far as
/// real elapsed time honestly earns, and a call with too little elapsed time
/// advances it by zero.
///
/// Deliberately free of any DOM/`web-sys` dependency (unlike
/// [`tick_and_draw`], which wraps this and then repaints) so it is directly
/// unit-testable under `cargo test --lib` — see the `tests` module below and
/// `src/timestep.rs`'s own tests for the same pattern.
///
/// Left as a stub for Green: the decision of how the harness's step count
/// turns into the new tick count is new logic this round, not existing
/// plumbing.
fn advance_tick(frame_duration_secs: Scalar) -> u32 {
    let steps = TIMESTEP.with(|timestep| timestep.borrow_mut().advance(frame_duration_secs));
    TICK.with(|tick| {
        let new_tick = tick.get() + steps;
        tick.set(new_tick);
        new_tick
    })
}

/// Advances the tick count via [`advance_tick`] from `frame_duration_secs` of
/// real elapsed time, repaints the rectangle for the resulting tick via
/// [`color_for_tick`] and [`paint_rect`], and returns the new tick count.
///
/// Called by `www/index.html` once per timer/frame fire, now passing the
/// real elapsed time (in seconds) since the previous call — measured via
/// `performance.now()` deltas in JS — rather than being called on a bare
/// per-tick assumption. This function is thin plumbing (wire
/// [`advance_tick`]'s result into [`paint_rect`]); the actual decision logic
/// lives in [`advance_tick`], which is why it is implemented for real here
/// while [`advance_tick`] is left as a stub.
#[wasm_bindgen]
pub fn tick_and_draw(canvas_id: &str, frame_duration_secs: f32) -> u32 {
    let new_tick = advance_tick(frame_duration_secs);
    paint_rect(canvas_id, color_for_tick(new_tick));
    new_tick
}

/// Renders one tick's frame as a flat RGB8 buffer (`width * height * 3`
/// bytes, row-major, no padding) — the native-binary fallback's equivalent of
/// [`paint_rect`], but pure (no DOM, no canvas) so it works on any target,
/// wasm or native. Fills the whole frame with black, then the rectangle at
/// `(RECT_X, RECT_Y)` in [`color_for_tick`]'s colour for `tick`, using the
/// same coordinate convention pinned above (origin top-left, y grows down).
///
/// M0.3's native fallback (`src/bin/native_viewer.rs`) is the only caller;
/// kept here rather than in the binary so the geometry/colour constants and
/// `color_for_tick` stay the single source of truth for both the wasm and
/// native rendering paths.
pub fn render_frame(tick: u32, width: u32, height: u32) -> Vec<u8> {
    let (r, g, b) = color_for_tick(tick);
    let mut buf = vec![0u8; (width * height * 3) as usize];
    for y in RECT_Y..(RECT_Y + RECT_H).min(height) {
        for x in RECT_X..(RECT_X + RECT_W).min(width) {
            let i = ((y * width + x) * 3) as usize;
            buf[i] = r;
            buf[i + 1] = g;
            buf[i + 2] = b;
        }
    }
    buf
}

/// Paints a `Scenario`'s current grid state to a named canvas element — the
/// "watchable" half of milestone target 2 (round 4 goal 2). Reuses
/// [`paint_rect`]'s established get-canvas-by-id/get-2d-context/draw
/// pattern, but paints a whole grid of cells (via
/// [`render::render_grid_to_rgb8`]) as a single `putImageData` call rather
/// than one `fill_rect` per cell — see [`paint_rgb8_to_canvas`].
///
/// Builds and paints `scenario::stone_and_water_pool()` (round 2's fixture)
/// specifically — the same `Scenario` value round 3's `run_headless`
/// measures, per round 4 goal 5 ("one definition, two consumers"). `cell_px`
/// is the on-screen pixel size of one grid cell (see `src/render.rs`'s
/// module doc comment for why this is decoupled from `Scenario::cell_size`'s
/// world-space unit); the canvas element is resized to fit the rendered
/// image exactly.
#[wasm_bindgen]
pub fn paint_scenario(canvas_id: &str, cell_px: u32) {
    let scenario = scenario::stone_and_water_pool();
    let grid = scenario.build_grid();
    let buf = render::render_grid_to_rgb8(&grid, &scenario.materials, cell_px);
    let (width_px, height_px) = render::render_dimensions_px(&grid, cell_px);
    paint_rgb8_to_canvas(canvas_id, &buf, width_px, height_px);
}

/// Looks up the canvas element by `canvas_id`, resizes it to `width` x
/// `height`, and paints `rgb` (a flat RGB8 buffer, `width * height * 3`
/// bytes — [`render::render_grid_to_rgb8`]'s own output shape) to it via a
/// single `putImageData` call.
///
/// Canvas `ImageData` requires RGBA (`Uint8ClampedArray`), so this expands
/// `rgb` by inserting a fully-opaque alpha byte after every 3 colour bytes
/// before handing it to the canvas — the one piece of new plumbing
/// [`paint_rect`]'s per-rectangle `fill_rect` pattern didn't need.
fn paint_rgb8_to_canvas(canvas_id: &str, rgb: &[u8], width: u32, height: u32) {
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("window has no document");
    let canvas = document
        .get_element_by_id(canvas_id)
        .unwrap_or_else(|| panic!("no element with id `{canvas_id}`"))
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .expect("element is not an HtmlCanvasElement");
    canvas.set_width(width);
    canvas.set_height(height);
    let ctx = canvas
        .get_context("2d")
        .expect("get_context(\"2d\") failed")
        .expect("canvas has no 2d context")
        .dyn_into::<CanvasRenderingContext2d>()
        .expect("context is not a CanvasRenderingContext2d");

    let mut rgba = Vec::with_capacity(rgb.len() / 3 * 4);
    for chunk in rgb.as_chunks::<3>().0 {
        rgba.extend_from_slice(chunk);
        rgba.push(255);
    }
    let image_data = web_sys::ImageData::new_with_u8_clamped_array_and_sh(
        wasm_bindgen::Clamped(&rgba),
        width,
        height,
    )
    .expect("failed to build ImageData");
    ctx.put_image_data(&image_data, 0.0, 0.0)
        .expect("put_image_data failed");
}

/// Runs automatically when the wasm module is instantiated in the browser
/// (see the `#[wasm_bindgen(start)]` attribute). Paints tick 0; the running
/// page (`www/index.html`) is responsible for calling [`tick_and_draw`] on
/// its own timer thereafter.
#[wasm_bindgen(start)]
pub fn main() {
    draw("canvas");
}

// --- Getters: the single source of truth for tests/e2e/canvas_rectangle.test.mjs ---
//
// Plain accessors, no decision-making — implemented for real (not stubbed).
// The JS test calls these on the loaded wasm module instead of re-declaring
// the numbers, which is round 2's fix for the duplication round 1's
// Refactor flagged forward.

#[wasm_bindgen]
pub fn rect_x() -> u32 {
    RECT_X
}

#[wasm_bindgen]
pub fn rect_y() -> u32 {
    RECT_Y
}

#[wasm_bindgen]
pub fn rect_w() -> u32 {
    RECT_W
}

#[wasm_bindgen]
pub fn rect_h() -> u32 {
    RECT_H
}

#[wasm_bindgen]
pub fn rect_color_rgb() -> Vec<u8> {
    vec![RECT_COLOR_RGB.0, RECT_COLOR_RGB.1, RECT_COLOR_RGB.2]
}

#[wasm_bindgen]
pub fn rect_color_rgb_alt() -> Vec<u8> {
    vec![
        RECT_COLOR_RGB_ALT.0,
        RECT_COLOR_RGB_ALT.1,
        RECT_COLOR_RGB_ALT.2,
    ]
}

#[wasm_bindgen]
pub fn tick_interval_ms() -> u32 {
    TICK_INTERVAL_MS
}

// --- Getters: the single source of truth for an e2e test of paint_scenario ---
//
// Same discipline as the block above (round 2's fix for round 1's
// hand-duplicated constants), applied to round 4's scenario renderer: a JS
// test reads a placed material's expected colour straight off the same
// `MaterialTable::reference()` `paint_scenario` itself paints from, rather
// than re-declaring the RGB literals and risking silent drift if the
// reference table's values ever change.

/// The on-screen pixel size of one grid cell `paint_scenario` is called
/// with in `www/scenario.html` — read by the e2e test to compute which
/// canvas pixel to sample for a given grid cell, instead of hardcoding it.
#[wasm_bindgen]
pub fn scenario_cell_px() -> u32 {
    SCENARIO_CELL_PX
}

/// `stone_and_water_pool()`'s material table (`MaterialTable::reference()`)
/// air/water/stone colours, as 8-bit sRGB — the same values
/// [`render::render_grid_to_rgb8`] reads via `Material::colour` when
/// `paint_scenario` paints. `MaterialId`s 0/1/2 respectively, per
/// `src/scenario.rs`'s `stone_and_water_pool` doc comment.
#[wasm_bindgen]
pub fn scenario_air_colour_rgb() -> Vec<u8> {
    material_colour_rgb(0)
}

#[wasm_bindgen]
pub fn scenario_water_colour_rgb() -> Vec<u8> {
    material_colour_rgb(1)
}

#[wasm_bindgen]
pub fn scenario_stone_colour_rgb() -> Vec<u8> {
    material_colour_rgb(2)
}

fn material_colour_rgb(id: u16) -> Vec<u8> {
    let materials = material::MaterialTable::reference();
    let (r, g, b) = materials.get(material::MaterialId::new(id)).colour;
    vec![r, g, b]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Disposable unit test (not a scenario): pins that the rectangle
    /// geometry actually fits inside the canvas declared in
    /// `www/index.html` (200x150). If either side changes, this is the
    /// tripwire that catches the mismatch before it silently clips.
    // Same reasoning as math.rs's up_convention test: clippy sees this as a
    // compile-time-constant assertion and suggests a `const` block, which
    // would turn a future geometry mistake into a compile error instead of a
    // failing test. Kept as a runtime assertion on purpose.
    #[allow(clippy::assertions_on_constants)]
    #[test]
    fn rectangle_fits_within_canvas() {
        const CANVAS_W: u32 = 200;
        const CANVAS_H: u32 = 150;
        assert!(
            RECT_X + RECT_W <= CANVAS_W,
            "rectangle right edge ({}) exceeds canvas width ({CANVAS_W})",
            RECT_X + RECT_W
        );
        assert!(
            RECT_Y + RECT_H <= CANVAS_H,
            "rectangle bottom edge ({}) exceeds canvas height ({CANVAS_H})",
            RECT_Y + RECT_H
        );
    }

    /// Green guard test (already green, pins it so a later round can't
    /// quietly break it): the getters round 2 adds to remove the
    /// Rust/JS constant duplication actually expose the same values the
    /// constants hold, not stale or miscopied ones.
    #[test]
    fn getters_expose_the_same_constants_js_reads() {
        assert_eq!(rect_x(), RECT_X);
        assert_eq!(rect_y(), RECT_Y);
        assert_eq!(rect_w(), RECT_W);
        assert_eq!(rect_h(), RECT_H);
        assert_eq!(
            rect_color_rgb(),
            vec![RECT_COLOR_RGB.0, RECT_COLOR_RGB.1, RECT_COLOR_RGB.2]
        );
        assert_eq!(
            rect_color_rgb_alt(),
            vec![
                RECT_COLOR_RGB_ALT.0,
                RECT_COLOR_RGB_ALT.1,
                RECT_COLOR_RGB_ALT.2
            ]
        );
        assert_eq!(tick_interval_ms(), TICK_INTERVAL_MS);
    }

    /// Disposable unit test (not a scenario): pins the tick-to-colour rule
    /// that makes the rectangle's change visible — even ticks (including
    /// tick 0) get `RECT_COLOR_RGB`, odd ticks get `RECT_COLOR_RGB_ALT`.
    /// Currently red: `color_for_tick` is a stub.
    /// M0.3 (native fallback): `render_frame` paints the rectangle at the
    /// pinned geometry in `color_for_tick`'s colour for that tick, and
    /// leaves the rest of the frame black — pins the pure buffer format
    /// `src/bin/native_viewer.rs` relies on, independent of any PNG codec.
    #[test]
    fn render_frame_paints_rect_in_tick_colour() {
        let (w, h) = (200u32, 150u32);
        let frame = render_frame(0, w, h);
        assert_eq!(frame.len(), (w * h * 3) as usize);
        let px = |x: u32, y: u32| {
            let i = ((y * w + x) * 3) as usize;
            (frame[i], frame[i + 1], frame[i + 2])
        };
        assert_eq!(px(RECT_X, RECT_Y), RECT_COLOR_RGB, "tick 0 rect pixel");
        assert_eq!(px(0, 0), (0, 0, 0), "outside the rect should stay black");
        let frame1 = render_frame(1, w, h);
        let px1 = |x: u32, y: u32| {
            let i = ((y * w + x) * 3) as usize;
            (frame1[i], frame1[i + 1], frame1[i + 2])
        };
        assert_eq!(px1(RECT_X, RECT_Y), RECT_COLOR_RGB_ALT, "tick 1 rect pixel");
    }

    #[test]
    fn color_for_tick_alternates_by_parity() {
        assert_eq!(color_for_tick(0), RECT_COLOR_RGB, "tick 0 should be the base colour");
        assert_eq!(color_for_tick(1), RECT_COLOR_RGB_ALT, "tick 1 should be the alt colour");
        assert_eq!(color_for_tick(2), RECT_COLOR_RGB, "tick 2 should be back to the base colour");
        assert_eq!(color_for_tick(3), RECT_COLOR_RGB_ALT, "tick 3 should be the alt colour");
    }

    // --- Round 4 retrofit: advance_tick (the FixedTimestep-driven replacement
    // for round 2's ad-hoc `TICK += 1` per call). No browser/DOM involved —
    // these exercise advance_tick directly, the same way src/timestep.rs's
    // own tests exercise FixedTimestep directly. Currently red: advance_tick
    // is a todo!() stub. ---

    /// Scenario (durable, restates round 4 goal 1's "a duration less than one
    /// dt does not advance the tick"): a call carrying less than one fixed
    /// step's worth of real elapsed time must not advance the tick count —
    /// proof this is now honest elapsed-time accounting, not a bare +1 per
    /// call.
    #[test]
    fn advance_tick_with_duration_under_one_dt_does_not_advance_the_tick() {
        let tick = advance_tick(DT_SECONDS / 2.0);
        assert_eq!(
            tick, 0,
            "half a dt's worth of elapsed time should not yet advance the tick count"
        );
    }

    /// Scenario (durable, restates round 4 goal 1's "several dts advances the
    /// tick count correctly and the colour matches the new parity"): a single
    /// call carrying several fixed steps' worth of real elapsed time advances
    /// the tick count by that many steps in one call (not by 1), and
    /// `color_for_tick` of the resulting tick is what JS/the e2e test would
    /// see painted.
    #[test]
    fn advance_tick_with_several_dts_advances_by_that_many_steps_and_colour_matches_parity() {
        let tick = advance_tick(DT_SECONDS * 3.0);
        assert_eq!(
            tick, 3,
            "three dts' worth of elapsed time in one call should advance the tick by 3, not 1"
        );
        assert_eq!(
            color_for_tick(tick),
            RECT_COLOR_RGB_ALT,
            "tick 3 is odd, so the alt colour should be what gets painted"
        );
    }

    /// Scenario (durable, restates round 4 goal 1's "0, 1, or occasionally
    /// more than 1 step per call, per the harness's own accumulator
    /// semantics"): real elapsed time arriving in several small,
    /// sub-dt-sized calls — the same pattern `www/index.html`'s
    /// `setInterval` produces — still accumulates correctly across calls: no
    /// steps until enough real time has actually passed, and exactly one
    /// once it has.
    #[test]
    fn advance_tick_accumulates_partial_durations_across_calls_until_a_full_dt_elapses() {
        let half_dt = DT_SECONDS / 2.0;
        assert_eq!(
            advance_tick(half_dt),
            0,
            "first half-dt call should not yet advance the tick"
        );
        assert_eq!(
            advance_tick(half_dt),
            1,
            "second half-dt call completes one full dt, so the tick should now be 1"
        );
    }

    /// Green guard test (pins a relationship, not a behaviour): `DT_SECONDS`
    /// is `TICK_INTERVAL_MS` converted to seconds, not an independently
    /// chosen number — the fixed-step cadence must match the cadence
    /// `www/index.html` and the e2e test already agree on via
    /// `tick_interval_ms()`.
    #[test]
    fn dt_seconds_matches_tick_interval_ms_converted_to_seconds() {
        let expected = TICK_INTERVAL_MS as Scalar / 1000.0;
        assert!(
            (DT_SECONDS - expected).abs() < 1e-6,
            "expected DT_SECONDS ({DT_SECONDS}) to equal TICK_INTERVAL_MS/1000 ({expected})"
        );
    }
}
