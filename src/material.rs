//! Material representation: `Material` is *data* describing a substance a
//! grid cell can be made of, and `MaterialTable` is a small, indexable
//! collection of such data.
//!
//! Every later grid behaviour (granular solids, liquids, pressure, gases,
//! temperature) adds *behaviour* keyed off a cell's material, and that
//! behaviour must read as "look up this material's data and act on it
//! generically" — never as a per-material `if material == Water { ... }
//! else if material == Stone { ... }` chain hard-coded into the simulation
//! step. Getting `Material` a real, growable shape now is what keeps that
//! promise honest later.
//!
//! Deliberately has no dependency on `web-sys`/`wasm-bindgen`, `grid`, or
//! any rendering concept — same reasoning `src/math.rs` and
//! `src/timestep.rs` give for their own independence: a material is data,
//! usable identically from native `cargo test`, a future wasm build, and
//! whatever GPU-side representation a future physics pipeline needs.

use crate::math::Scalar;

/// A material's physical phase: `Solid`/`Liquid`/`Gas`. This is not a place
/// for per-material special cases (that defeats the point of `Material`
/// being data); it exists so later behaviour can branch on *phase-level*
/// properties (does this cell flow, does it settle, does it diffuse heat
/// the way a gas does) without each caring which specific material a cell
/// holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Immovable under gravity — a wall/floor material. Never a mover in
    /// `src/grid.rs`'s physics step, and never a valid swap *target* either
    /// (a `Solid` cell blocks anything from displacing it) — see
    /// `Grid::try_move_cell`'s target check.
    Solid,
    /// A granular solid (e.g. sand): falls straight down or diagonally
    /// down-slope under gravity, same density-driven displacement rule as
    /// `Liquid`, but does not flow sideways to seek a level the way a
    /// liquid does — see `src/grid.rs`'s module doc comment for the physics
    /// rule this phase distinction exists to drive.
    Granular,
    /// Falls under gravity like `Granular`, and additionally flows
    /// sideways (once it cannot fall straight down or diagonally) to seek
    /// its own level — see `src/grid.rs`.
    Liquid,
    /// Currently treated as immobile background by `src/grid.rs`'s physics
    /// step (never a mover), though it can still be *displaced upward* when
    /// a denser `Granular`/`Liquid` cell swaps into its cell. Real gas
    /// buoyancy/diffusion is deliberately not implemented yet — see
    /// `NORTH_STARS.md`'s physics-then-chemistry-then-biology ordering;
    /// this is a physics-phase gap to close later, not a decision that gas
    /// doesn't move.
    Gas,
}

/// A material's physical properties, as plain data — never as a
/// per-material code path. Any later code that needs to know "how does this
/// cell behave" reads it off a `Material` value looked up from a
/// [`MaterialTable`], not off a hard-coded match on which material it is.
///
/// Fields are a stated minimum (density, viscosity, heat_capacity,
/// conductivity, phase, colour) plus evident room to grow: adding a field
/// later (e.g. a `dissolves_in`/`permeable` flag, deliberately not added
/// yet) is "add a field and a constructor argument", not a structural
/// rework.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Material {
    /// Mass per unit area (this is a 2D simulation), in the crate's
    /// otherwise-unspecified world units — no specific unit system is
    /// pinned yet (nothing downstream depends on one; that would be pinned
    /// once real mass is measured, if needed).
    pub density: Scalar,
    /// Resistance to flow, for later liquid/gas behaviour. Meaningless for
    /// `Phase::Solid` materials but still present — a uniform field beats a
    /// phase-conditional one.
    pub viscosity: Scalar,
    /// Energy required to raise this material's temperature, for later
    /// temperature behaviour.
    pub heat_capacity: Scalar,
    /// Rate at which this material conducts heat to neighbouring cells, for
    /// later temperature behaviour.
    pub conductivity: Scalar,
    /// This material's phase — see [`Phase`].
    pub phase: Phase,
    /// The colour this material paints as, as 8-bit sRGB — same
    /// `(u8, u8, u8)` convention `src/lib.rs`'s `RECT_COLOR_RGB` already
    /// uses, so the renderer needs no new colour representation.
    pub colour: (u8, u8, u8),
}

