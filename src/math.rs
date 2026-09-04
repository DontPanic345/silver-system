//! Shared math primitives for world-space / physics quantities: the scalar
//! numeric type, a 2D vector, and an integer grid-cell index built on them.
//!
//! This module is M0.4's "small, boring substrate" (see the milestone's
//! intent in `cycle-log/tranche-0/m0.4/plan.md`): later tranches reach for
//! `Scalar`, `Vec2` and `GridIndex` rather than each inventing their own.
//!
//! **Update, M1.1 round 1:** `Vec2` and `GridIndex` now have a real caller —
//! `src/grid.rs`'s `Grid`, which is addressed by `GridIndex` throughout and
//! whose `Grid::cell_center` delegates to `GridIndex::center` (returning a
//! `Vec2`). Until this round they were called only by their own unit tests
//! below; `Scalar` and `FixedTimestep` (`src/timestep.rs`) were already
//! exercised by real code (`src/lib.rs`'s `tick_and_draw`). This was found
//! and named, not silently dropped, in tranche 0's tranche-scope refactor
//! pass: forcing a grid-cell index into M0.1's static two-colour rectangle
//! to technically satisfy the target would have been exactly the kind of
//! artificial, paper-exercise wiring the tranche's own reach notes
//! (`cycle-log/tranche-0/plan.md` §2) warned against for the hello-world's
//! *animation* — `GridIndex` needed a real grid to be a genuine use, and
//! M1.1 is that grid. The `#[allow(dead_code)]` attributes that recorded
//! the old, unused state are removed below accordingly.

/// The scalar type every world-space / physics quantity in this crate is
/// expressed in.
///
/// **Decision: `f32`, not `f64` or fixed-point.**
///
/// Reasoning, not a default:
///
/// - `CLAUDE.md` states GPU execution is the intended direction for this
///   project. `f32` is the GPU-native float: GPUs (and WebGPU/WGSL in
///   particular, the path a wasm/browser target like this crate would take)
///   either lack `f64` support or run it far slower than `f32` — choosing
///   `f64` now would mean re-deciding this later under migration pressure
///   instead of once, here.
/// - `CLAUDE.md` also states bit-identical determinism is
///   architecture-contingent, not a standing requirement, and that
///   conservation only needs to hold to a *stated tolerance*. That is exactly
///   the deal `f32` asks for: it does not offer `f64`'s extra headroom, but
///   nothing in this project's current requirements spends that headroom.
/// - Fixed-point buys deterministic cross-platform reproducibility at the
///   cost of range, ergonomics, and every downstream library (rendering,
///   eventually a physics/GPU pipeline) expecting floats. Nothing in the
///   current plan asks for that trade; it can be revisited if a later
///   tranche's target specifically demands it.
/// - `f32` halves the memory traffic of `f64` for the same data, which matters
///   once this substrate is under a "sufficiently full" large universe (the
///   north star) rather than a handful of values.
///
/// If a later milestone's target needs `f64`'s precision (e.g. an
/// accumulating quantity over a very long run proves `f32` drifts past its
/// tolerance), that is a new decision to make there, in the open, against a
/// measured failure — not a silent reversal of this one.
pub type Scalar = f32;

/// A 2D vector of [`Scalar`]s, used for world-space positions, velocities,
/// forces, and displacements.
///
/// ## Coordinate convention
///
/// `+y` is **up** — the standard math/physics convention. This is the
/// opposite of the canvas convention already pinned in `src/lib.rs`, where
/// the origin is the canvas's top-left corner and `+y` is *down*. `Vec2` is
/// the world-space type; the canvas constants in `src/lib.rs` are the
/// pixel-space type. Anything that later maps a `Vec2` onto the canvas must
/// flip the sign of `y` explicitly at that boundary — this convention does
/// not assume that flip away, and neither should any future caller.
///
/// See [`UP`] and the `up_convention_*` test below: they exist so a future
/// change that silently swapped this convention (e.g. redefining "up" as
/// `-y` to match the canvas instead) would fail a test, not just a doc
/// comment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    pub x: Scalar,
    pub y: Scalar,
}

impl Vec2 {
    /// Builds a vector from its components. Plain field construction, no
    /// decision to make, so implemented directly rather than stubbed (same
    /// reasoning `src/lib.rs` applies to its own plumbing getters).
    pub fn new(x: Scalar, y: Scalar) -> Self {
        Vec2 { x, y }
    }

