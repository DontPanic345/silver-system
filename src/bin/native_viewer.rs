//! M0.3 — the fallback. If wasm-in-the-browser had not worked, this native
//! binary is plan B: it renders the same rectangle-per-tick as
//! `src/lib.rs`'s wasm path, but to real PNG files on disk instead of a
//! canvas, using [`viewer::render_frame`] so both paths share one source of
//! truth for geometry and colour.
//!
//! Pages (M0.2) is live and working, so this is *not* the path in use — see
//! `README.md`. It exists, proven and exercised, so it doesn't have to be
//! discovered-and-built under pressure if the web path ever breaks.
//!
//! Usage: `cargo run --bin native_viewer -- <out-dir>` (defaults to
//! `native-fallback-out/` in the current directory). Writes `tick-0.png`,
//! `tick-1.png`, `tick-2.png` — enough ticks for the three-sample check
//! `tests/native_fallback.rs` runs (two samples can't tell "advancing" from
//! "stuck at tick 1", per M0.1's round 2 finding).

use std::path::PathBuf;

use viewer::render_frame;

const WIDTH: u32 = 200;
const HEIGHT: u32 = 150;
const TICKS_TO_RENDER: u32 = 3;

fn main() {
    let out_dir: PathBuf = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("native-fallback-out"));
    std::fs::create_dir_all(&out_dir)
        .unwrap_or_else(|e| panic!("failed to create output dir {out_dir:?}: {e}"));

    for tick in 0..TICKS_TO_RENDER {
        let buf = render_frame(tick, WIDTH, HEIGHT);
        let path = out_dir.join(format!("tick-{tick}.png"));
        image::save_buffer(&path, &buf, WIDTH, HEIGHT, image::ColorType::Rgb8)
            .unwrap_or_else(|e| panic!("failed to write {path:?}: {e}"));
        println!("wrote {}", path.display());
    }
}
