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
//! results into `next` via [`Grid::set_next`] without disturbing `current`
//! mid-step — every cell's next state is computed from a consistent,
//! unchanging view of the current one — and then calls [`Grid::swap`] once
//! the whole step is done, making the written state the new `current`.
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

use crate::material::MaterialId;
use crate::math::{GridIndex, Scalar, Vec2};
use crate::timestep::FixedTimestep;

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
    /// many times.
    ///
    /// The *mechanism* of stepping is real elapsed-time accounting (not a
    /// bare per-call `+1`), wired to the existing, unmodified
    /// `FixedTimestep`. The per-cell transformation each elapsed step
    /// applies is deliberately the identity — see [`Grid::step_once`].
    pub fn step(&mut self, timestep: &mut FixedTimestep, frame_duration_secs: Scalar) -> u32 {
        let steps = timestep.advance(frame_duration_secs);
        for _ in 0..steps {
            self.step_once();
        }
        steps
    }

    /// Applies exactly one fixed step: for every cell, writes `next` from
    /// `current` (via [`Grid::get`]/[`Grid::set_next`], never touching
    /// `current` mid-step) and then [`Grid::swap`]s once the whole pass is
    /// done.
    ///
    /// **The per-cell transformation is currently the identity** — every
    /// cell's next value is exactly its current value, unchanged. No
    /// gravity, no transport, no material behaviour yet: that is later
    /// work's job. This function exists so real per-cell physics can
    /// replace the body of the inner loop without changing the stepping
    /// mechanism ([`Grid::step`], [`FixedTimestep`] wiring) around it at
    /// all.
    fn step_once(&mut self) {
        for j in 0..self.height as i32 {
            for i in 0..self.width as i32 {
                let idx = GridIndex::new(i, j);
                let current_value = self.get(idx);
                self.set_next(idx, current_value);
            }
        }
        self.swap();
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
        let mut grid = Grid::new(2, 2, AIR);
        let mut timestep = FixedTimestep::new(0.1);

        // Durations: 0.03, 0.12, 0.02, 0.11, 0.22 -> total 0.50 -> 5 steps,
        // the same total `src/timestep.rs`'s own
        // `irregular_frame_durations_total_the_expected_step_count` pins for
        // `FixedTimestep::advance` directly.
        let durations = [0.03, 0.12, 0.02, 0.11, 0.22];
        let total_steps: u32 = durations.iter().map(|&d| grid.step(&mut timestep, d)).sum();

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
        let mut grid = Grid::new(2, 2, AIR);
        let idx = GridIndex::new(0, 0);
        grid.set(idx, WATER);
        let mut timestep = FixedTimestep::new(0.1);

        let steps = grid.step(&mut timestep, 0.05);

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
    /// exercised by counting swaps indirectly: after a call worth 3 steps,
    /// `current`/`next` have been exchanged an odd number of times overall,
    /// which for a 3-step call from a fresh grid means the *contents* match
    /// (identity transform, so unobservable directly by value) but the step
    /// *count itself* is what this test pins, mirroring
    /// `advance_tick_with_several_dts_advances_by_that_many_steps_and_colour_matches_parity`
    /// in `src/lib.rs`.
    #[test]
    fn grid_step_with_several_dts_in_one_call_advances_by_that_many_steps() {
        let mut grid = Grid::new(2, 2, AIR);
        let mut timestep = FixedTimestep::new(0.1);

        let steps = grid.step(&mut timestep, 0.35);

        assert_eq!(
            steps, 3,
            "0.35 / 0.1 should elapse 3 whole steps in a single Grid::step call"
        );
    }

    // --- Scenario: stepping the identity transformation is a true no-op ---

    /// Scenario: a resting scenario — several distinct materials placed in
    /// several cells, nothing moving — stays exactly as it was after
    /// stepping forward by a real elapsed duration covering several fixed
    /// steps. The current step content is the identity transform (no
    /// gravity, no transport, no material behaviour), so every cell must
    /// read back exactly what it held before stepping, for every step in
    /// between — not just the final one.
    #[test]
    fn stepping_the_identity_transformation_leaves_a_resting_scenario_unchanged() {
        let mut grid = Grid::new(3, 3, AIR);
        grid.set(GridIndex::new(0, 0), STONE);
        grid.set(GridIndex::new(1, 1), WATER);
        grid.set(GridIndex::new(2, 2), STONE);

        let snapshot = |g: &Grid| -> Vec<MaterialId> {
            (0..3)
                .flat_map(|j| (0..3).map(move |i| GridIndex::new(i, j)))
                .map(|idx| g.get(idx))
                .collect()
        };
        let before = snapshot(&grid);

        let mut timestep = FixedTimestep::new(0.1);
        // Several fixed steps' worth of real elapsed time, split across
        // multiple calls, so the no-op property is checked after every
        // single step, not just after one big jump.
        for _ in 0..5 {
            let steps = grid.step(&mut timestep, 0.1);
            assert_eq!(
                steps, 1,
                "each 0.1s call at dt=0.1 should elapse exactly one step"
            );
            assert_eq!(
                snapshot(&grid),
                before,
                "the identity transform must leave every cell exactly as it was, \
                 after each and every step"
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

        let mut grid = Grid::new(REFERENCE_GRID_WIDTH, REFERENCE_GRID_HEIGHT, AIR);
        let mut timestep = FixedTimestep::new(DT);

        for _ in 0..WARMUP_STEPS {
            grid.step(&mut timestep, DT);
        }

        let start = Instant::now();
        for _ in 0..MEASURED_STEPS {
            grid.step(&mut timestep, DT);
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
