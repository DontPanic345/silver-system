//! The grid: a fixed-size, structure-of-arrays, double-buffered collection
//! of cells, each addressed by `math::GridIndex`.
//!
//! This is the shape all later grid behaviour (granular solids, liquids,
//! pressure, gases, temperature) adds on top of. This module itself adds no
//! per-cell physics — its whole job is the bones being right: a cell is
//! addressed the way callers will expect, and a step can write a next state
//! without disturbing the current one.
//!
//! ## Structure-of-arrays, and why only one array so far
//!
//! Each per-cell field the simulation needs is its own parallel `Vec`,
//! indexed by the same linear position — not a `Vec<Cell>` of per-cell
//! structs. Currently stores exactly one such field (`MaterialId`); the SoA
//! shape is chosen now specifically so later behaviour (temperature,
//! velocity, ...) adds a new parallel `Vec<Scalar>` field to `Grid` rather
//! than reshaping every cell's layout retroactively — the same "get the
//! shape right before the retrofit tax compounds" reasoning `src/math.rs`'s
//! own module doc comment gives for `Vec2`/`GridIndex`.
//!
//! ## Double buffering
//!
//! `Grid` holds two `MaterialId` buffers, `current` and `next`, of identical
//! size. Reads ([`Grid::get`]) and ad-hoc writes ([`Grid::set`], used by
//! scenario setup and tests) go through `current`. A step writes its
//! results into `next` without disturbing `current` mid-step — from outside
//! `step`/`step_once`, `current` is guaranteed unchanged until the whole
//! step completes and calls [`Grid::swap`], at which point the written state
//! becomes the new `current`.
//!
//! **Internal exception, since the physics step (below) exists:** the
//! *external* guarantee above ("current unchanged until swap") does not mean
//! `next` is computed as one pure function of a frozen `current` snapshot,
//! the way the original identity-transform `step_once` worked. A cellular
//! movement rule that swaps two cells' contents needs an *exclusive claim*
//! on both cells — two cells independently deciding to move into the same
//! third cell from a frozen snapshot would race, and double buffering alone
//! can't express "first claim wins" between two independent per-cell
//! decisions. So `step_once` seeds `next` as a copy of `current` and then
//! reads *and* writes `next` progressively as it scans the grid once,
//! guarded by a per-step `moved` bitset (below) so a parcel of material only
//! ever moves once per step. This is the standard technique real
//! falling-sand engines use for exactly this reason. `current` itself is
//! still never touched until [`Grid::swap`] — the external contract holds.

//!
//! ## Coordinate convention: this is `GridIndex`'s first real caller
//!
//! `src/math.rs` pins `GridIndex`'s `(i, j)` convention (`i` maps to world
//! `x`, `j` maps to world `y`, `+y` up) — see that module's doc comment.
//! This grid is `GridIndex`'s real caller: [`Grid::linear_index`] decides
//! how a `GridIndex` maps onto this grid's flat backing storage (row-major,
//! `i` fastest-varying — i.e. adjacent `i` are adjacent in storage, adjacent
//! `j` are `width` apart), and [`Grid::cell_center`] delegates to
//! `GridIndex::center` so a cell's world-space position is available
//! without this module re-deriving it.
//!
//! **Note for any future renderer:** increasing `j` moves one full row
//! *forward* in this flat storage (`linear_index` grows), and `j` increases
//! in world `+y` (up, per `math.rs`). Neither of those facts says anything
//! about which way is "up" on screen — a canvas row index increases
//! *downward* (`src/lib.rs`'s own pinned convention). Code that walks
//! `Grid::cells()` linearly to fill image rows top-to-bottom must flip that
//! axis explicitly (row 0 of the image is the *largest* `j`, not `j == 0`),
//! the same flip `math.rs`'s `Vec2` doc comment already requires at any
//! world-to-canvas boundary — this is not a new rule, just this module's own
//! instance of it, named here so it isn't rediscovered as a bug (see
//! `src/render.rs`, which does exactly this flip).

use crate::material::{MaterialId, MaterialTable, Phase};
use crate::math::{GridIndex, Scalar, Vec2};
use crate::timestep::FixedTimestep;