impl Material {
    /// Builds a material from its properties. Plain field construction, no
    /// decision to make, so implemented directly rather than stubbed — same
    /// reasoning `src/math.rs`'s `Vec2::new`/`GridIndex::new` apply to
    /// themselves.
    pub fn new(
        density: Scalar,
        viscosity: Scalar,
        heat_capacity: Scalar,
        conductivity: Scalar,
        phase: Phase,
        colour: (u8, u8, u8),
    ) -> Self {
        Material {
            density,
            viscosity,
            heat_capacity,
            conductivity,
            phase,
            colour,
        }
    }
}

/// A newtype index into a [`MaterialTable`], distinct from a bare integer so
/// a cell's material reference can't be silently confused with, say, a
/// `GridIndex` coordinate or a raw array offset.
///
/// Deliberately does not assume anything about how `MaterialTable` stores
/// its materials internally (direct-index `Vec` vs. something else) — that
/// mapping is `MaterialTable::get`'s decision, not this type's decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterialId(pub u16);

impl MaterialId {
    /// Builds a material id from its raw value. Plain field construction,
    /// no decision to make, so implemented directly — same reasoning as
    /// `Material::new`.
    pub fn new(value: u16) -> Self {
        MaterialId(value)
    }
}

/// A small, indexable collection of [`Material`] values — the "material
/// table as data, not per-material code paths" shape.
///
/// Deliberately holds owned `Material` values (not references) and exposes
/// them only through [`MaterialTable::get`] keyed by [`MaterialId`] — this
/// is the single seam every "look up this cell's material" operation goes
/// through, so it can be swapped (e.g. for a GPU-resident table, later)
/// without every caller changing.
pub struct MaterialTable {
    materials: Vec<Material>,
}

impl MaterialTable {
    /// Builds a table from an explicit list of materials. Plain field
    /// construction, no decision to make, so implemented directly — same
    /// reasoning as `Material::new`. `materials[n]`'s position in the
    /// vector is that material's [`MaterialId`] under the id convention —
    /// see [`MaterialTable::get`].
    pub fn new(materials: Vec<Material>) -> Self {
        MaterialTable { materials }
    }

    /// How many materials this table holds.
    pub fn len(&self) -> usize {
        self.materials.len()
    }

    /// Whether this table holds no materials at all.
    pub fn is_empty(&self) -> bool {
        self.materials.is_empty()
    }

    /// Looks up the material at `id`.
    ///
    /// How a [`MaterialId`] maps to a position in this table's storage
    /// (direct indexing into the backing `Vec`, or something else) is a
    /// choice, not plain plumbing — pinned by a test below.
    ///
    /// Panics if `id` does not name a material in this table.
    pub fn get(&self, id: MaterialId) -> &Material {
        &self.materials[id.0 as usize]
    }