    /// Vector addition: combines two displacements (or a point and a
    /// displacement) into one.
    pub fn add(self, other: Vec2) -> Vec2 {
        Vec2 {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }

    /// Vector subtraction: the displacement from `other` to `self`.
    pub fn sub(self, other: Vec2) -> Vec2 {
        Vec2 {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }

    /// Scalar multiplication: uniformly scales both components by `s`.
    pub fn scale(self, s: Scalar) -> Vec2 {
        Vec2 {
            x: self.x * s,
            y: self.y * s,
        }
    }

    /// Dot product: `self.x * other.x + self.y * other.y`. Positive when the
    /// two vectors point in a broadly similar direction, zero when they are
    /// perpendicular, negative when broadly opposed.
    pub fn dot(self, other: Vec2) -> Scalar {
        self.x * other.x + self.y * other.y
    }
}

/// The "up" direction under this module's coordinate convention: `+y`.
///
/// A real value, not a stub — it is a constant declaration, not an
/// algorithm. Deliberately distinct from the canvas convention pinned in
/// `src/lib.rs`, where `+y` is down. Exists specifically so the convention
/// pinned above is backed by something a test can assert on: see
/// `up_convention_pins_math_physics_y_up_not_canvas_y_down` below.
#[allow(dead_code)] // see the module doc comment: real production use lands in M1.
pub const UP: Vec2 = Vec2 { x: 0.0, y: 1.0 };

/// An integer `(i, j)` grid-cell coordinate.
///
/// ## Indexing convention: cell-center, not corner
///
/// **Decision (M0.4 round 2): index `(i, j)` denotes a specific grid
/// *cell*, and that cell's world-space position — the value
/// [`GridIndex::center`] returns — is the cell's *center*, not any of its
/// corners.**
///
/// Concretely, under a uniform `cell_size`, cell `(i, j)`'s center sits at
/// world position:
///
/// ```text
/// x = (i as Scalar + 0.5) * cell_size
/// y = (j as Scalar + 0.5) * cell_size
/// ```
///
/// So cell `(0, 0)`'s center is offset **half a cell** from the world
/// origin in both axes — not coincident with it. The world origin
/// `(0.0, 0.0)` is cell `(0, 0)`'s bottom-left *corner* under this
/// convention, never returned by `center` itself. This is the grid-origin
/// convention this milestone pins: cell `(0, 0)` is the cell whose
/// lower-value corner touches the world origin.
///
/// `j` increases in the `+y` direction, matching this module's math/physics
/// `+y`-is-up convention (see [`Vec2`]'s doc comment and [`UP`]) — not the
/// canvas's `+y`-is-down convention used in `src/lib.rs`. Anything that maps
/// a `GridIndex` onto the canvas later must account for both that flip and
/// this type's own cell-size scaling explicitly.
///
/// ## Cell-center is a default, not the only layout
///
/// Physics quantities this crate will accumulate (density, temperature,
/// pressure) are conventionally stored cell-centered, which is why this is
/// the default M0.4 ships. A staggered/compact grid — where some
/// quantities (e.g. velocity components in a MAC grid) live on cell faces
/// or corners instead — is a later, *additive* decision explicitly deferred
/// to M1.4 (see `PLAN.md`), to be made there on evidence from the solver
/// that needs it. `GridIndex` and `center` below do not need to anticipate
/// that decision; a staggered variant can be added alongside this one
/// without changing what this one guarantees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridIndex {
    pub i: i32,
    pub j: i32,
}

impl GridIndex {
    /// Builds a grid index from its integer cell coordinates. Plain field
    /// construction, no decision to make, so implemented directly rather
    /// than stubbed (same reasoning `Vec2::new` applies to itself).
    pub fn new(i: i32, j: i32) -> Self {
        GridIndex { i, j }
    }