// The movement rule, in words (implemented by `Grid::try_move_cell` below):
// a single mobile cell's destination candidates are checked in priority
// order, and the rule applied against each is: **a denser cell may swap
// into a neighbour's position iff that neighbour is strictly less dense and
// not `Phase::Solid`** — generic over materials, driven entirely by
// `Material::density`/`Phase` data, never a per-material `if` chain (see
// `src/material.rs`'s own module doc comment for why that distinction
// matters).
//
// Priority, matching how granular media and liquids actually settle:
//
// 1. Straight down (`(i, j-1)`) — gravity's own direction, since `+y` is up
//    (`src/math.rs`).
// 2. Diagonally down, both sides — lets a grain slide off a slope instead
//    of stacking into an unstable spike, and lets a denser cell settle to
//    one side when directly below is blocked.
// 3. **Liquid only:** sideways, both sides, at the same row — a liquid that
//    cannot fall any further spreads out instead. Plain "swap with any
//    less-dense same-row neighbour" oscillates forever between two
//    perfectly symmetric open columns (every cell in a column independently
//    finds the same open neighbour column, so the whole block translates
//    sideways as a rigid unit, then translates right back next step when
//    the tie-break side flips) rather than actually levelling — so a
//    horizontal move additionally requires the destination column to
//    currently hold *strictly fewer* cells of this exact material than the
//    source column (`Grid::column_count`, read from `next`, so it reflects
//    swaps already applied earlier in this same step). That one extra
//    condition is what turns "spreads out" into "spreads out and stops
//    once both sides are level" — a cheap approximation of hydrostatic
//    pressure that produces "a resting pool stays flat" / "a column of
//    water finds its level" (`NORTH_STARS.md` #3) as a real, non-oscillating
//    equilibrium rather than a screenshot that merely looks flat once.
//    `Phase::Granular` stops at step 2: sand piles into a slope, it does
//    not flow flat.
//
// The two diagonal (and, for liquids, the two sideways) candidates
// alternate which side is tried first every step (`Grid`'s internal
// `alternate_first_left` flip) so repeated ties do not all resolve the same
// direction, which would visibly bias flow/piling to one side.

/// A fixed-size, double-buffered grid of cells, each holding a
/// [`MaterialId`]. See the module doc comment for the structure-of-arrays
/// and double-buffering shape.
///
/// Dimensions are fixed at construction ([`Grid::new`]); resizing a grid
/// after construction is not supported and no method here attempts it.
pub struct Grid {
    width: usize,
    height: usize,
    current: Vec<MaterialId>,
    next: Vec<MaterialId>,
    /// Flips every [`Grid::step_once`] call; decides which side diagonal/
    /// sideways movement candidates are tried first, so repeated ties don't
    /// all resolve the same direction — see the movement-rule comment above.
    alternate_first_left: bool,
}

