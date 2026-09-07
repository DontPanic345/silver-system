//! Grid-to-pixels rendering: the watchable counterpart to `measure.rs`'s
//! headless runner. That runner is `Scenario`'s first consumer (measures
//! rather than draws); this module is the second — a renderer that paints
//! the *same* `Scenario` value's grid state to pixels, no browser/DOM
//! dependency, so it works identically from a native binary and (via a thin
//! wrapper in `src/lib.rs`) from wasm.
//!
//! Mirrors `src/lib.rs`'s `render_frame` shape on purpose (pure buffer
//! function, thin wasm wrapper, thin native wrapper, headless pixel-reading
//! test) rather than inventing a parallel rendering pattern — see that
//! function's doc comment.
//!
//! ## Coordinate flip: `Grid`'s own flagged note
//!
//! `src/grid.rs`'s module doc comment flags this exact gap in advance:
//! increasing `j` moves *forward* in the grid's flat storage and is world
//! `+y` (up), but a canvas/image row index increases *downward*. This
//! module is where that flip actually happens: [`render_grid_to_rgb8`] paints
//! grid row `j == height - 1` (the topmost, largest-`j` row) as image row 0
//! (the top of the buffer), and grid row `j == 0` as the bottom image row —
//! explicit, not incidental.
//!
//! ## Cell size in pixels, not world units
//!
//! `Scenario::cell_size` (a `Scalar`) is the cell's size in *world* units,
//! used for physics (`GridIndex::center`) — this renderer takes a separate
//! `cell_px: u32`, the on-screen pixel size of one cell, deliberately
//! decoupled from the physics cell size. No world-to-screen camera/zoom
//! exists yet (no overlays, camera, or interaction), so a flat integer
//! pixels-per-cell is the simplest thing that paints a whole grid visibly.

use crate::grid::Grid;
use crate::material::MaterialTable;
use crate::math::GridIndex;

/// The pixel dimensions a grid renders to at a given `cell_px`: exactly
/// `grid.width() * cell_px` by `grid.height() * cell_px`, with no border or
/// padding. [`render_grid_to_rgb8`]'s buffer is always exactly this size —
/// callers (the wasm canvas wrapper, the native PNG writer) use this to size
/// the canvas/image they hand the buffer to, rather than recomputing it
/// themselves.
///
/// Plain arithmetic, no decision to make, so implemented directly rather
/// than stubbed — same reasoning `src/math.rs`'s `Vec2::new` gives for its
/// own plumbing.
pub fn render_dimensions_px(grid: &Grid, cell_px: u32) -> (u32, u32) {
    (
        grid.width() as u32 * cell_px,
        grid.height() as u32 * cell_px,
    )
}

