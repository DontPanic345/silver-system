//! Proves the native fallback end to end: runs the actual compiled
//! `native_viewer` binary as a subprocess (not just calling `render_frame`
//! in-process — that would only prove the pure function, not that the
//! binary really writes readable files), then decodes the PNGs it wrote and
//! reads real pixel bytes back, the same "read real output data, don't take
//! it on faith" discipline `tests/e2e/canvas_rectangle.test.mjs` applies to
//! the browser canvas.
//!
//! Three samples (ticks 0, 1, 2), not two — a two-sample "differs from tick
//! 0" check can't tell a genuinely advancing sequence from one that changed
//! once and froze.

use std::process::Command;

use viewer::{RECT_COLOR_RGB, RECT_COLOR_RGB_ALT, RECT_X, RECT_Y};

#[test]
fn native_binary_writes_pngs_with_the_right_pixels_at_three_ticks() {
    let out_dir = std::env::temp_dir().join(format!(
        "silver-system-native-fallback-test-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&out_dir);

    let bin = env!("CARGO_BIN_EXE_native_viewer");
    let status = Command::new(bin)
        .arg(&out_dir)
        .status()
        .expect("failed to run native_viewer binary");
    assert!(status.success(), "native_viewer exited non-zero");

    let expected = [RECT_COLOR_RGB, RECT_COLOR_RGB_ALT, RECT_COLOR_RGB];
    for (tick, expected_rgb) in expected.iter().enumerate() {
        let path = out_dir.join(format!("tick-{tick}.png"));
        let img = image::open(&path)
            .unwrap_or_else(|e| panic!("failed to decode {path:?}: {e}"))
            .to_rgb8();
        let px = img.get_pixel(RECT_X, RECT_Y);
        assert_eq!(
            (px[0], px[1], px[2]),
            *expected_rgb,
            "tick {tick}: rect pixel at ({RECT_X},{RECT_Y}) in {path:?}"
        );
    }

    let _ = std::fs::remove_dir_all(&out_dir);
}
