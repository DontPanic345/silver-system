//! `Scenario`: a single data value holding everything needed to build and
//! run a scenario — the "one definition, two consumers" type this
//! milestone's round 2 exists to shape (see
//! `cycle-log/tranche-1/m1.1/round-02.md`, goal 2). Round 3 (a headless
//! runner) and round 4 (a renderer) will both build a `Grid` from the same
//! `Scenario` value and then diverge — one measuring, one drawing — without
//! either owning scenario *definition* itself. Neither consumer exists yet;
//! this round's job is the shape being genuinely usable by both, not
//! building either one.
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
//! placement list is exactly the same "(index, id) pairs" shape round 1's
//! own tests already use ad hoc (`grid.set(GridIndex::new(..), ..)` calls);
//! this just gives that pattern a name and a durable, reusable home.
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
    /// This is the round's one real decision (how a scenario's data becomes
    /// a grid), so it is content, not plumbing — but it is a short, direct
    /// composition of `Grid::new` and `Grid::set`, both already proven by
    /// round 1's own tests, so it is implemented directly here rather than
    /// left as a stub.
    pub fn build_grid(&self) -> Grid {
        let mut grid = Grid::new(self.width, self.height, self.background);
        for &(index, id) in &self.placements {
            grid.set(index, id);
        }
        grid
    }
}

/// A small, concrete fixture: a `6x4` grid of air with a lump of stone in
/// one corner and a pool of water in another — the "small grid with a lump
/// of stone and a pool of water sitting in air" example this round's goal 3
/// asks for, named so round 3 (headless runner) and round 4 (renderer) can
/// both find and reuse it rather than each inventing their own.
///
/// Uses [`MaterialTable::reference`] (air/water/stone, per round 1) as its
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

#[cfg(test)]
mod tests {
    use super::*;

    // --- Scenario: Scenario converts to a runnable Grid (goal 2) ---

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

    // --- Scenario: the named fixture (goal 3) ---

    /// Scenario: `stone_and_water_pool` builds a grid whose dimensions match
    /// what it declares, with air as the background everywhere except its
    /// documented stone lump and water pool — pins the fixture is genuinely
    /// usable (constructs, has the right shape) so round 3/4 can rely on it
    /// without re-deriving its layout.
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
}