    /// A reference material table: at least 2-3 distinct materials (e.g.
    /// "empty/air", "water", "stone") as data.
    ///
    /// Choosing the concrete property values (what density water has, what
    /// colour stone paints) is this function's actual content, not
    /// plumbing — the tests below pin the *shape* those values must have
    /// (distinct phases, at least three entries) without pinning the
    /// numbers themselves, leaving room to choose different sensible values
    /// later.
    ///
    /// **Note on units:** these numbers are plausible relative to
    /// each other (stone denser than water, water denser than empty/air;
    /// water's `heat_capacity` is its real specific heat in J/(g·K)) but are
    /// **not** drawn from one consistent unit system — `density` is an
    /// unpinned, effectively normalised-to-water quantity (per `Material`'s
    /// own doc comment), while `heat_capacity` borrows a real physical
    /// constant. Any later code computing actual energy (`heat_capacity *
    /// mass`) must not assume these compose dimensionally correctly without
    /// first pinning a real unit system for `density` — that pinning is
    /// still an open decision, not implied by this table.
    pub fn reference() -> Self {
        let empty = Material::new(0.0, 0.0, 1.0, 0.0, Phase::Gas, (0, 0, 0));
        let water = Material::new(1.0, 0.5, 4.186, 0.6, Phase::Liquid, (40, 90, 200));
        let stone = Material::new(2.5, 0.0, 0.8, 2.0, Phase::Solid, (120, 120, 120));
        // Sand: appended at id 3, after the original three — every existing
        // caller that hardcodes ids 0/1/2 (air/water/stone) stays correct.
        // Denser than water (so it sinks through it), Granular phase (falls
        // under gravity, does not flow sideways the way Liquid does) — see
        // `src/grid.rs`'s physics step.
        let sand = Material::new(1.6, 0.0, 0.83, 0.3, Phase::Granular, (194, 178, 128));
        MaterialTable::new(vec![empty, water, stone, sand])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Scenario: material table as data, not per-material code ---

    /// Scenario: a table built from an explicit list of materials returns
    /// each material, unchanged, when looked up by its position-derived id
    /// — the basic "store data, get the same data back" contract every
    /// "look up this cell's material" call relies on.
    #[test]
    fn material_table_get_returns_the_material_stored_at_its_id() {
        let water = Material::new(1.0, 0.5, 4.0, 0.6, Phase::Liquid, (40, 90, 200));
        let stone = Material::new(2.5, 0.0, 0.8, 2.0, Phase::Solid, (120, 120, 120));
        let table = MaterialTable::new(vec![water, stone]);

        assert_eq!(table.len(), 2);
        assert_eq!(*table.get(MaterialId::new(0)), water);
        assert_eq!(*table.get(MaterialId::new(1)), stone);
    }

    /// Scenario: the reference table holds at least three distinct
    /// materials — "a small material table... holding at least 2-3 distinct
    /// materials". Checks the count, not specific values (those are free to
    /// choose).
    #[test]
    fn reference_table_holds_at_least_three_materials() {
        let table = MaterialTable::reference();
        assert!(
            table.len() >= 3,
            "expected at least 3 materials (e.g. empty/air, water, stone), \
             got {}",
            table.len()
        );
    }

    /// Scenario: the reference table's materials are genuinely distinct
    /// data, not the same material repeated — spans at least a solid and a
    /// liquid phase, matching its own example ("empty/air, water, stone").
    /// Checks phase variety, not exact colours/densities.
    #[test]
    fn reference_table_spans_at_least_a_solid_and_a_liquid_phase() {
        let table = MaterialTable::reference();
        let phases: Vec<Phase> = (0..table.len())
            .map(|i| table.get(MaterialId::new(i as u16)).phase)
            .collect();

        assert!(
            phases.contains(&Phase::Solid),
            "expected at least one Solid-phase material (e.g. stone), got {phases:?}"
        );
        assert!(
            phases.contains(&Phase::Liquid),
            "expected at least one Liquid-phase material (e.g. water), got {phases:?}"
        );
    }

    // --- Disposable unit tests ---

    /// Disposable: `Material::new` assigns each argument to its matching
    /// field, in the declared order — a plumbing check on the constructor
    /// itself.
    #[test]
    fn material_new_assigns_every_field() {
        let m = Material::new(1.2, 3.4, 5.6, 7.8, Phase::Gas, (9, 8, 7));
        assert_eq!(m.density, 1.2);
        assert_eq!(m.viscosity, 3.4);
        assert_eq!(m.heat_capacity, 5.6);
        assert_eq!(m.conductivity, 7.8);
        assert_eq!(m.phase, Phase::Gas);
        assert_eq!(m.colour, (9, 8, 7));
    }

    /// Disposable: an empty table reports zero length and `is_empty()`.
    #[test]
    fn empty_table_reports_zero_length() {
        let table = MaterialTable::new(vec![]);
        assert_eq!(table.len(), 0);
        assert!(table.is_empty());
    }
}
