//! Shared math primitives for world-space / physics quantities: the scalar
//! numeric type and a 2D vector built on it.
//!
//! This module is M0.4's "small, boring substrate" (see the milestone's
//! intent in `cycle-log/tranche-0/m0.4/plan.md`): later tranches reach for
//! `Scalar` and `Vec2` rather than each inventing their own. Nothing here is
//! wired into rendering yet — `src/lib.rs`'s canvas code is untouched by this
//! module and still works entirely in its own pixel coordinates.

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
        let _ = other;
        unimplemented!("component-wise addition of self and other")
    }

    /// Vector subtraction: the displacement from `other` to `self`.
    pub fn sub(self, other: Vec2) -> Vec2 {
        let _ = other;
        unimplemented!("component-wise subtraction of other from self")
    }

    /// Scalar multiplication: uniformly scales both components by `s`.
    pub fn scale(self, s: Scalar) -> Vec2 {
        let _ = s;
        unimplemented!("component-wise multiplication of self by s")
    }

    /// Dot product: `self.x * other.x + self.y * other.y`. Positive when the
    /// two vectors point in a broadly similar direction, zero when they are
    /// perpendicular, negative when broadly opposed.
    pub fn dot(self, other: Vec2) -> Scalar {
        let _ = other;
        unimplemented!("dot product of self and other")
    }
}

/// The "up" direction under this module's coordinate convention: `+y`.
///
/// A real value, not a stub — it is a constant declaration, not an
/// algorithm. Deliberately distinct from the canvas convention pinned in
/// `src/lib.rs`, where `+y` is down. Exists specifically so the convention
/// pinned above is backed by something a test can assert on: see
/// `up_convention_pins_math_physics_y_up_not_canvas_y_down` below.
pub const UP: Vec2 = Vec2 { x: 0.0, y: 1.0 };

#[cfg(test)]
mod tests {
    use super::*;

    // --- Convention pin (durable scenario, cross-cutting) ---

    /// Pins the coordinate convention itself, independent of the arithmetic
    /// below: `UP` must point in `+y`. If a future change silently flipped
    /// this module's convention to match the canvas (`+y` down) instead of
    /// math/physics (`+y` up) — e.g. by redefining `UP` as `(0.0, -1.0)` —
    /// this assertion is what would catch it, not just the doc comment.
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
}
