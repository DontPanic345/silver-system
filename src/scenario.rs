//! `Scenario`: a single data value holding everything needed to build and
//! run a scenario — the "one definition, two consumers" shape this module
//! exists to provide. `src/measure.rs` (a headless runner) and
//! `src/render.rs` (a renderer) both build a `Grid` from the same
//! `Scenario` value and then diverge — one measuring, one drawing — without
//! either owning scenario *definition* itself.
//!
//! ## Shape chosen: data + an explicit placement list, not a builder/closure
//!
//! A `Scenario` is plain data: grid dimensions, a `cell_size`, an owned
//! [`MaterialTable`], a `background` material every cell starts as, and an
//! explicit `Vec<(GridIndex, MaterialId)>` of placements applied on top of
//! that background. [`Scenario::build_grid`] is the single conversion to a
//! runnable [`Grid`].
//!
//! A closure/initializer-function shape was considered and rejected: a
//! `Box<dyn Fn(&mut Grid)>` (or generic initializer) is not `Clone`,
//! `Debug`, or serializable, and a headless runner or renderer inspecting
//! *what a scenario contains* (for logging, for a UI list of fixtures, for
//! a future save/replay format) would have to run it against a real `Grid`
//! just to find out — an explicit data list can be read, counted, and
//! compared directly, by both consumers, without executing anything. The
//! placement list is exactly the same "(index, id) pairs" shape
//! `src/grid.rs`'s own tests already use ad hoc (`grid.set(GridIndex::new(..),
//! ..)` calls); this just gives that pattern a name and a reusable home.
//!
//! `MaterialTable` is stored *owned*, not referenced, so a `Scenario` is a
//! fully self-contained value — a headless runner and a renderer loading
//! the same named fixture (see [`stone_and_water_pool`]) each get their own
//! independent table and grid, with no lifetime or shared-ownership
//! plumbing required between them.

use crate::grid::Grid;
use crate::material::{MaterialId, MaterialTable};
use crate::math::{GridIndex, Scalar};

/// Everything needed to build a runnable [`Grid`]: fixed dimensions, a
/// uniform `cell_size`, the [`MaterialTable`] cell values reference, a
/// `background` material every cell starts as, and an explicit list of
/// `(GridIndex, MaterialId)` placements applied on top of that background.
///
/// See the module doc comment for why this shape (plain data, not a
/// builder/closure) was chosen, and [`Scenario::build_grid`] for the one
/// conversion to a runnable `Grid`.
pub struct Scenario {
    pub width: usize,
    pub height: usize,
    pub cell_size: Scalar,
    pub materials: MaterialTable,
    pub background: MaterialId,
    pub placements: Vec<(GridIndex, MaterialId)>,
}

impl Scenario {
    /// Builds a scenario from its parts. Plain field construction, no
    /// decision to make, so implemented directly — same reasoning
    /// `src/math.rs`'s `Vec2::new`/`GridIndex::new` give for their own
    /// plumbing.
    pub fn new(
        width: usize,
        height: usize,
        cell_size: Scalar,
        materials: MaterialTable,
        background: MaterialId,
        placements: Vec<(GridIndex, MaterialId)>,
    ) -> Self {
        Scenario {
            width,
            height,
            cell_size,
            materials,
            background,
            placements,
        }
    }

    /// Converts this scenario into a runnable [`Grid`]: a fresh grid of
    /// this scenario's `width`/`height`, filled everywhere with
    /// `background`, then every `placements` entry applied on top via
    /// [`Grid::set`] — later entries in the list win if two placements name
    /// the same cell, the same "last write wins" behaviour `Grid::set`
    /// itself already has.
    ///
    /// This is the one real decision (how a scenario's data becomes a
    /// grid), so it is content, not plumbing — but it is a short, direct
    /// composition of `Grid::new` and `Grid::set`, both already proven by
    /// `src/grid.rs`'s own tests, so it is implemented directly here rather
    /// than left as a stub.
    pub fn build_grid(&self) -> Grid {
        let mut grid = Grid::new(self.width, self.height, self.background);
        for &(index, id) in &self.placements {
            grid.set(index, id);
        }
        grid
    }
}