impl Grid {
    /// Builds a `width` x `height` grid with every cell, in both buffers,
    /// set to `fill`.
    ///
    pub fn new(width: usize, height: usize, fill: MaterialId) -> Self {
        let size = width * height;
        Grid {
            width,
            height,
            current: vec![fill; size],
            next: vec![fill; size],
            alternate_first_left: false,
        }
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
    /// This is the storage convention decision (see the module doc
    /// comment); the tests below pin what the convention must satisfy
    /// (agreement with [`Grid::cells`], row-major with `i` fastest-varying).
    ///
    /// Panics if `index` names a cell outside this grid's fixed bounds
    /// (`0 <= i < width`, `0 <= j < height`) — this grid does not wrap or
    /// clamp out-of-bounds access.
    pub fn linear_index(&self, index: GridIndex) -> usize {
        assert!(
            index.i >= 0 && (index.i as usize) < self.width,
            "GridIndex {index:?} out of bounds: width is {}",
            self.width
        );
        assert!(
            index.j >= 0 && (index.j as usize) < self.height,
            "GridIndex {index:?} out of bounds: height is {}",
            self.height
        );
        index.j as usize * self.width + index.i as usize
    }

    /// Reads the material at `index` from the **current** buffer.
    pub fn get(&self, index: GridIndex) -> MaterialId {
        self.current[self.linear_index(index)]
    }

    /// Writes the material at `index` into the **current** buffer directly
    /// — for scenario setup and tests, not for a step's per-cell writes
    /// (those go through [`Grid::set_next`] so a step never observes its
    /// own in-progress results).
    pub fn set(&mut self, index: GridIndex, id: MaterialId) {
        let pos = self.linear_index(index);
        self.current[pos] = id;
    }

    /// Reads the material at `index` from the **next** buffer — mainly for
    /// tests to observe a write before [`Grid::swap`] makes it current.
    pub fn get_next(&self, index: GridIndex) -> MaterialId {
        self.next[self.linear_index(index)]
    }

    /// Writes the material at `index` into the **next** buffer — the write
    /// a step uses so it never disturbs `current` mid-step.
    pub fn set_next(&mut self, index: GridIndex, id: MaterialId) {
        let pos = self.linear_index(index);
        self.next[pos] = id;
    }

    /// Swaps the `current` and `next` buffers, so whatever was just written
    /// into `next` (via [`Grid::set_next`]) becomes the grid's new
    /// `current` state — exactly a swap, no copying, no partial state.
    pub fn swap(&mut self) {
        std::mem::swap(&mut self.current, &mut self.next);
    }

    /// A read-only view of this grid's **current**-buffer storage, in the
    /// same flat layout [`Grid::linear_index`] addresses into. Exists so
    /// the grid/storage-agreement convention can be pinned by a test:
    /// `grid.get(idx) == grid.cells()[grid.linear_index(idx)]`.
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
    /// existed, tested, but uncalled by real code). Plain delegation, so
    /// implemented directly rather than stubbed.
    pub fn cell_center(&self, index: GridIndex, cell_size: Scalar) -> Vec2 {
        index.center(cell_size)
    }

    /// Advances this grid by however many fixed steps `frame_duration_secs`
    /// of real elapsed time earns, per `timestep` (the same
    /// `timestep::FixedTimestep` harness `src/lib.rs`'s `advance_tick`
    /// drives — see that function's doc comment for the pattern this
    /// mirrors). Returns the number of fixed steps that elapsed (0, 1, or
    /// occasionally more, per `FixedTimestep::advance`'s own accumulator/
    /// spiral-of-death semantics), having applied [`Grid::step_once`] that
    /// many times against `materials` — the same table the grid's cells are
    /// [`MaterialId`]s into.
    ///
    /// The *mechanism* of stepping is real elapsed-time accounting (not a
    /// bare per-call `+1`), wired to the existing, unmodified
    /// `FixedTimestep`. The per-cell transformation each elapsed step
    /// applies is the gravity/density movement rule — see
    /// [`Grid::step_once`].
    pub fn step(
        &mut self,
        timestep: &mut FixedTimestep,
        frame_duration_secs: Scalar,
        materials: &MaterialTable,
    ) -> u32 {
        let steps = timestep.advance(frame_duration_secs);
        for _ in 0..steps {
            self.step_once(materials);
        }
        steps
    }

    /// Applies exactly one fixed step of the gravity/density movement rule
    /// (see the module-level comment above `Grid`'s definition) to every
    /// cell, then [`Grid::swap`]s once the whole pass is done.
    ///
    /// Seeds `next` as a copy of `current`, then scans every grid position
    /// exactly once — row by row from `j = 0` (gravity's own direction,
    /// since `+y` is up) to the top, alternating left-to-right/right-to-left
    /// within a row and across steps (`alternate_first_left`) — reading and
    /// writing `next` progressively rather than a frozen snapshot, since a
    /// swap needs an exclusive claim on both cells involved (see the
    /// module doc comment's "internal exception" section for why). A
    /// per-step `moved` bitset guards every position visited (as a source
    /// or as a swap target) so a single parcel of material is moved at most
    /// once per step, regardless of scan order.
    ///
    /// Only [`Phase::Granular`]/[`Phase::Liquid`] cells are movers;
    /// [`Phase::Solid`] never moves and never yields its cell to a swap;
    /// [`Phase::Gas`] does not yet move on its own (see [`Phase::Gas`]'s own
    /// doc comment) but can still be displaced by a denser mover swapping
    /// into it. Because every change is a swap of two cells' contents,
    /// never a creation or deletion, the count of cells holding each
    /// [`MaterialId`] is exactly conserved by construction, for any number
    /// of steps — `src/measure.rs`'s per-material cell-count/mass
    /// conservation checks rely on exactly this property.
    fn step_once(&mut self, materials: &MaterialTable) {
        self.next.copy_from_slice(&self.current);
        let mut moved = vec![false; self.width * self.height];
        let left_first = self.alternate_first_left;
        self.alternate_first_left = !self.alternate_first_left;

        for j in 0..self.height {
            let columns: Box<dyn Iterator<Item = usize>> = if left_first {
                Box::new(0..self.width)
            } else {
                Box::new((0..self.width).rev())
            };
            for i in columns {
                let idx = j * self.width + i;
                if moved[idx] {
                    continue;
                }
                let mat = materials.get(self.next[idx]);
                if !matches!(mat.phase, Phase::Granular | Phase::Liquid) {
                    continue;
                }
                self.try_move_cell(materials, &mut moved, i, j, mat.phase, mat.density, left_first);
            }
        }

        self.swap();
    }

    /// The core movement decision for one mover at `(i, j)` (`density`,
    /// `phase` already looked up by the caller): builds this phase's
    /// destination candidates in priority order (see the module-level
    /// movement-rule comment), and swaps `next`'s two positions at the
    /// first candidate that is strictly less dense and not
    /// [`Phase::Solid`] — marking both positions in `moved` so neither is
    /// revisited this step. Does nothing if no candidate qualifies (the
    /// cell is already resting).
    ///
    /// `left_first` picks which side of each diagonal/sideways pair is
    /// tried first, alternated by the caller every step.
    #[allow(clippy::too_many_arguments)]
    fn try_move_cell(
        &mut self,
        materials: &MaterialTable,
        moved: &mut [bool],
        i: usize,
        j: usize,
        phase: Phase,
        density: Scalar,
        left_first: bool,
    ) {
        let width = self.width as i32;
        let (i, j) = (i as i32, j as i32);
        let (first_dx, second_dx): (i32, i32) = if left_first { (-1, 1) } else { (1, -1) };

        let mut candidates: Vec<(i32, i32)> = Vec::with_capacity(5);
        if j > 0 {
            candidates.push((i, j - 1));
            if i + first_dx >= 0 && i + first_dx < width {
                candidates.push((i + first_dx, j - 1));
            }
            if i + second_dx >= 0 && i + second_dx < width {
                candidates.push((i + second_dx, j - 1));
            }
        }
        if phase == Phase::Liquid {
            if i + first_dx >= 0 && i + first_dx < width {
                candidates.push((i + first_dx, j));
            }
            if i + second_dx >= 0 && i + second_dx < width {
                candidates.push((i + second_dx, j));
            }
        }

        let idx = self.linear_index(GridIndex::new(i, j));
        for (ti, tj) in candidates {
            let tidx = self.linear_index(GridIndex::new(ti, tj));
            if moved[tidx] {
                continue;
            }
            let target = materials.get(self.next[tidx]);
            if target.phase == Phase::Solid {
                continue;
            }
            if target.density < density {
                // Horizontal (same-row) candidate: only flow toward the
                // side that currently holds strictly fewer cells of this
                // exact material — see the movement-rule comment above for
                // why plain "any less-dense same-row neighbour" oscillates
                // instead of levelling.
                if tj == j {
                    let self_id = self.next[idx];
                    let source_count = self.column_count(i as usize, self_id);
                    let target_count = self.column_count(ti as usize, self_id);
                    if target_count >= source_count {
                        continue;
                    }
                }
                self.next.swap(idx, tidx);
                moved[idx] = true;
                moved[tidx] = true;
                return;
            }
        }
    }

    /// How many cells in column `i` currently hold exactly `id`, read from
    /// `next` (so it reflects any swaps already applied earlier in the same
    /// [`Grid::step_once`] pass) — the "how full is this side" measure
    /// [`Grid::try_move_cell`]'s horizontal-flow gate compares between a
    /// source and destination column. `O(height)`; only ever called for a
    /// `Phase::Liquid` cell's same-row candidates, not on every cell of
    /// every step.
    fn column_count(&self, i: usize, id: MaterialId) -> usize {
        (0..self.height)
            .filter(|&j| self.next[j * self.width + i] == id)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::MaterialId;

    const AIR: MaterialId = MaterialId(0);
    const WATER: MaterialId = MaterialId(1);
    const STONE: MaterialId = MaterialId(2);

    // --- Scenario: fixed dimensions, GridIndex-addressed ---

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
    /// `GridIndex`, not a whole-grid operation.
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

    // --- Scenario: double buffering ---

    /// Scenario: writing to the next buffer is invisible through `get`
    /// (which reads current) until `swap` is called, after which it becomes
    /// the current state — the double-buffer isolation the module doc
    /// comment describes, and the exact mechanism the step function relies
    /// on.
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

    /// Scenario: a *step* function will call `swap` once per tick, every
    /// tick, for the life of a run — the "swappable" claim is only actually
    /// true if that holds under repeated use, not just the one-swap case
    /// above. Simulates several ticks by hand (write a distinct,
    /// tick-numbered material into `next`, then swap) and checks after
    /// every single swap — not just the last one — that `current` holds
    /// exactly what that tick wrote and nothing from an earlier or later
    /// tick leaks through: the "does it still hold after repetition, not
    /// just once" check a brand-new shared primitive deserves.
    #[test]
    fn swap_behaves_correctly_across_many_repeated_swaps_in_sequence() {
        let mut grid = Grid::new(2, 2, AIR);
        let idx = GridIndex::new(1, 0);
        let untouched = GridIndex::new(0, 1);

        for tick in 0..7u16 {
            let this_tick = MaterialId(tick);
            grid.set_next(idx, this_tick);
            grid.swap();
            assert_eq!(
                grid.get(idx),
                this_tick,
                "after swap #{tick}, current should hold exactly what was \
                 written into next for this tick, not an earlier or later one"
            );
            assert_eq!(
                grid.get(untouched),
                AIR,
                "after swap #{tick}, a cell never written to should still \
                 read its original fill, unaffected by any number of swaps"
            );
        }
    }

    // --- Scenario: convention pin, grid/storage agreement ---

    /// Scenario: indexing the grid via `GridIndex` (`get`) and indexing its
    /// underlying flat storage directly (`cells()[linear_index(idx)]`)
    /// agree, for a variety of cells including the origin and a
    /// non-trivial interior cell. This is the core pin: the public
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

    // --- Scenario: Vec2/GridIndex genuinely called ---

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

    /// Scenario: the out-of-bounds contract holds on *every* edge and axis,
    /// not just the one direction `indexing_past_width_panics` above
    /// happens to cover — negative `i`, negative `j`, `i == width`, and
    /// `j == height` each panic on their own. `linear_index`'s two
    /// `assert!`s are structurally symmetric between `i`/width and
    /// `j`/height (see `src/grid.rs`'s source). All four panics are checked
    /// independently so a future change that broke, say, only the
    /// negative-`j` check specifically would be caught.
    #[test]
    fn out_of_bounds_contract_holds_on_every_edge_and_axis() {
        let grid = Grid::new(3, 2, AIR);

        let cases: [(&str, GridIndex); 4] = [
            ("negative i", GridIndex::new(-1, 0)),
            ("negative j", GridIndex::new(0, -1)),
            ("i == width", GridIndex::new(3, 0)),
            ("j == height", GridIndex::new(0, 2)),
        ];

        for (label, idx) in cases {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| grid.get(idx)));
            assert!(
                result.is_err(),
                "expected grid.get({idx:?}) to panic ({label}), but it returned a value"
            );
        }
    }