    /// Converts this grid index to the world-space position of its cell's
    /// *center*, under a uniform `cell_size` and the cell-center convention
    /// documented on [`GridIndex`] itself.
    ///
    /// `cell_size` is assumed to be a positive world-space cell dimension.
    /// This function does not validate that assumption (no other primitive
    /// in this module validates its inputs either); a zero or negative
    /// `cell_size` produces arithmetically-consistent but physically
    /// meaningless output rather than a panic.
    pub fn center(self, cell_size: Scalar) -> Vec2 {
        Vec2::new(
            (self.i as Scalar + 0.5) * cell_size,
            (self.j as Scalar + 0.5) * cell_size,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Convention pin (durable scenario, cross-cutting) ---

    /// Pins the coordinate convention itself, independent of the arithmetic
    /// below: `UP` must point in `+y`. If a future change silently flipped
    /// this module's convention to match the canvas (`+y` down) instead of
    /// math/physics (`+y` up) — e.g. by redefining `UP` as `(0.0, -1.0)` —
    /// this assertion is what would catch it, not just the doc comment.
    // Clippy sees `UP.y` as a compile-time constant and suggests moving this
    // into a `const { assert!(..) }` block — that would turn a broken
    // convention into a compile error somewhere else in the crate instead of
    // a failing test right here, which defeats the point of pinning it as a
    // test. Deliberately kept as a runtime assertion.
    #[allow(clippy::assertions_on_constants)]
    #[test]
    fn up_convention_pins_math_physics_y_up_not_canvas_y_down() {
        assert!(
            UP.y > 0.0,
            "UP.y should be positive under this module's math/physics \
             convention (+y is up); src/lib.rs's canvas convention is the \
             opposite (+y is down) and must not leak in here"
        );
        assert_eq!(UP, Vec2::new(0.0, 1.0));
    }

    /// Scenario: moving a point "up" by the pinned `UP` vector increases its
    /// `y`, matching the convention documented on `Vec2`. Exercises `add`
    /// together with the convention pin above, so a future change that broke
    /// either the arithmetic or the convention would fail here.
    #[test]
    fn moving_up_by_the_pinned_up_vector_increases_y() {
        let origin = Vec2::new(0.0, 0.0);
        let moved = origin.add(UP);
        assert!(
            moved.y > origin.y,
            "adding UP should increase y (this module's 'up'), got {moved:?} \
             from origin {origin:?}"
        );
    }

    // --- Arithmetic scenarios (durable: restate the round's goal 2) ---

    /// Scenario: two displacements in sequence combine into one displacement
    /// equal to their sum — the everyday use `add` exists for.
    #[test]
    fn add_combines_two_displacements_into_one() {
        let first_leg = Vec2::new(3.0, 1.0);
        let second_leg = Vec2::new(-1.0, 4.0);
        let total = first_leg.add(second_leg);
        assert_eq!(total, Vec2::new(2.0, 5.0));
    }

    /// Scenario: the displacement from one point to another, computed via
    /// `sub`, added back to the start point, returns to the end point — the
    /// relationship `sub` and `add` are expected to have.
    #[test]
    fn sub_gives_the_displacement_that_add_can_undo() {
        let start = Vec2::new(5.0, -2.0);
        let end = Vec2::new(1.0, 3.0);
        let displacement = end.sub(start);
        assert_eq!(displacement, Vec2::new(-4.0, 5.0));
        assert_eq!(start.add(displacement), end);
    }

    /// Scenario: scaling a displacement by a factor stretches (or shrinks,
    /// or reverses) it uniformly in both components — the behaviour later
    /// tranches rely on for things like "half the velocity" or "double the
    /// force".
    #[test]
    fn scale_stretches_both_components_uniformly() {
        let velocity = Vec2::new(2.0, -3.0);
        assert_eq!(velocity.scale(2.0), Vec2::new(4.0, -6.0));
        assert_eq!(velocity.scale(0.5), Vec2::new(1.0, -1.5));
        assert_eq!(velocity.scale(-1.0), Vec2::new(-2.0, 3.0));
    }

    /// Scenario: perpendicular vectors have zero dot product — the
    /// "unrelated directions" case later tranches will use `dot` to detect
    /// (e.g. is this force doing any work along this direction of motion?).
    #[test]
    fn dot_is_zero_for_perpendicular_vectors() {
        let right = Vec2::new(1.0, 0.0);
        let up = Vec2::new(0.0, 1.0);
        assert_eq!(right.dot(up), 0.0);
    }

    /// Scenario: two vectors pointing the same way have a positive dot
    /// product equal to the product of their lengths along that shared
    /// axis — the "aligned" case later tranches will use `dot` to detect.
    #[test]
    fn dot_is_positive_for_aligned_vectors() {
        let a = Vec2::new(2.0, 0.0);
        let b = Vec2::new(3.0, 0.0);
        assert_eq!(a.dot(b), 6.0);
    }

    // --- Disposable unit tests (may be deleted if the implementation changes shape) ---

    /// Disposable: scaling by zero collapses a vector to the origin.
    #[test]
    fn scale_by_zero_collapses_to_origin() {
        let v = Vec2::new(7.0, -3.0);
        assert_eq!(v.scale(0.0), Vec2::new(0.0, 0.0));
    }

    /// Disposable: dot product is commutative (`a.dot(b) == b.dot(a)`), a
    /// property test rather than a scenario.
    #[test]
    fn dot_is_commutative() {
        let a = Vec2::new(1.5, -2.0);
        let b = Vec2::new(-3.0, 4.0);
        assert_eq!(a.dot(b), b.dot(a));
    }

    // --- GridIndex: convention pins (durable scenarios, cross-cutting) ---

    /// Pins the cell-*center* (not corner) convention stated in
    /// `GridIndex`'s doc comment: cell `(0, 0)`'s center sits half a cell
    /// away from the world origin in both axes, not coincident with it. If
    /// a future change silently switched this to a corner convention (index
    /// `(0,0)` maps straight to world `(0,0)`), this is what would catch
    /// it, not just the doc comment.
    #[test]
    fn grid_zero_zero_center_is_offset_half_a_cell_from_world_origin() {
        let cell_size = 2.0;
        let center = GridIndex::new(0, 0).center(cell_size);
        assert_eq!(
            center,
            Vec2::new(1.0, 1.0),
            "cell (0,0)'s center should sit at (cell_size/2, cell_size/2) \
             under the cell-center convention, not at the world origin \
             (which would imply a corner convention instead)"
        );
        assert_ne!(
            center,
            Vec2::new(0.0, 0.0),
            "cell (0,0)'s center must not coincide with the world origin — \
             that would be a corner convention, not this module's pinned \
             cell-center one"
        );
    }

    /// Scenario: stepping the grid index by one cell along `i` moves the
    /// cell-center position by exactly one `cell_size` along world `x`, and
    /// leaves `y` unchanged — the "index spacing equals cell size" property
    /// goal 4 asks to be pinned, exercised along the `i`/`x` axis.
    #[test]
    fn adjacent_indices_along_i_are_exactly_one_cell_size_apart_in_x() {
        let cell_size = 3.0;
        let here = GridIndex::new(4, 4).center(cell_size);
        let one_over = GridIndex::new(5, 4).center(cell_size);
        assert_eq!(one_over.sub(here), Vec2::new(cell_size, 0.0));
    }

    /// Scenario: stepping the grid index by one cell along `j` moves the
    /// cell-center position by exactly one `cell_size` along world `y`, in
    /// the `+y` direction — pinning both the spacing and that `j` follows
    /// this module's `+y`-up convention (not the canvas's `+y`-down one).
    #[test]
    fn adjacent_indices_along_j_are_exactly_one_cell_size_apart_in_plus_y() {
        let cell_size = 3.0;
        let here = GridIndex::new(4, 4).center(cell_size);
        let one_up = GridIndex::new(4, 5).center(cell_size);
        let step = one_up.sub(here);
        assert_eq!(step, Vec2::new(0.0, cell_size));
        assert!(
            step.y > 0.0,
            "incrementing j should move the center in +y (this module's \
             'up'), matching Vec2's pinned y-up convention"
        );
    }

    // --- GridIndex: disposable unit tests ---

    /// Disposable: a different cell size scales the center position
    /// linearly, exercised at a cell size other than the round numbers used
    /// above, and at a non-origin index.
    #[test]
    fn center_scales_linearly_with_cell_size_away_from_the_origin_cell() {
        let idx = GridIndex::new(2, 3);
        assert_eq!(idx.center(1.0), Vec2::new(2.5, 3.5));
        assert_eq!(idx.center(10.0), Vec2::new(25.0, 35.0));
    }

    /// Disposable: negative indices (cells "below"/"left of" the origin
    /// cell) still follow the same `(i + 0.5) * cell_size` rule, with no
    /// special-casing around zero.
    #[test]
    fn center_handles_negative_indices_consistently() {
        let cell_size = 2.0;
        assert_eq!(
            GridIndex::new(-1, -1).center(cell_size),
            Vec2::new(-1.0, -1.0)
        );
    }
}