/// A small, concrete fixture: a `6x4` grid of air with a lump of stone in
/// one corner and a pool of water in another — a "small grid with a lump of
/// stone and a pool of water sitting in air" example, named so the headless
/// runner and the renderer can both find and reuse it rather than each
/// inventing their own.
///
/// Uses [`MaterialTable::reference`] (air/water/stone) as its
/// material table, air as the background, a 2x2 stone lump in the
/// bottom-left and a 2x1 water pool along the right edge — chosen only to
/// be visually/structurally distinct (a lump vs. a pool, different sizes,
/// non-overlapping), not to model anything physically meaningful yet.
pub fn stone_and_water_pool() -> Scenario {
    let materials = MaterialTable::reference();
    let air = MaterialId::new(0);
    let water = MaterialId::new(1);
    let stone = MaterialId::new(2);

    let mut placements = Vec::new();
    // A 2x2 lump of stone in the bottom-left corner.
    for j in 0..2 {
        for i in 0..2 {
            placements.push((GridIndex::new(i, j), stone));
        }
    }
    // A 2-cell-tall pool of water along the right edge.
    for j in 0..2 {
        placements.push((GridIndex::new(5, j), water));
    }

    Scenario::new(6, 4, 1.0, materials, air, placements)
}

/// A larger, watchable fixture built to exercise `src/grid.rs`'s real
/// gravity/density physics rather than just sit statically — the "one
/// definition, two consumers" shape unchanged, but this scenario is meant
/// to be *stepped*, not just painted once. Used by `src/lib.rs`'s
/// `step_and_paint_physics_demo` (`www/physics.html`) and available to a
/// headless caller the same way [`stone_and_water_pool`] is.
///
/// A sealed stone container (floor + both side walls), a flat, already-
/// resting pool of water filling the interior's lower rows (every column
/// the same height — under the movement rule's own horizontal-flow gate,
/// see `src/grid.rs`, a perfectly flat pool has nowhere lower to flow into
/// and stays put on its own), and several sand grains suspended in the open
/// air above at staggered heights. Watching it step forward should show:
/// each grain falling straight down, then sinking through the water column
/// beneath it (denser than water) and displacing that water upward, with
/// the pool re-levelling around the disturbance afterward — granular
/// falling, density-driven sinking, and liquid levelling, all from the one
/// generic movement rule, no per-material special case.
pub fn physics_demo() -> Scenario {
    let materials = MaterialTable::reference();
    let air = MaterialId::new(0);
    let water = MaterialId::new(1);
    let stone = MaterialId::new(2);
    let sand = MaterialId::new(3);

    const WIDTH: usize = 24;
    const HEIGHT: usize = 16;

    let mut placements = Vec::new();
    for i in 0..WIDTH as i32 {
        placements.push((GridIndex::new(i, 0), stone));
    }
    for j in 0..HEIGHT as i32 {
        placements.push((GridIndex::new(0, j), stone));
        placements.push((GridIndex::new(WIDTH as i32 - 1, j), stone));
    }
    // A flat, already-level pool across the whole interior width, rows 1-4.
    for j in 1..5 {
        for i in 1..(WIDTH as i32 - 1) {
            placements.push((GridIndex::new(i, j), water));
        }
    }
    // Sand grains suspended in the open air above, staggered so they don't
    // all land at once.
    for (n, &i) in [3, 7, 11, 15, 19].iter().enumerate() {
        placements.push((GridIndex::new(i, 10 + n as i32), sand));
    }

    Scenario::new(WIDTH, HEIGHT, 1.0, materials, air, placements)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Scenario: Scenario converts to a runnable Grid ---

    /// Scenario: `build_grid` fills every cell with `background` except the
    /// explicitly placed ones, which read back exactly the material they
    /// were placed with — the basic "data in, correct grid out" contract
    /// every later consumer (headless runner, renderer) relies on.
    #[test]
    fn build_grid_applies_background_and_placements() {
        let materials = MaterialTable::reference();
        let air = MaterialId::new(0);
        let water = MaterialId::new(1);
        let placements = vec![(GridIndex::new(1, 1), water)];
        let scenario = Scenario::new(3, 3, 1.0, materials, air, placements);

        let grid = scenario.build_grid();
        assert_eq!(grid.width(), 3);
        assert_eq!(grid.height(), 3);
        for j in 0..3 {
            for i in 0..3 {
                let idx = GridIndex::new(i, j);
                let expected = if (i, j) == (1, 1) { water } else { air };
                assert_eq!(
                    grid.get(idx),
                    expected,
                    "cell ({i}, {j}) should be {expected:?}"
                );
            }
        }
    }

    /// Scenario: when two placements name the same cell, the later one in
    /// the list wins — matching `Grid::set`'s own last-write-wins
    /// behaviour, so a `Scenario`'s placement order has an unsurprising,
    /// documented meaning.
    #[test]
    fn build_grid_lets_a_later_placement_override_an_earlier_one_at_the_same_cell() {
        let materials = MaterialTable::reference();
        let air = MaterialId::new(0);
        let water = MaterialId::new(1);
        let stone = MaterialId::new(2);
        let idx = GridIndex::new(0, 0);
        let placements = vec![(idx, water), (idx, stone)];
        let scenario = Scenario::new(2, 2, 1.0, materials, air, placements);

        let grid = scenario.build_grid();
        assert_eq!(
            grid.get(idx),
            stone,
            "the later placement in the list should win"
        );
    }

    // --- Scenario: the named fixture ---

    /// Scenario: `stone_and_water_pool` builds a grid whose dimensions match
    /// what it declares, with air as the background everywhere except its
    /// documented stone lump and water pool — pins the fixture is genuinely
    /// usable (constructs, has the right shape) so the headless runner and
    /// renderer can rely on it without re-deriving its layout.
    #[test]
    fn stone_and_water_pool_builds_a_grid_with_air_background_and_both_materials_present() {
        let scenario = stone_and_water_pool();
        let grid = scenario.build_grid();

        assert_eq!(grid.width(), scenario.width);
        assert_eq!(grid.height(), scenario.height);

        let air = MaterialId::new(0);
        let water = MaterialId::new(1);
        let stone = MaterialId::new(2);

        // The documented stone lump.
        assert_eq!(grid.get(GridIndex::new(0, 0)), stone);
        assert_eq!(grid.get(GridIndex::new(1, 1)), stone);
        // The documented water pool.
        assert_eq!(grid.get(GridIndex::new(5, 0)), water);
        assert_eq!(grid.get(GridIndex::new(5, 1)), water);
        // A cell outside either placed region stays background air.
        assert_eq!(grid.get(GridIndex::new(3, 3)), air);
    }

    // --- Scenario: physics_demo is a genuine physics fixture, not a static image ---

    /// Scenario: `physics_demo()` builds with the documented shape (sealed
    /// stone container, a flat resting water pool, five suspended sand
    /// grains) — pins the fixture is usable before the physics test below
    /// asks anything harder of it.
    #[test]
    fn physics_demo_builds_with_its_documented_shape() {
        let scenario = physics_demo();
        let grid = scenario.build_grid();
        assert_eq!(grid.width(), 24);
        assert_eq!(grid.height(), 16);

        let sand = MaterialId::new(3);
        let mut sand_cells = 0;
        for j in 0..16 {
            for i in 0..24 {
                if grid.get(GridIndex::new(i, j)) == sand {
                    sand_cells += 1;
                    assert!(j >= 10, "sand should start suspended above the pool, not in it");
                }
            }
        }
        assert_eq!(sand_cells, 5, "physics_demo should place exactly 5 sand grains");
    }

    /// Scenario: stepping `physics_demo()` forward under real gravity/
    /// density physics (see `src/grid.rs`) for long enough settles every
    /// suspended sand grain out of the open air it started in — pins that
    /// the fixture is genuinely watchable physics (things fall and land),
    /// not a scenario that happens to already be at rest, and does so
    /// headlessly (grid inspection + assertions, no rendered pixels, no
    /// human judging a screenshot).
    #[test]
    fn physics_demo_settles_every_suspended_grain_after_enough_steps() {
        let scenario = physics_demo();
        let materials = MaterialTable::reference();
        let mut grid = scenario.build_grid();
        let mut timestep = crate::timestep::FixedTimestep::new(1.0 / 30.0);

        let sand = MaterialId::new(3);
        let count_sand_at_or_above = |grid: &crate::grid::Grid, min_j: i32| -> usize {
            (min_j..16)
                .flat_map(|j| (0..24).map(move |i| GridIndex::new(i, j)))
                .filter(|&idx| grid.get(idx) == sand)
                .count()
        };
        assert_eq!(
            count_sand_at_or_above(&grid, 10),
            5,
            "all 5 grains should start suspended at/above row 10"
        );

        for _ in 0..300 {
            grid.step(&mut timestep, 1.0 / 30.0, &materials);
        }

        assert_eq!(
            count_sand_at_or_above(&grid, 10),
            0,
            "after 300 steps, every grain should have fallen out of the \
             open-air rows it started in"
        );

        // Conservation, once more, at this fixture's larger scale: still
        // exactly 5 sand cells somewhere in the grid, none created or lost.
        let total_sand = (0..16)
            .flat_map(|j| (0..24).map(move |i| GridIndex::new(i, j)))
            .filter(|&idx| grid.get(idx) == sand)
            .count();
        assert_eq!(total_sand, 5, "sand cell count must be exactly conserved");
    }
}
