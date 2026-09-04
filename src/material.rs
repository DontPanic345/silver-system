//! Material representation: `Material` is *data* describing a substance a
//! grid cell can be made of, and `MaterialTable` is a small, indexable
//! collection of such data.
//!
//! This module is M1.1 round 1's goal 2 (see
//! `cycle-log/tranche-1/m1.1/round-01.md`): every later milestone in
//! tranche 1 (granular solids, liquids, pressure, gases, temperature) adds
//! *behaviour* keyed off a cell's material, and that behaviour must read as
//! "look up this material's data and act on it generically" — never as a
//! per-material `if material == Water { ... } else if material == Stone
//! { ... }` chain hard-coded into the simulation step. Getting `Material` a
//! real, growable shape now is what keeps that promise honest later.
//!
//! Deliberately has no dependency on `web-sys`/`wasm-bindgen`, `grid`, or
//! any rendering concept — same reasoning `src/math.rs` and
//! `src/timestep.rs` give for their own independence: a material is data,
//! usable identically from native `cargo test`, a future wasm build, and
//! whatever GPU-side representation M1.7 eventually needs.

use crate::math::Scalar;

/// A material's physical phase. At minimum `Solid`/`Liquid`/`Gas`, per this
/// round's goal 2 — this is not a place for per-material special cases
/// (that defeats the point of `Material` being data); it exists so later
/// tranche-1 rounds can branch on *phase-level* behaviour (does this cell
/// flow, does it settle, does it diffuse heat the way a gas does) without
/// each caring which specific material a cell holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Solid,
    Liquid,
    Gas,
}

/// A material's physical properties, as plain data — never as a
/// per-material code path. Every later tranche-1 round that needs to know
/// "how does this cell behave" reads it off a `Material` value looked up
/// from a [`MaterialTable`], not off a hard-coded match on which material it
/// is.
///
/// Fields are round 1's stated minimum (density, viscosity, heat_capacity,
/// conductivity, phase, colour) plus evident room to grow: adding a field
/// later (e.g. tranche 2's `dissolves_in`/`permeable`, explicitly deferred —
/// see `cycle-log/tranche-1/m1.1/plan.md` §2) is "add a field and a
/// constructor argument", not a structural rework.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Material {
    /// Mass per unit area (this is a 2D simulation), in the crate's
    /// otherwise-unspecified world units — no specific unit system is
    /// pinned yet (nothing downstream depends on one this round; the first
    /// round that measures real mass, M1.1 round 3, is where that would be
    /// pinned if needed).
    pub density: Scalar,
    /// Resistance to flow, for later liquid/gas rounds. Meaningless for
    /// `Phase::Solid` materials but still present — a uniform field beats a
    /// phase-conditional one.
    pub viscosity: Scalar,
    /// Energy required to raise this material's temperature, for later
    /// temperature rounds.
    pub heat_capacity: Scalar,
    /// Rate at which this material conducts heat to neighbouring cells, for
    /// later temperature rounds.
    pub conductivity: Scalar,
    /// This material's phase — see [`Phase`].
    pub phase: Phase,
    /// The colour this material paints as, for the later renderer (M1.1
    /// round 4), as 8-bit sRGB — same `(u8, u8, u8)` convention
    /// `src/lib.rs`'s `RECT_COLOR_RGB` already uses, so the renderer needs
    /// no new colour representation when it arrives.
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
/// mapping is `MaterialTable::get`'s decision, not this type's.
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
/// table as data, not per-material code paths" round 1's goal 2 asks for.
///
/// Deliberately holds owned `Material` values (not references) and exposes
/// them only through [`MaterialTable::get`] keyed by [`MaterialId`] — this
/// is the single seam every later round's "look up this cell's material"
/// operation goes through, so it can be swapped (e.g. for a GPU-resident
/// table, M1.7) without every caller changing.
pub struct MaterialTable {
    materials: Vec<Material>,
}

impl MaterialTable {
    /// Builds a table from an explicit list of materials. Plain field
    /// construction, no decision to make, so implemented directly — same
    /// reasoning as `Material::new`. `materials[n]`'s position in the
    /// vector is that material's [`MaterialId`] under this round's id
    /// convention — see [`MaterialTable::get`] for why that convention
    /// itself is left for Green to commit to, not assumed here.
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
    /// This is the round's real decision, left as a stub for Green: how a
    /// [`MaterialId`] maps to a position in this table's storage (direct
    /// indexing into the backing `Vec`, or something else) is a choice, not
    /// plain plumbing — pinned by a test below rather than assumed here.
    ///
    /// Panics if `id` does not name a material in this table.
    pub fn get(&self, id: MaterialId) -> &Material {
        let _ = id;
        todo!("look up the material stored at `id` in this table")
    }

    /// The reference material table this round's goal 2 asks for: at least
    /// 2-3 distinct materials (e.g. "empty/air", "water", "stone") as data.
    ///
    /// Left as a stub for Green: choosing the concrete property values
    /// (what density water has, what colour stone paints) is this round's
    /// actual content, not plumbing — the tests below pin the *shape* those
    /// values must have (distinct phases, at least three entries) without
    /// pinning the numbers themselves, so Green has real room to choose
    /// sensible values.
    pub fn reference() -> Self {
        todo!(
            "build the reference material table: at least an empty/air, a \
             water, and a stone material, as data"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Scenario: material table as data, not per-material code (goal 2) ---

    /// Scenario: a table built from an explicit list of materials returns
    /// each material, unchanged, when looked up by its position-derived id
    /// — the basic "store data, get the same data back" contract every
    /// later round's "look up this cell's material" call relies on.
    #[test]
    fn material_table_get_returns_the_material_stored_at_its_id() {
        let water = Material::new(1.0, 0.5, 4.0, 0.6, Phase::Liquid, (40, 90, 200));
        let stone = Material::new(2.5, 0.0, 0.8, 2.0, Phase::Solid, (120, 120, 120));
        let table = MaterialTable::new(vec![water, stone]);

        assert_eq!(table.len(), 2);
        assert_eq!(*table.get(MaterialId::new(0)), water);
        assert_eq!(*table.get(MaterialId::new(1)), stone);
    }

    /// Scenario: the reference table round 1's goal 2 asks for holds at
    /// least three distinct materials — "a small material table... holding
    /// at least 2-3 distinct materials". Checks the count, not specific
    /// values (those are Green's to choose).
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
    /// liquid phase, matching the round's own example ("empty/air, water,
    /// stone"). Checks phase variety, not exact colours/densities.
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
