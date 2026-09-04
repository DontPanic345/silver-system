//! The grid: a fixed-size, structure-of-arrays, double-buffered collection
//! of cells, each addressed by `math::GridIndex`.
//!
//! This module is M1.1 round 1's goals 1, 3 and 4 (see
//! `cycle-log/tranche-1/m1.1/round-01.md`) — the shape every later round in
//! this milestone, and every later milestone in tranche 1 (granular solids,
//! liquids, pressure, gases, temperature), adds behaviour on top of. This
//! round adds no behaviour: no step function (round 2's job), no per-cell
//! physics. Its whole job is the bones being right: a cell is addressed the
//! way callers will expect, and a step can write a next state without
//! disturbing the current one.
//!
//! ## Structure-of-arrays, and why only one array so far
//!
//! Each per-cell field the simulation needs is its own parallel `Vec`,
//! indexed by the same linear position — not a `Vec<Cell>` of per-cell
//! structs. Round 1 stores exactly one such field (`MaterialId`), because
//! that is all this round's goals ask a cell to hold; the SoA shape is
//! chosen now specifically so a later round (temperature, velocity, ...)
//! adds a new parallel `Vec<Scalar>` field to `Grid` rather than reshaping
//! every cell's layout retroactively — the same "get the shape right before
//! the retrofit tax compounds" reasoning `src/math.rs`'s own module doc
//! comment gives for `Vec2`/`GridIndex`.
//!
//! ## Double buffering
//!
//! `Grid` holds two `MaterialId` buffers, `current` and `next`, of identical
//! size. Reads ([`Grid::get`]) and ad-hoc writes ([`Grid::set`], used by
//! scenario setup and tests) go through `current`. A step (round 2's job)
//! writes its results into `next` via [`Grid::set_next`] without disturbing
//! `current` mid-step — every cell's next state is computed from a
//! consistent, unchanging view of the current one — and then calls
//! [`Grid::swap`] once the whole step is done, making the written state the
//! new `current`. This round does not implement a step; it only makes the
//! buffer shape exist and be swappable, per the round's stated goal 1.
//!
//! ## Coordinate convention: this is `GridIndex`'s first real caller
//!
//! `src/math.rs` pins `GridIndex`'s `(i, j)` convention (`i` maps to world
//! `x`, `j` maps to world `y`, `+y` up) but had no real caller before this
//! round — see that module's doc comment. This grid is that caller:
//! [`Grid::linear_index`] is the new decision this round makes about how a
//! `GridIndex` maps onto this grid's flat backing storage (row-major, `i`
//! fastest-varying — i.e. adjacent `i` are adjacent in storage, adjacent `j`
//! are `width` apart), and [`Grid::cell_center`] delegates to
//! `GridIndex::center` so a cell's world-space position is available
//! without this module re-deriving it.

use crate::material::MaterialId;
use crate::math::{GridIndex, Scalar, Vec2};

/// A fixed-size, double-buffered grid of cells, each holding a
/// [`MaterialId`]. See the module doc comment for the structure-of-arrays
/// and double-buffering shape.
///
/// Dimensions are fixed at construction ([`Grid::new`]) — this round's goal
/// 1 states this explicitly; resizing a grid after construction is not
/// supported and no method here attempts it.
pub struct Grid {
    width: usize,
    height: usize,
    current: Vec<MaterialId>,
    next: Vec<MaterialId>,
}

impl Grid {
    /// Builds a `width` x `height` grid with every cell, in both buffers,
    /// set to `fill`.
    ///
    /// Left as a stub for Green: allocating and filling both buffers at the
    /// chosen size is this round's real content, not plumbing.
    pub fn new(width: usize, height: usize, fill: MaterialId) -> Self {
        let _ = (width, height, fill);
        todo!("allocate current and next buffers of width * height cells, filled with `fill`")
    }

    /// This grid's fixed width, in cells. Plain plumbing (returns the field
    /// fixed at construction), so implemented directly rather than stubbed
    /// — same reasoning `src/math.rs`'s `Vec2::new` gives for its own
    /// plumbing.
    pub fn width(&self) -> usize {
        self.width
    }