/// Renders `grid`'s **current** state to a flat RGB8 buffer (`width_px *
/// height_px * 3` bytes, row-major, no padding, no alpha), each grid cell's
/// material colour ([`crate::material::Material::colour`]) filling an exact
/// `cell_px * cell_px` square of pixels. `width_px`/`height_px` are always
/// [`render_dimensions_px`]`(grid, cell_px)` — see that function.
///
/// Pure: no DOM, no wasm-bindgen, no canvas — callable identically from a
/// native binary ([`crate::render`]'s only expected native caller,
/// `src/bin/native_viewer.rs`) and, via a thin `#[wasm_bindgen]` wrapper in
/// `src/lib.rs`, from a browser.
///
/// See the module doc comment for the top/bottom row flip this function
/// applies (grid `j == height - 1` paints image row 0), and why `cell_px` is
/// screen pixels per cell, not `Scenario::cell_size`'s world-space unit.
///
/// Implemented directly since it composes only already-proven
/// `Grid`/`MaterialTable` reads, the same "short, direct composition of
/// proven pieces" reasoning `Scenario::build_grid` gives for its own
/// directly-implemented status — but the coordinate flip and per-cell fill
/// are genuinely new decisions, so they're exercised directly by this
/// module's own tests rather than assumed correct.
pub fn render_grid_to_rgb8(grid: &Grid, materials: &MaterialTable, cell_px: u32) -> Vec<u8> {
    let (width_px, height_px) = render_dimensions_px(grid, cell_px);
    let mut buf = vec![0u8; (width_px * height_px * 3) as usize];

    for j in 0..grid.height() {
        // The flip: grid row j == height-1 (topmost, largest j, world +y up)
        // becomes image row 0 (top of the buffer, since image rows grow
        // downward) — see the module doc comment.
        let image_row = grid.height() - 1 - j;
        for i in 0..grid.width() {
            let id = grid.get(GridIndex::new(i as i32, j as i32));
            let (r, g, b) = materials.get(id).colour;

            let base_x = i as u32 * cell_px;
            let base_y = image_row as u32 * cell_px;
            for dy in 0..cell_px {
                let y = base_y + dy;
                let row_start = (y * width_px * 3) as usize;
                for dx in 0..cell_px {
                    let x = base_x + dx;
                    let idx = row_start + (x * 3) as usize;
                    buf[idx] = r;
                    buf[idx + 1] = g;
                    buf[idx + 2] = b;
                }
            }
        }
    }

    buf
}

/// Renders a [`World`](crate::world::World) — with its gnomes — to a flat
/// RGB8 buffer, the same shape [`render_grid_to_rgb8`] produces.
///
/// Two things this does that the plain grid renderer cannot, because a
/// `Grid` has nowhere to put them:
///
/// - **Temperature is visible.** Each cell's material colour is shifted
///   toward red when it is hotter than a comfortable room and toward blue
///   when colder, so a heat gradient reads at a glance. Without it the jar
///   looks static even while the whole interesting half of the simulation
///   is happening.
/// - **Gnomes are drawn over the world**, not in it. A gnome is an agent at
///   a cell, not a material occupying one, so it is painted on top; an
///   ethereal gnome is drawn as a faint ghost at its anchor, which is what
///   makes "go and rescue them" legible.
pub fn render_world_to_rgb8(
    world: &crate::world::World,
    colony: &crate::gnome::Colony,
    cell_px: u32,
) -> Vec<u8> {
    let width_px = world.width() as u32 * cell_px;
    let height_px = world.height() as u32 * cell_px;
    let mut buf = vec![0u8; (width_px * height_px * 3) as usize];

    let put = |buf: &mut Vec<u8>, x: u32, y: u32, c: (u8, u8, u8)| {
        if x >= width_px || y >= height_px {
            return;
        }
        let idx = ((y * width_px + x) * 3) as usize;
        buf[idx] = c.0;
        buf[idx + 1] = c.1;
        buf[idx + 2] = c.2;
    };

    for j in 0..world.height() {
        let image_row = world.height() - 1 - j;
        for i in 0..world.width() {
            let index = GridIndex::new(i as i32, j as i32);
            let cell = world.cell(index);
            let material = world.materials().get(cell.material);
            let base = shade_by_pressure(material, cell.mass);
            let colour = tint_by_temperature(base, cell.temperature);
            for dy in 0..cell_px {
                for dx in 0..cell_px {
                    put(
                        &mut buf,
                        i as u32 * cell_px + dx,
                        image_row as u32 * cell_px + dy,
                        colour,
                    );
                }
            }
        }
    }

    for gnome in &colony.gnomes {
        let (pos, colour) = if gnome.is_embodied() {
            (gnome.pos, (245, 225, 140))
        } else {
            (gnome.anchor, (150, 120, 210))
        };
        if !world.in_bounds(pos) {
            continue;
        }
        let image_row = world.height() - 1 - pos.j as usize;
        let inset = (cell_px / 4).max(1);
        for dy in inset..cell_px.saturating_sub(inset).max(inset + 1) {
            for dx in inset..cell_px.saturating_sub(inset).max(inset + 1) {
                put(
                    &mut buf,
                    pos.i as u32 * cell_px + dx,
                    image_row as u32 * cell_px + dy,
                    colour,
                );
            }
        }
    }

    buf
}

