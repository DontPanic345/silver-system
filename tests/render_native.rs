//! Headless empirical check for the grid renderer, native/PNG path: runs
//! the actual compiled `native_viewer` binary as a subprocess (not just
//! calling `render::render_grid_to_rgb8` in-process — that would only prove
//! the pure function, not that the binary really writes a readable file),
//! decodes the `scenario.png` it writes, and reads real pixel bytes back —
//! the same discipline `tests/native_fallback.rs` already applies to the
//! rectangle, mirrored here rather than reinvented.
//!
//! Kept as a separate file rather than added to `tests/native_fallback.rs`:
//! the existing `tests/native_fallback.rs` stays unmodified, proving the
//! rectangle path still works untouched — this is a second, parallel test
//! file, not a change to that one.
//!
//! Asserts specific pixels match specific materials' `Material::colour`
//! values for `stone_and_water_pool()` — the same `Scenario` value
//! `run_headless` measures.

use std::process::Command;

use viewer::material::MaterialId;
use viewer::render::render_dimensions_px;
use viewer::scenario::stone_and_water_pool;
use viewer::SCENARIO_CELL_PX;

#[test]
fn native_binary_writes_scenario_png_with_the_right_material_pixels() {
    let out_dir = std::env::temp_dir().join(format!(
        "silver-system-render-native-test-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&out_dir);

    let bin = env!("CARGO_BIN_EXE_native_viewer");
    let status = Command::new(bin)
        .arg(&out_dir)
        .status()
        .expect("failed to run native_viewer binary");
    assert!(status.success(), "native_viewer exited non-zero");

    let path = out_dir.join("scenario.png");
    let img = image::open(&path)
        .unwrap_or_else(|e| panic!("failed to decode {path:?}: {e}"))
        .to_rgb8();

    // `viewer::SCENARIO_CELL_PX` is the single source of truth both
    // `src/bin/native_viewer.rs` and this test read — no restated literal.
    const CELL_PX: u32 = SCENARIO_CELL_PX;

    let scenario = stone_and_water_pool();
    let grid = scenario.build_grid();
    let (width_px, height_px) = render_dimensions_px(&grid, CELL_PX);
    assert_eq!(
        (img.width(), img.height()),
        (width_px, height_px),
        "scenario.png dimensions should match render_dimensions_px for the \
         same scenario/cell_px this test samples against"
    );

    let air_colour = scenario.materials.get(MaterialId::new(0)).colour;
    let water_colour = scenario.materials.get(MaterialId::new(1)).colour;
    let stone_colour = scenario.materials.get(MaterialId::new(2)).colour;

    // Fixture layout (src/scenario.rs's stone_and_water_pool): 6x4 grid,
    // stone lump at grid (0,0)/(1,1) etc (j in 0..2, i in 0..2), water pool
    // at (5,0)/(5,1), rest air. Image row is flipped (grid j=height-1-j),
    // so grid j=0 -> image row (4-1-0)=3 -> pixel y = 3*CELL_PX + a few.
    let pixel_at = |img: &image::RgbImage, grid_i: u32, grid_j: u32| -> (u8, u8, u8) {
        let image_row = 3 - grid_j; // height (4) - 1 - grid_j
        let x = grid_i * CELL_PX + CELL_PX / 2;
        let y = image_row * CELL_PX + CELL_PX / 2;
        let px = img.get_pixel(x, y);
        (px[0], px[1], px[2])
    };

    assert_eq!(pixel_at(&img, 0, 0), stone_colour, "stone lump pixel (0,0)");
    assert_eq!(pixel_at(&img, 1, 1), stone_colour, "stone lump pixel (1,1)");
    assert_eq!(pixel_at(&img, 5, 0), water_colour, "water pool pixel (5,0)");
    assert_eq!(pixel_at(&img, 5, 1), water_colour, "water pool pixel (5,1)");
    assert_eq!(
        pixel_at(&img, 3, 3),
        air_colour,
        "background air pixel (3,3)"
    );

    let _ = std::fs::remove_dir_all(&out_dir);
}