    // --- Scenario: step only advances what real elapsed time earns ---

    /// Scenario: mirrors `src/timestep.rs`'s and `src/lib.rs`'s own
    /// "irregular durations total the expected step count" scenarios, now
    /// re-proven at `Grid::step`'s call site specifically. Feeds a stream of
    /// irregular, sub-`dt` and multi-`dt` real durations across several
    /// calls and checks the *total* number of steps `Grid::step` reports
    /// across all calls matches what the total elapsed time honestly earns
    /// — not a bare per-call `+1`, and not losing or inventing steps at call
    /// boundaries.
    #[test]
    fn grid_step_only_advances_what_real_elapsed_time_earns_across_irregular_calls() {
        let materials = MaterialTable::reference();
        let mut grid = Grid::new(2, 2, AIR);
        let mut timestep = FixedTimestep::new(0.1);

        // Durations: 0.03, 0.12, 0.02, 0.11, 0.22 -> total 0.50 -> 5 steps,
        // the same total `src/timestep.rs`'s own
        // `irregular_frame_durations_total_the_expected_step_count` pins for
        // `FixedTimestep::advance` directly.
        let durations = [0.03, 0.12, 0.02, 0.11, 0.22];
        let total_steps: u32 = durations
            .iter()
            .map(|&d| grid.step(&mut timestep, d, &materials))
            .sum();

        assert_eq!(
            total_steps, 5,
            "the total steps Grid::step reports across several irregular calls \
             should match what the total real elapsed time earns"
        );
    }

