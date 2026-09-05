//! The native fallback. If wasm-in-the-browser had not worked, this native
//! binary is plan B: it renders the same rectangle-per-tick as
//! `src/lib.rs`'s wasm path, but to real PNG files on disk instead of a
//! canvas, using [`viewer::render_frame`] so both paths share one source of
//! truth for geometry and colour.
//!
//! The wasm/browser path is live and working, so this is *not* the path in
//! use — see `README.md`. It exists, proven and exercised, so it doesn't
//! have to be discovered-and-built under pressure if the web path ever
//! breaks.
//!
//! Usage: `cargo run --bin native_viewer -- <out-dir>` (defaults to
//! `native-fallback-out/` in the current directory). Writes `tick-0.png`,
//! `tick-1.png`, `tick-2.png` — enough ticks for the three-sample check
//! `tests/native_fallback.rs` runs (two samples can't tell "advancing" from
//! "stuck at tick 1") — and `scenario.png`:
//! `scenario::stone_and_water_pool()`'s current grid state rendered via
//! `viewer::render::render_grid_to_rgb8`, the native-binary fallback path
//! for the grid renderer. Both share this one binary rather than splitting
//! into two, since both are "render something pure to a PNG on disk" and
//! the rectangle path already proved the `image` crate plumbing this reuses
//! unchanged.

use std::path::PathBuf;

use viewer::material::MaterialTable;
use viewer::render::render_dimensions_px;
use viewer::render_frame;
use viewer::scenario::stone_and_water_pool;
use viewer::timestep::FixedTimestep;
use viewer::{render, scenario, SCENARIO_CELL_PX};

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

    // The same `stone_and_water_pool()` Scenario the headless runner
    // measures, painted here via the pure grid-to-pixels function — "one
    // definition, two consumers" demonstrated concretely against the
    // native fallback path.
    let scenario: scenario::Scenario = stone_and_water_pool();
    let grid = scenario.build_grid();
    let buf = render::render_grid_to_rgb8(&grid, &scenario.materials, SCENARIO_CELL_PX);
    let (width_px, height_px) = render_dimensions_px(&grid, SCENARIO_CELL_PX);
    let path = out_dir.join("scenario.png");
    image::save_buffer(&path, &buf, width_px, height_px, image::ColorType::Rgb8)
        .unwrap_or_else(|e| panic!("failed to write {path:?}: {e}"));
    println!("wrote {}", path.display());

    // scenario::physics_demo() (see its own doc comment) stepped forward
    // under real gravity/density physics (src/grid.rs) and snapshotted at a
    // few points, so the movement rule's actual effect (grains falling,
    // sinking through the water, the pool re-levelling) is visible in a
    // sequence of real PNGs on disk, not just proven by the headless
    // assertions in src/scenario.rs's own tests.
    let physics_scenario = scenario::physics_demo();
    let materials: MaterialTable = physics_scenario.materials;
    let mut physics_grid = viewer::grid::Grid::new(
        physics_scenario.width,
        physics_scenario.height,
        physics_scenario.background,
    );
    for &(index, id) in &physics_scenario.placements {
        physics_grid.set(index, id);
    }
    let mut timestep = FixedTimestep::new(1.0 / 30.0);
    let mut steps_so_far = 0u32;
    for target_ticks in [0u32, 60, 150, 300] {
        while steps_so_far < target_ticks {
            steps_so_far += physics_grid.step(&mut timestep, 1.0 / 30.0, &materials);
        }
        let buf = render::render_grid_to_rgb8(&physics_grid, &materials, SCENARIO_CELL_PX);
        let (width_px, height_px) = render_dimensions_px(&physics_grid, SCENARIO_CELL_PX);
        let path = out_dir.join(format!("physics-demo-tick-{target_ticks}.png"));
        image::save_buffer(&path, &buf, width_px, height_px, image::ColorType::Rgb8)
            .unwrap_or_else(|e| panic!("failed to write {path:?}: {e}"));
        println!("wrote {}", path.display());
    }
}
