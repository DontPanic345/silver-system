//! M0.1 toolchain proving ground.
//!
//! Round goal: prove the chain from `cargo build --target wasm32-unknown-unknown`
//! through `wasm-bindgen` to a real browser canvas actually works on this
//! machine. This crate's only job, for this round, is to draw a single
//! coloured rectangle onto an HTML `<canvas>` element.
//!
//! Rendering approach: canvas 2D via `wasm-bindgen`/`web-sys`, chosen over a
//! WebGL/WebGPU crate because it needs no GPU context negotiation and the
//! plain toolchain (rustup wasm32 target, crates.io access, `wasm-bindgen-cli`
//! matching the `wasm-bindgen` crate version) was confirmed to work end to end
//! during this round — see `cycle-log/tranche-0/m0.1/round-01.md` for the
//! exact commands and their output. If a later round needs the extra power of
//! WebGL/WebGPU, that is a fresh decision against what actually builds then,
//! not a revision of this one.

use wasm_bindgen::prelude::*;

/// The colour Green's implementation must paint the rectangle, as 8-bit sRGB.
/// Pinned here so Red's test and Green's implementation agree on what
/// "success" looks like without either having to guess at the other's
/// number. Kept in sync by hand with `tests/e2e/canvas_rectangle.test.mjs`
/// until there is a shared source of truth (see round log, "Compromises").
pub const RECT_COLOR_RGB: (u8, u8, u8) = (200, 60, 60);

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
/// `RECT_COLOR_RGB`.
///
/// This is the function the host page (`www/index.html`) calls once the wasm
/// module has loaded. Green fills in the actual `web_sys` calls; this stub
/// only proves the crate compiles for `wasm32-unknown-unknown` and the
/// `wasm-bindgen` export shape (name, signature, visibility) is right for the
/// JS glue that `wasm-bindgen-cli` generates.
#[wasm_bindgen]
pub fn draw(canvas_id: &str) {
    let _ = canvas_id;
    todo!(
        "look up document.getElementById(canvas_id) as HtmlCanvasElement, \
         get_context(\"2d\") as CanvasRenderingContext2d, set_fill_style to \
         RECT_COLOR_RGB, and fill_rect the RECT_* constants"
    )
}

/// Runs automatically when the wasm module is instantiated in the browser
/// (see the `#[wasm_bindgen(start)]` attribute). Calls [`draw`] against the
/// host page's well-known canvas element id, `"canvas"`.
#[wasm_bindgen(start)]
pub fn main() {
    draw("canvas");
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
}