    /// Scenario: a single call carrying less than one `dt` of real elapsed
    /// time must not advance the grid at all — `Grid::step` returns 0 and
    /// swap never runs, so a value written straight into `current` (not via
    /// a step) survives unchanged. Pins the same "no step yet" property
    /// `FixedTimestep`'s own tests pin, now at the `Grid`-stepping call
    /// site.
    #[test]
    fn grid_step_with_duration_under_one_dt_reports_zero_steps_and_grid_is_unchanged() {
        let materials = MaterialTable::reference();
        let mut grid = Grid::new(2, 2, AIR);
        let idx = GridIndex::new(0, 0);
        grid.set(idx, WATER);
        let mut timestep = FixedTimestep::new(0.1);

        let steps = grid.step(&mut timestep, 0.05, &materials);

        assert_eq!(
            steps, 0,
            "half a dt's worth of time should not yet elapse a step"
        );
        assert_eq!(
            grid.get(idx),
            WATER,
            "with zero steps elapsed, the grid must be entirely untouched"
        );
    }

    /// Scenario: a single call carrying several whole `dt`s of real elapsed
    /// time advances the grid by that many steps in one call, not by one —
    /// the step *count itself* is what this test pins (an all-air grid has
    /// nothing to move, so content is unobservable by value here; see the
    /// physics scenarios below for content-level assertions), mirroring
    /// `advance_tick_with_several_dts_advances_by_that_many_steps_and_colour_matches_parity`
    /// in `src/lib.rs`.
    #[test]
    fn grid_step_with_several_dts_in_one_call_advances_by_that_many_steps() {
        let materials = MaterialTable::reference();
        let mut grid = Grid::new(2, 2, AIR);
        let mut timestep = FixedTimestep::new(0.1);

        let steps = grid.step(&mut timestep, 0.35, &materials);

        assert_eq!(
            steps, 3,
            "0.35 / 0.1 should elapse 3 whole steps in a single Grid::step call"
        );
    }

