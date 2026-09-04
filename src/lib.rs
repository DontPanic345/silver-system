//! M0.1 toolchain proving ground.
//!
//! Round 1 proved the chain from `cargo build --target wasm32-unknown-unknown`
//! through `wasm-bindgen` to a real browser canvas: a single static coloured
//! rectangle. Round 2's job is to prove the chain also carries a *running*
//! program — the rectangle must visibly change once per fixed tick — and to
//! remove the hand-duplicated constants Round 1's Refactor flagged forward.
//!
//! Rendering approach unchanged from round 1: canvas 2D via
//! `wasm-bindgen`/`web-sys`. See `cycle-log/tranche-0/m0.1/round-01.md` for
//! why.
//!
//! ## Round 2 tick mechanism (decided in Red; Green fills in the marked stubs)
//!
//! JS owns the wall-clock timer (a plain `setInterval` in `www/index.html`);
//! Rust owns the tick counter and the decision of what the rectangle should
//! look like at a given tick. Every timer fire calls the exported
//! [`tick_and_draw`], which is expected to advance a crate-local counter,
//! decide the colour for the new tick via [`color_for_tick`], repaint via
//! [`paint_rect`], and return the new count so JS (and the headless test) can
//! observe it advancing. This is explicitly NOT the shared M0.4
//! fixed-timestep harness — that harness doesn't exist yet and will retrofit
//! this crate later; this is a small, local, throwaway-if-need-be counter.
//!
//! ## Single source of truth for shared constants (round 2, goal 2)
//!
//! Round 1 left the expected colour/coordinate values hand-duplicated as
//! literals in both this file and `tests/e2e/canvas_rectangle.test.mjs`. Fixed
//! here by exporting plain `#[wasm_bindgen]` getter functions for every value
//! the JS test needs (`rect_x`, `rect_y`, `rect_w`, `rect_h`,
//! `rect_color_rgb`, `rect_color_rgb_alt`, `tick_interval_ms`). The JS test
//! calls these on the already-loaded wasm module at runtime instead of
//! re-declaring the numbers — there is exactly one place these values are
//! written down. The getters are plumbing (a return statement, no decision to
//! make), so they are implemented for real here rather than stubbed; the
//! genuinely new *logic* (which colour a given tick gets, how the counter
//! advances) is left as `todo!()` for Green.

use std::cell::Cell;

// M0.4: shared math primitives (Scalar, Vec2). Not used by this file's own
// canvas logic yet — see src/math.rs for what it is and why.
mod math;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::CanvasRenderingContext2d;

thread_local! {
    /// Crate-local tick counter driving [`tick_and_draw`]. Wasm in a browser
    /// tab is single-threaded, so a `thread_local!` `Cell` is sufficient —
    /// this is explicitly the small, local, throwaway-if-need-be counter
    /// described in the module doc comment, not the shared M0.4
    /// fixed-timestep harness.
    static TICK: Cell<u32> = const { Cell::new(0) };
}

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
    if tick % 2 == 0 {
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

/// Advances the crate-local tick counter by one, repaints the rectangle for
/// the new tick via [`color_for_tick`] and [`paint_rect`], and returns the
/// new tick count.
///
/// Called by `www/index.html` on a fixed `setInterval` (period
/// [`TICK_INTERVAL_MS`], read via [`tick_interval_ms`]) — this function is
/// what turns the artifact into proof of a *running* program rather than a
/// single paint call. The tick counter itself (its storage — a `Cell`,
/// `AtomicU32`, or similar — is Green's choice) is crate-local state, not the
/// shared M0.4 fixed-timestep harness.
///
/// Left as a stub for Green: both the counter's storage and the
/// increment-then-paint sequence are new logic this round, not existing
/// plumbing.
#[wasm_bindgen]
pub fn tick_and_draw(canvas_id: &str) -> u32 {
    let new_tick = TICK.with(|t| {
        let new_tick = t.get() + 1;
        t.set(new_tick);
        new_tick
    });
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Disposable unit test (not a scenario): pins that the rectangle
    /// geometry actually fits inside the canvas declared in
    /// `www/index.html` (200x150). If either side changes, this is the
    /// tripwire that catches the mismatch before it silently clips.
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
}