/// Brightens a gas cell in proportion to how much gas is packed into it.
///
/// Gases are the only cells whose mass is genuinely variable (see
/// `src/gas.rs`), and without this the most interesting thing in a gas
/// scene — where the pressure is — is invisible: thin air and a
/// ten-atmosphere bottle paint the identical dark blue. The scale is
/// relative to the material's own density, i.e. to one atmosphere, so it is
/// data-driven and says nothing about which gas it is. Non-gases are
/// returned untouched, since for them mass and density are the same number
/// by construction and the shading would always be a no-op anyway.
fn shade_by_pressure(
    material: &crate::material::Material,
    mass: crate::math::Scalar,
) -> (u8, u8, u8) {
    if material.phase != crate::material::Phase::Gas || material.density <= 0.0 {
        return material.colour;
    }
    let atmospheres = (mass / material.density).clamp(0.0, 6.0);
    let lift = ((atmospheres - 1.0) * 22.0).clamp(-14.0, 110.0);
    let add = |c: u8| (c as f32 + lift).clamp(0.0, 255.0) as u8;
    (
        add(material.colour.0),
        add(material.colour.1),
        add(material.colour.2),
    )
}

/// Shifts a material colour warm or cool according to `temperature_k`.
/// Neutral at 293 K, fully warm by 800 K, fully cool by 200 K.
fn tint_by_temperature(base: (u8, u8, u8), temperature_k: crate::math::Scalar) -> (u8, u8, u8) {
    let t = temperature_k;
    let mix = |a: u8, b: u8, f: f32| (a as f32 + (b as f32 - a as f32) * f).clamp(0.0, 255.0) as u8;
    if t > 293.0 {
        let f = ((t - 293.0) / 507.0).clamp(0.0, 1.0) * 0.75;
        (mix(base.0, 255, f), mix(base.1, 90, f), mix(base.2, 30, f))
    } else {
        let f = ((293.0 - t) / 93.0).clamp(0.0, 1.0) * 0.5;
        (mix(base.0, 90, f), mix(base.1, 140, f), mix(base.2, 255, f))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::{Material, MaterialId, Phase};
    use crate::scenario::stone_and_water_pool;

    fn px(buf: &[u8], width_px: u32, x: u32, y: u32) -> (u8, u8, u8) {
        let i = ((y * width_px + x) * 3) as usize;
        (buf[i], buf[i + 1], buf[i + 2])
    }

    // --- Scenario: a pure grid-to-pixels function ---

    /// Scenario: rendering a small grid with two distinct, known materials
    /// at a known `cell_px` produces a buffer of exactly the expected size,
    /// with every pixel in each cell's square painted that cell's material
    /// colour — the round's core "pure buffer function" claim.
    #[test]
    fn render_grid_paints_each_cells_colour_across_its_full_pixel_square() {
        let red = Material::new(1.0, 0.0, 1.0, 0.0, Phase::Solid, (200, 0, 0));
        let blue = Material::new(1.0, 0.0, 1.0, 0.0, Phase::Solid, (0, 0, 200));
        let materials = MaterialTable::new(vec![red, blue]);
        let red_id = MaterialId::new(0);
        let blue_id = MaterialId::new(1);

        let mut grid = Grid::new(2, 1, red_id);
        grid.set(GridIndex::new(1, 0), blue_id);

        let cell_px = 3;
        let (width_px, height_px) = render_dimensions_px(&grid, cell_px);
        assert_eq!((width_px, height_px), (6, 3));

        let buf = render_grid_to_rgb8(&grid, &materials, cell_px);
        assert_eq!(buf.len(), (width_px * height_px * 3) as usize);

        // Cell (0,0) is red, occupies image x in [0,3), y in [0,3).
        for y in 0..3 {
            for x in 0..3 {
                assert_eq!(
                    px(&buf, width_px, x, y),
                    (200, 0, 0),
                    "cell (0,0) pixel ({x},{y})"
                );
            }
        }
        // Cell (1,0) is blue, occupies image x in [3,6), y in [0,3).
        for y in 0..3 {
            for x in 3..6 {
                assert_eq!(
                    px(&buf, width_px, x, y),
                    (0, 0, 200),
                    "cell (1,0) pixel ({x},{y})"
                );
            }
        }
    }

    // --- Scenario: the coordinate flip pinned explicitly (grid.rs's flagged note) ---

    /// Scenario: a two-row grid where row j=0 and row j=1 hold visibly
    /// different materials renders with j=1 (the larger-j, "up" in world
    /// space) as the *top* image row and j=0 as the *bottom* image row —
    /// pins the exact flip `src/grid.rs`'s module doc comment names in
    /// advance, so a silent unflipped implementation (which would still pass
    /// the single-row test above) is caught here.
    #[test]
    fn larger_j_row_renders_as_the_top_image_row_not_the_bottom() {
        let bottom_colour = Material::new(1.0, 0.0, 1.0, 0.0, Phase::Solid, (10, 10, 10));
        let top_colour = Material::new(1.0, 0.0, 1.0, 0.0, Phase::Solid, (250, 250, 250));
        let materials = MaterialTable::new(vec![bottom_colour, top_colour]);
        let bottom_id = MaterialId::new(0);
        let top_id = MaterialId::new(1);

        let mut grid = Grid::new(1, 2, bottom_id);
        grid.set(GridIndex::new(0, 1), top_id); // j=1: world-space "up"

        let cell_px = 1;
        let buf = render_grid_to_rgb8(&grid, &materials, cell_px);
        let (width_px, _) = render_dimensions_px(&grid, cell_px);

        assert_eq!(
            px(&buf, width_px, 0, 0),
            (250, 250, 250),
            "j=1 (world +y, up) should render as image row 0 (the top)"
        );
        assert_eq!(
            px(&buf, width_px, 0, 1),
            (10, 10, 10),
            "j=0 should render as image row 1 (the bottom), not the top"
        );
    }

    // --- Scenario: one definition, two consumers, demonstrated concretely ---

    /// Scenario: the exact same `stone_and_water_pool()` `Scenario` value
    /// `run_headless` measures is the one rendered here — pins
    /// that a stone-placed cell and a water-placed cell (per that fixture's
    /// documented layout) paint the reference table's own stone/water
    /// colours, i.e. this is genuinely reading the same scenario's grid and
    /// material table, not a lookalike shape invented for this test.
    #[test]
    fn renders_the_same_scenario_value_run_headless_measures() {
        let scenario = stone_and_water_pool();
        let grid = scenario.build_grid();
        let cell_px = 2;

        let buf = render_grid_to_rgb8(&grid, &scenario.materials, cell_px);
        let (width_px, _) = render_dimensions_px(&grid, cell_px);

        let stone_colour = scenario.materials.get(MaterialId::new(2)).colour;
        let water_colour = scenario.materials.get(MaterialId::new(1)).colour;
        let air_colour = scenario.materials.get(MaterialId::new(0)).colour;

        // Stone lump at grid (0,0): j=0 -> bottom image row, i.e. image row
        // (height-1)*cell_px = 3*2 = 6, image col 0.
        assert_eq!(px(&buf, width_px, 0, 6), stone_colour, "stone lump pixel");
        // Water pool at grid (5,0): image row 6, image col 5*2=10.
        assert_eq!(px(&buf, width_px, 10, 6), water_colour, "water pool pixel");
        // A background cell, e.g. grid (3,3): j=3 -> image row 0, col 6.
        assert_eq!(px(&buf, width_px, 6, 0), air_colour, "background air pixel");
    }
}