    // --- Scenario: gravity/density physics (the movement rule) ---

    const SAND: MaterialId = MaterialId(3);

    /// Scenario: a genuinely resting arrangement — a sealed stone container
    /// (floor plus both side walls) full of water, air on top — stays
    /// exactly as it was after stepping forward by a real elapsed duration
    /// covering several fixed steps. Nothing here has anywhere less dense
    /// to move into, so every cell must read back exactly what it held
    /// before stepping, after every single step, not just the final one —
    /// "a resting pool stays flat" (`NORTH_STARS.md` #3), pinned as a true
    /// no-op rather than merely "looks flat in a screenshot".
    #[test]
    fn a_sealed_resting_pool_stays_exactly_unchanged_under_gravity() {
        let materials = MaterialTable::reference();
        // 4 wide x 4 tall: stone floor (j=0) and stone side walls (i=0,3)
        // for every row, water filling the interior at j=1..3, air on top.
        let mut grid = Grid::new(4, 4, AIR);
        for i in 0..4 {
            grid.set(GridIndex::new(i, 0), STONE);
        }
        for j in 0..4 {
            grid.set(GridIndex::new(0, j), STONE);
            grid.set(GridIndex::new(3, j), STONE);
        }
        for j in 1..3 {
            grid.set(GridIndex::new(1, j), WATER);
            grid.set(GridIndex::new(2, j), WATER);
        }

        let snapshot = |g: &Grid| -> Vec<MaterialId> {
            (0..4)
                .flat_map(|j| (0..4).map(move |i| GridIndex::new(i, j)))
                .map(|idx| g.get(idx))
                .collect()
        };
        let before = snapshot(&grid);

        let mut timestep = FixedTimestep::new(0.1);
        for _ in 0..5 {
            let steps = grid.step(&mut timestep, 0.1, &materials);
            assert_eq!(
                steps, 1,
                "each 0.1s call at dt=0.1 should elapse exactly one step"
            );
            assert_eq!(
                snapshot(&grid),
                before,
                "a sealed, already-flat pool has nothing less dense to move \
                 into, so it must stay exactly as it was after each and \
                 every step"
            );
        }
    }

    /// Scenario: a single grain of sand with only air below it falls
    /// exactly one cell per step (never more, never less) until it lands on
    /// the stone floor and then stays put — the basic granular-falls-under-
    /// gravity behaviour, checked step by step rather than only at the end
    /// so an off-by-one or a too-fast/too-slow fall would be caught.
    #[test]
    fn a_single_grain_of_sand_falls_one_cell_per_step_and_rests_on_the_floor() {
        let materials = MaterialTable::reference();
        let mut grid = Grid::new(1, 5, AIR);
        grid.set(GridIndex::new(0, 0), STONE); // floor
        grid.set(GridIndex::new(0, 4), SAND); // starts at the top

        let mut timestep = FixedTimestep::new(0.1);
        // Falls from j=4 to j=1 over three steps (one cell per step).
        for expected_j in [3, 2, 1] {
            grid.step(&mut timestep, 0.1, &materials);
            assert_eq!(
                grid.get(GridIndex::new(0, expected_j)),
                SAND,
                "sand should have fallen to j={expected_j}"
            );
        }
        // One more step: blocked by the stone floor at j=0, so it rests at
        // j=1 rather than trying to fall into stone.
        grid.step(&mut timestep, 0.1, &materials);
        assert_eq!(
            grid.get(GridIndex::new(0, 1)),
            SAND,
            "sand should rest on top of the stone floor, not move into it"
        );
        assert_eq!(grid.get(GridIndex::new(0, 0)), STONE, "floor stays stone");
    }