    /// This grid's fixed height, in cells. Plain plumbing, implemented
    /// directly — same reasoning as [`Grid::width`].
    pub fn height(&self) -> usize {
        self.height
    }

    /// Converts a [`GridIndex`] into this grid's flat backing-storage
    /// position (the same position in both `current` and `next`).
    ///
    /// This is the round's real convention decision (see the module doc
    /// comment), left as a stub for Green: the tests below pin what the
    /// convention must satisfy (agreement with [`Grid::cells`], row-major
    /// with `i` fastest-varying) without assuming an implementation here.
    ///
    /// Panics if `index` names a cell outside this grid's fixed bounds
    /// (`0 <= i < width`, `0 <= j < height`) — this round's grid does not
    /// wrap or clamp out-of-bounds access.
    pub fn linear_index(&self, index: GridIndex) -> usize {
        let _ = index;
        todo!("convert `index` to a flat position, row-major with i fastest-varying; panic if out of bounds")
    }

    /// Reads the material at `index` from the **current** buffer.
    ///
    /// Left as a stub for Green: depends on [`Grid::linear_index`], this
    /// round's real decision.
    pub fn get(&self, index: GridIndex) -> MaterialId {
        let _ = index;
        todo!("read current[self.linear_index(index)]")
    }

    /// Writes the material at `index` into the **current** buffer directly
    /// — for scenario setup and tests, not for a step's per-cell writes
    /// (those go through [`Grid::set_next`] so a step never observes its
    /// own in-progress results).
    ///
    /// Left as a stub for Green: depends on [`Grid::linear_index`].
    pub fn set(&mut self, index: GridIndex, id: MaterialId) {
        let _ = (index, id);
        todo!("write id into current[self.linear_index(index)]")
    }

    /// Reads the material at `index` from the **next** buffer — mainly for
    /// tests to observe a write before [`Grid::swap`] makes it current.
    ///
    /// Left as a stub for Green: depends on [`Grid::linear_index`].
    pub fn get_next(&self, index: GridIndex) -> MaterialId {
        let _ = index;
        todo!("read next[self.linear_index(index)]")
    }

    /// Writes the material at `index` into the **next** buffer — the write
    /// a step (round 2's job) uses so it never disturbs `current` mid-step.
    ///
    /// Left as a stub for Green: depends on [`Grid::linear_index`].
    pub fn set_next(&mut self, index: GridIndex, id: MaterialId) {
        let _ = (index, id);
        todo!("write id into next[self.linear_index(index)]")
    }

    /// Swaps the `current` and `next` buffers, so whatever was just written
    /// into `next` (via [`Grid::set_next`]) becomes the grid's new
    /// `current` state. This round's job is this swap existing and doing
    /// exactly a swap (no copying, no partial state) — round 2 decides when
    /// a step calls it.
    ///
    /// Left as a stub for Green: a real swap (`std::mem::swap` or
    /// equivalent) is one line, but it is still this round's content to
    /// commit to, not assumed here.
    pub fn swap(&mut self) {
        todo!("swap the current and next buffers")
    }

    /// A read-only view of this grid's **current**-buffer storage, in the
    /// same flat layout [`Grid::linear_index`] addresses into. Exists so
    /// the grid/storage-agreement convention (this round's goal 4) can be
    /// pinned by a test: `grid.get(idx) == grid.cells()[grid.linear_index(idx)]`.
    ///
    /// Plain plumbing (returns a slice view of the existing field), so
    /// implemented directly rather than stubbed — same reasoning as
    /// [`Grid::width`].
    pub fn cells(&self) -> &[MaterialId] {
        &self.current
    }