    /// Scenario: stone never moves under gravity, no matter what is beneath
    /// it (here: nothing — a lump of stone floating over open air) — pins
    /// `Phase::Solid`'s "never a mover" contract directly, independent of
    /// whether anything else in the grid is unstable.
    #[test]
    fn stone_never_falls_even_over_open_air() {
        let materials = MaterialTable::reference();
        let mut grid = Grid::new(1, 3, AIR);
        grid.set(GridIndex::new(0, 2), STONE);

        let mut timestep = FixedTimestep::new(0.1);
        for _ in 0..10 {
            grid.step(&mut timestep, 0.1, &materials);
        }
        assert_eq!(
            grid.get(GridIndex::new(0, 2)),
            STONE,
            "Solid-phase stone must never move, however many steps elapse"
        );
    }

    /// Scenario: a denser granular material sinks straight down through a
    /// column of a less-dense liquid, displacing it upward — sand (density
    /// 1.6) sinking through water (density 1.0) resting on a stone floor.
    /// Demonstrates the movement rule is genuinely density-driven (a
    /// `Liquid` cell can be a swap *target*, not just a mover) rather than
    /// only handling "falls into empty air".
    #[test]
    fn denser_sand_sinks_through_a_water_column_displacing_it_upward() {
        let materials = MaterialTable::reference();
        let mut grid = Grid::new(1, 4, AIR);
        grid.set(GridIndex::new(0, 0), STONE); // floor
        grid.set(GridIndex::new(0, 1), WATER);
        grid.set(GridIndex::new(0, 2), WATER);
        grid.set(GridIndex::new(0, 3), SAND); // starts above the water

        let mut timestep = FixedTimestep::new(0.1);
        for _ in 0..10 {
            grid.step(&mut timestep, 0.1, &materials);
        }

        // Sand ends up resting directly on the floor; the two water cells
        // end up displaced above it — same three materials, same counts,
        // just reordered by density.
        assert_eq!(grid.get(GridIndex::new(0, 0)), STONE);
        assert_eq!(
            grid.get(GridIndex::new(0, 1)),
            SAND,
            "sand should have sunk all the way to rest on the floor"
        );
        assert_eq!(grid.get(GridIndex::new(0, 2)), WATER);
        assert_eq!(
            grid.get(GridIndex::new(0, 3)),
            WATER,
            "both water cells should have been displaced upward by the \
             denser sinking sand"
        );
    }

    /// Scenario: "a column of water finds its level" (`NORTH_STARS.md` #3),
    /// pinned numerically rather than by eye. A stone container with a flat
    /// floor and side walls starts with all its water piled on one side (a
    /// tall column) and open air on the other (equal floor height, no
    /// water) — after enough steps, the water has spread so that both
    /// columns hold the same number of water cells (height 2 each out of 4
    /// total water cells across 2 columns), i.e. a level surface, not still
    /// piled on the original side.
    #[test]
    fn a_column_of_water_finds_its_level_across_an_open_container() {
        let materials = MaterialTable::reference();
        // 4 wide x 6 tall: stone floor and side walls; interior columns
        // i=1..3 are the open container (2 columns wide).
        let mut grid = Grid::new(4, 6, AIR);
        for i in 0..4 {
            grid.set(GridIndex::new(i, 0), STONE);
        }
        for j in 0..6 {
            grid.set(GridIndex::new(0, j), STONE);
            grid.set(GridIndex::new(3, j), STONE);
        }
        // All 4 water cells piled in column i=1; column i=2 starts empty
        // (air) at the same floor height.
        for j in 1..5 {
            grid.set(GridIndex::new(1, j), WATER);
        }

        let mut timestep = FixedTimestep::new(0.1);
        // Generously many steps: this is a cellular approximation of
        // hydrostatic levelling, not an instant pressure solve, so it needs
        // several passes to fully equalize a column this tall.
        for _ in 0..200 {
            grid.step(&mut timestep, 0.1, &materials);
        }

        let count_water_in_column = |grid: &Grid, i: i32| -> usize {
            (1..5)
                .filter(|&j| grid.get(GridIndex::new(i, j)) == WATER)
                .count()
        };
        let left = count_water_in_column(&grid, 1);
        let right = count_water_in_column(&grid, 2);
        assert_eq!(
            left + right,
            4,
            "all 4 water cells must still be present somewhere in the \
             container (conservation), just possibly redistributed"
        );
        assert_eq!(
            (left, right),
            (2, 2),
            "a column piled entirely on one side should have levelled to \
             an equal split across both columns, not stayed piled"
        );
    }

    /// Scenario: because every movement is a swap of two cells' contents,
    /// never a creation or deletion, the total count of cells holding each
    /// material is exactly conserved by construction — checked here across
    /// a busy scenario (sand falling through water in a container with
    /// air above) for many steps, not just the specific arrangements the
    /// scenarios above already happen to conserve.
    #[test]
    fn per_material_cell_counts_are_exactly_conserved_across_many_steps_of_real_physics() {
        let materials = MaterialTable::reference();
        let mut grid = Grid::new(5, 6, AIR);
        for i in 0..5 {
            grid.set(GridIndex::new(i, 0), STONE);
        }
        for j in 0..6 {
            grid.set(GridIndex::new(0, j), STONE);
            grid.set(GridIndex::new(4, j), STONE);
        }
        grid.set(GridIndex::new(1, 1), WATER);
        grid.set(GridIndex::new(2, 1), WATER);
        grid.set(GridIndex::new(3, 1), WATER);
        grid.set(GridIndex::new(1, 4), SAND);
        grid.set(GridIndex::new(2, 5), SAND);
        grid.set(GridIndex::new(3, 4), SAND);

        let count_of = |grid: &Grid, id: MaterialId| -> usize {
            (0..6)
                .flat_map(|j| (0..5).map(move |i| GridIndex::new(i, j)))
                .filter(|&idx| grid.get(idx) == id)
                .count()
        };
        let before = (
            count_of(&grid, AIR),
            count_of(&grid, WATER),
            count_of(&grid, STONE),
            count_of(&grid, SAND),
        );

        let mut timestep = FixedTimestep::new(0.1);
        for step_num in 0..60 {
            grid.step(&mut timestep, 0.1, &materials);
            let now = (
                count_of(&grid, AIR),
                count_of(&grid, WATER),
                count_of(&grid, STONE),
                count_of(&grid, SAND),
            );
            assert_eq!(
                now, before,
                "per-material cell counts must stay exactly conserved after \
                 step {step_num}, since movement is only ever a swap"
            );
        }
    }

    // --- Reference grid / performance budget ---

    /// Timing measurement, not a correctness scenario — `#[ignore]`d by
    /// default per the fast-path convention (`README.md`'s "Test tagging /
    /// fast path" section: a timing run is not "fast" in the unit-test
    /// sense). Run it explicitly with:
    ///
    /// ```sh
    /// cargo test --lib -- --ignored reference_grid_step_timing --nocapture
    /// ```
    ///
    /// (add `--release` before `--lib` for an optimized-build number).
    ///
    /// Builds the reference grid — `REFERENCE_GRID_WIDTH` x
    /// `REFERENCE_GRID_HEIGHT` cells — and times `MEASURED_STEPS` calls to
    /// the existing, unmodified `Grid::step`/`FixedTimestep` path, after
    /// `WARMUP_STEPS` untimed steps to let allocator/cache effects settle.
    /// Reports the average wall-clock time per step to stdout.
    ///
    /// The only assertion here is a generous sanity ceiling, not a recorded
    /// budget: this stays a measurement harness that can be re-run to check
    /// for a real regression, not a brittle timing-dependent pass/fail gate
    /// tied to one dev machine's exact number.
    const REFERENCE_GRID_WIDTH: usize = 1024;
    const REFERENCE_GRID_HEIGHT: usize = 1024;

    #[test]
    #[ignore]
    fn reference_grid_step_timing() {
        use std::time::Instant;

        const WARMUP_STEPS: u32 = 5;
        const MEASURED_STEPS: u32 = 50;
        const DT: Scalar = 1.0 / 60.0;

        let materials = MaterialTable::reference();
        let mut grid = Grid::new(REFERENCE_GRID_WIDTH, REFERENCE_GRID_HEIGHT, AIR);
        let mut timestep = FixedTimestep::new(DT);

        for _ in 0..WARMUP_STEPS {
            grid.step(&mut timestep, DT, &materials);
        }

        let start = Instant::now();
        for _ in 0..MEASURED_STEPS {
            grid.step(&mut timestep, DT, &materials);
        }
        let elapsed = start.elapsed();
        let per_step = elapsed / MEASURED_STEPS;

        println!(
            "reference grid {REFERENCE_GRID_WIDTH}x{REFERENCE_GRID_HEIGHT} \
             ({} cells): {MEASURED_STEPS} measured steps (after {WARMUP_STEPS} \
             warm-up steps) in {elapsed:?} total, average {per_step:?} per step",
            REFERENCE_GRID_WIDTH * REFERENCE_GRID_HEIGHT
        );

        assert!(
            per_step.as_secs_f64() < 1.0,
            "reference-grid step took {per_step:?} on average — far beyond a sane \
             sanity ceiling, suggesting a real regression rather than ordinary \
             machine variance"
        );
    }
}