    /// The world-space position of `index`'s cell center, under a uniform
    /// `cell_size`. Delegates entirely to `GridIndex::center` — no new
    /// decision here, just this grid's first genuine caller of it (closing
    /// the gap `src/math.rs`'s module doc comment names: `GridIndex`/`Vec2`
    /// existed, tested, but uncalled by real code before this round). Plain
    /// delegation, so implemented directly rather than stubbed.
    pub fn cell_center(&self, index: GridIndex, cell_size: Scalar) -> Vec2 {
        index.center(cell_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::MaterialId;

    const AIR: MaterialId = MaterialId(0);
    const WATER: MaterialId = MaterialId(1);

    // --- Scenario: fixed dimensions, GridIndex-addressed (goal 1) ---

    /// Scenario: a freshly built grid reports the exact width and height it
    /// was constructed with, and every cell in that range reads back the
    /// fill material given at construction — "dimensions are fixed at
    /// construction" and "addressed by GridIndex" together.
    #[test]
    fn fresh_grid_has_its_constructed_dimensions_and_is_filled_everywhere() {
        let grid = Grid::new(4, 3, AIR);
        assert_eq!(grid.width(), 4);
        assert_eq!(grid.height(), 3);

        for j in 0..3 {
            for i in 0..4 {
                assert_eq!(
                    grid.get(GridIndex::new(i, j)),
                    AIR,
                    "cell ({i}, {j}) should read back the fill material"
                );
            }
        }
    }

    /// Scenario: writing a specific material to one cell via `set` changes
    /// only that cell — its neighbours still read the original fill
    /// material. Pins that `set`/`get` are genuinely per-cell, addressed by
    /// `GridIndex`, not a whole-grid operation (goals 1 and 3 together).
    #[test]
    fn setting_one_cell_does_not_disturb_its_neighbours() {
        let mut grid = Grid::new(3, 3, AIR);
        grid.set(GridIndex::new(1, 1), WATER);

        assert_eq!(grid.get(GridIndex::new(1, 1)), WATER);
        for j in 0..3 {
            for i in 0..3 {
                if (i, j) != (1, 1) {
                    assert_eq!(
                        grid.get(GridIndex::new(i, j)),
                        AIR,
                        "cell ({i}, {j}) should be untouched by setting (1, 1)"
                    );
                }
            }
        }
    }

    // --- Scenario: double buffering (goal 1) ---

    /// Scenario: writing to the next buffer is invisible through `get`
    /// (which reads current) until `swap` is called, after which it becomes
    /// the current state — the double-buffer isolation the module doc
    /// comment describes, and the exact mechanism round 2's step function
    /// will rely on.
    #[test]
    fn next_buffer_write_is_isolated_from_current_until_swap() {
        let mut grid = Grid::new(2, 2, AIR);
        let idx = GridIndex::new(0, 0);

        grid.set_next(idx, WATER);
        assert_eq!(
            grid.get(idx),
            AIR,
            "writing to next must not be visible through get (current) before swap"
        );
        assert_eq!(grid.get_next(idx), WATER);

        grid.swap();
        assert_eq!(
            grid.get(idx),
            WATER,
            "after swap, what was written to next should be the new current"
        );
    }

    /// Scenario: after a swap, the buffer that used to be current becomes
    /// the new next — a real swap of both buffers, not a one-directional
    /// copy. Writing into the (new) next buffer again and reading the (new)
    /// current confirms the old current's data is preserved there until the
    /// following swap.
    #[test]
    fn swap_exchanges_both_buffers_not_just_one_direction() {
        let mut grid = Grid::new(2, 2, AIR);
        let idx = GridIndex::new(0, 0);

        grid.set(idx, WATER); // current = WATER, next = AIR
        grid.swap(); // current = AIR (old next), next = WATER (old current)

        assert_eq!(
            grid.get(idx),
            AIR,
            "after swap, current should hold what next held before the swap"
        );
        assert_eq!(
            grid.get_next(idx),
            WATER,
            "after swap, next should hold what current held before the swap, \
             confirming this is a real exchange, not a one-way copy"
        );
    }

    // --- Scenario: convention pin, grid/storage agreement (goal 4) ---

    /// Scenario: indexing the grid via `GridIndex` (`get`) and indexing its
    /// underlying flat storage directly (`cells()[linear_index(idx)]`)
    /// agree, for a variety of cells including the origin and a
    /// non-trivial interior cell. This is goal 4's core pin: the public
    /// `GridIndex`-addressed API and the raw storage layout must never
    /// silently disagree.
    #[test]
    fn grid_index_lookup_and_raw_storage_lookup_agree() {
        let mut grid = Grid::new(5, 4, AIR);
        grid.set(GridIndex::new(3, 2), WATER);

        for j in 0..4 {
            for i in 0..5 {
                let idx = GridIndex::new(i, j);
                assert_eq!(
                    grid.get(idx),
                    grid.cells()[grid.linear_index(idx)],
                    "get({idx:?}) and cells()[linear_index({idx:?})] must agree"
                );
            }
        }
    }

    /// Scenario: stepping the `GridIndex` by one along `i` moves exactly one
    /// position in the flat backing storage — the grid's width axis matches
    /// `GridIndex::i` the way callers will expect (`i` is fastest-varying in
    /// storage). Mirrors `src/math.rs`'s own
    /// `adjacent_indices_along_i_are_exactly_one_cell_size_apart_in_x`, but
    /// pinned for this grid's storage layout specifically, not world-space
    /// position.
    #[test]
    fn stepping_i_by_one_moves_exactly_one_position_in_storage() {
        let grid = Grid::new(6, 6, AIR);
        let here = grid.linear_index(GridIndex::new(2, 3));
        let one_over = grid.linear_index(GridIndex::new(3, 3));
        assert_eq!(
            one_over,
            here + 1,
            "adjacent i should be adjacent in the grid's flat storage"
        );
    }

    /// Scenario: stepping the `GridIndex` by one along `j` moves exactly
    /// `width` positions in the flat backing storage — the grid's height
    /// axis matches `GridIndex::j` as a full row stride. Mirrors
    /// `src/math.rs`'s
    /// `adjacent_indices_along_j_are_exactly_one_cell_size_apart_in_plus_y`,
    /// pinned for this grid's storage layout.
    #[test]
    fn stepping_j_by_one_moves_exactly_one_row_stride_in_storage() {
        let grid = Grid::new(6, 6, AIR);
        let here = grid.linear_index(GridIndex::new(2, 3));
        let one_up = grid.linear_index(GridIndex::new(2, 4));
        assert_eq!(
            one_up,
            here + grid.width(),
            "adjacent j should be exactly one row (width cells) apart in the \
             grid's flat storage"
        );
    }

    // --- Scenario: Vec2/GridIndex genuinely called (goal 3 / milestone target 4) ---

    /// Scenario: `Grid::cell_center` (this grid's own use of
    /// `GridIndex::center`) agrees with calling `GridIndex::center`
    /// directly — pins that the grid is a genuine caller of `GridIndex`
    /// (and, transitively, `Vec2`, `center`'s return type), not a
    /// coincidental one, by checking the two paths produce the same
    /// `Vec2`.
    #[test]
    fn grid_cell_center_agrees_with_calling_grid_index_center_directly() {
        let grid = Grid::new(4, 4, AIR);
        let idx = GridIndex::new(2, 1);
        let cell_size = 5.0;

        assert_eq!(grid.cell_center(idx, cell_size), idx.center(cell_size));
        assert_eq!(grid.cell_center(idx, cell_size), Vec2::new(12.5, 7.5));
    }

    // --- Disposable unit tests ---

    /// Disposable: a 1x1 grid is a degenerate but valid case — width and
    /// height both 1, exactly one addressable cell.
    #[test]
    fn one_by_one_grid_has_exactly_one_cell() {
        let grid = Grid::new(1, 1, WATER);
        assert_eq!(grid.width(), 1);
        assert_eq!(grid.height(), 1);
        assert_eq!(grid.get(GridIndex::new(0, 0)), WATER);
    }

    /// Disposable: indexing at or past the grid's width/height panics
    /// rather than silently wrapping or clamping — pins the documented
    /// out-of-bounds contract on `Grid::linear_index`/`get`.
    #[test]
    #[should_panic]
    fn indexing_past_width_panics() {
        let grid = Grid::new(2, 2, AIR);
        let _ = grid.get(GridIndex::new(2, 0));
    }
}
