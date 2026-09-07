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

/// How a material moves under gravity. Derived by default from
/// [`Phase`] (solids sit still, liquids and gases flow) but separable from
/// it, because sand is a solid that falls and piles — a distinction phase
/// alone cannot express.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mobility {
    /// Never moves on its own: stone, ice, a juniper bush.
    Static,
    /// Falls straight down and slides diagonally, but does not level out:
    /// sand.
    Granular,
    /// Falls (or rises, if lighter than what's above it) and spreads
    /// sideways to find its level: water, lava, air, steam.
    Flowing,
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
    /// Enthalpy offset: the energy per unit mass this material holds *over
    /// and above* its sensible heat (`mass * heat_capacity * temperature`),
    /// measured against an arbitrary shared zero.
    ///
    /// This is how latent heat becomes data instead of code. Total world
    /// energy is `Σ mass * (heat_capacity * T + latent_energy)`, so a phase
    /// change that debits the sensible term by exactly the enthalpy gap
    /// between the two materials conserves that total by construction — see
    /// `physics::apply_phase_changes`.
    ///
    /// **Do not set this by hand on a material that takes part in a
    /// transition.** [`MaterialTable::with_transitions`] solves for it from
    /// the transitions' real latent heats; see that method for why a
    /// hand-written value is almost always subtly wrong.
    pub latent_energy: Scalar,
    /// How this material moves under gravity — see [`Mobility`].
    pub mobility: Mobility,
    /// Whether a gnome standing in this material can breathe it. Air and
    /// steam are breathable-ish; water, stone and lava are not. Used only
    /// by the gnome layer, but it lives here for the same reason every
    /// other property does: no per-material `if` chains elsewhere.
    pub breathable: bool,
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
            latent_energy: 0.0,
            mobility: match phase {
                Phase::Solid => Mobility::Static,
                Phase::Liquid | Phase::Gas => Mobility::Flowing,
            },
            breathable: false,
        }
    }

    /// Overrides [`Material::mobility`], builder-style.
    pub fn with_mobility(mut self, mobility: Mobility) -> Self {
        self.mobility = mobility;
        self
    }

    /// Sets [`Material::latent_energy`], builder-style.
    pub fn with_latent_energy(mut self, latent_energy: Scalar) -> Self {
        self.latent_energy = latent_energy;
        self
    }

    /// Marks this material breathable, builder-style.
    pub fn breathable(mut self) -> Self {
        self.breathable = true;
        self
    }

    /// Energy per unit mass held by this material at `temperature`:
    /// sensible heat plus this material's enthalpy offset. The quantity
    /// every conservation check in this crate sums.
    pub fn specific_energy(&self, temperature: Scalar) -> Scalar {
        self.heat_capacity * temperature + self.latent_energy
    }
}

/// The direction a [`Transition`] fires in: on the way up in temperature
/// (melting, boiling) or on the way down (freezing, condensing).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Heating,
    Cooling,
}

/// A data-driven phase change: "material `from` becomes material `to` when
/// its temperature crosses `threshold_k` in `direction`".
///
/// `latent_heat` is the real physical figure (J/g absorbed going `from` →
/// `to`, so negative for a cooling transition, which releases it). The
/// per-material [`Material::latent_energy`] offsets that make total-energy
/// bookkeeping work out are *derived* from these numbers by
/// [`MaterialTable::with_transitions`], not declared alongside them — which
/// is what stops a transition and its reverse ever disagreeing about the
/// cost of the change.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transition {
    pub from: MaterialId,
    pub to: MaterialId,
    pub direction: Direction,
    pub threshold_k: Scalar,
    pub latent_heat: Scalar,
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
    transitions: Vec<Transition>,
}

impl MaterialTable {
    /// Builds a table from an explicit list of materials. Plain field
    /// construction, no decision to make, so implemented directly — same
    /// reasoning as `Material::new`. `materials[n]`'s position in the
    /// vector is that material's [`MaterialId`] under the id convention —
    /// see [`MaterialTable::get`].
    pub fn new(materials: Vec<Material>) -> Self {
        MaterialTable {
            materials,
            transitions: Vec::new(),
        }
    }

    /// Attaches this table's phase-transition rules and derives every
    /// material's [`Material::latent_energy`] offset from them.
    ///
    /// ## Why the offsets are derived rather than declared
    ///
    /// Total world energy is defined as `Σ mass * (c·T + latent_energy)`.
    /// For a transition A → B at temperature `Θ` absorbing `L` J/g, that
    /// definition only produces the right answer if
    ///
    /// ```text
    /// (c_B·Θ + L_B) - (c_A·Θ + L_A) == L
    /// ```
    ///
    /// i.e. `L_B == L_A + L + (c_A - c_B)·Θ`. Hand-written offsets satisfy
    /// that only by accident — the `(c_A - c_B)·Θ` term is easy to forget,
    /// and forgetting it silently makes every melt or boil create or
    /// destroy energy. So the offsets are solved for here instead: one
    /// material per connected component of the transition graph is pinned
    /// at zero (an arbitrary but harmless choice, since only differences
    /// are ever observed) and the rest are propagated out from it.
    ///
    /// Panics if two transitions imply contradictory offsets for the same
    /// material — that is a badly specified table, and it should be loud.
    pub fn with_transitions(mut self, transitions: Vec<Transition>) -> Self {
        let n = self.materials.len();
        let mut offset: Vec<Option<Scalar>> = vec![None; n];
        let capacity = |i: usize, materials: &[Material]| materials[i].heat_capacity;

        // Relaxation: propagate from whatever is already pinned, seeding a
        // fresh component at zero whenever a pass makes no progress.
        loop {
            let mut progressed = false;
            for tr in &transitions {
                let (a, b) = (tr.from.0 as usize, tr.to.0 as usize);
                let shift = tr.latent_heat
                    + (capacity(a, &self.materials) - capacity(b, &self.materials))
                        * tr.threshold_k;
                match (offset[a], offset[b]) {
                    (Some(la), None) => {
                        offset[b] = Some(la + shift);
                        progressed = true;
                    }
                    (None, Some(lb)) => {
                        offset[a] = Some(lb - shift);
                        progressed = true;
                    }
                    (Some(la), Some(lb)) => {
                        assert!(
                            (lb - (la + shift)).abs() <= 1e-2 * (1.0 + lb.abs()),
                            "transition {:?} -> {:?} implies a latent-energy offset of {} for \
                             material {}, but another transition already fixed it at {} — the \
                             table's transitions contradict each other",
                            tr.from,
                            tr.to,
                            la + shift,
                            b,
                            lb
                        );
                    }
                    (None, None) => {}
                }
            }
            if !progressed {
                // Seed the next unresolved component, if any remains.
                match transitions
                    .iter()
                    .find(|tr| offset[tr.from.0 as usize].is_none())
                {
                    Some(tr) => offset[tr.from.0 as usize] = Some(0.0),
                    None => break,
                }
            }
        }

        for (i, o) in offset.into_iter().enumerate() {
            if let Some(value) = o {
                self.materials[i].latent_energy = value;
            }
        }
        self.transitions = transitions;
        self
    }

    /// This table's phase-transition rules — see [`Transition`].
    pub fn transitions(&self) -> &[Transition] {
        &self.transitions
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

    /// The table the gnome terrarium actually runs on: eight materials
    /// spanning three phases, wired together by six phase transitions
    /// covering the full water cycle (ice ⇄ water ⇄ steam) and rock melting
    /// (stone ⇄ lava).
    ///
    /// Ids are the [`terrarium`] module's constants — `terrarium::WATER`
    /// and so on — never bare integers at call sites.
    ///
    /// **Units.** Unlike [`MaterialTable::reference`], this table *is*
    /// dimensionally consistent with itself, which the conservation checks
    /// require. Mass is in grams per cell-volume, temperature in kelvin,
    /// energy in joules; `heat_capacity` is J/(g·K) and `latent_energy` is
    /// J/g, both at their real-world values for the materials that have
    /// them. `density` is g per cell, so a full cell of water massing 1.0
    /// sets the scale. `conductivity` is a per-second exchange coefficient,
    /// not a real W/(m·K) figure — it is the one number here tuned for
    /// simulation feel rather than taken from a table.
    pub fn terrarium() -> Self {
        use terrarium as t;
        // Latent heats are real figures: 334 J/g to melt ice, 2260 J/g to
        // boil water. Per-material enthalpy offsets are derived from them
        // by `with_transitions`, not written here.
        const L_FUSION: Scalar = 334.0;
        const L_VAPORISATION: Scalar = 2260.0;
        const L_ROCK_MELT: Scalar = 400.0;
        const T_FREEZE: Scalar = 273.15;
        const T_BOIL: Scalar = 373.15;
        const T_ROCK_MELT: Scalar = 1500.0;

        let mut materials = vec![Material::new(0.0, 0.0, 0.0, 0.0, Phase::Gas, (0, 0, 0)); 8];
        materials[t::AIR.0 as usize] =
            Material::new(0.0012, 0.02, 1.005, 0.05, Phase::Gas, (16, 18, 28)).breathable();
        materials[t::STEAM.0 as usize] =
            Material::new(0.0006, 0.01, 2.08, 0.04, Phase::Gas, (170, 180, 200)).breathable();
        materials[t::WATER.0 as usize] =
            Material::new(1.0, 0.5, 4.186, 0.6, Phase::Liquid, (40, 90, 200));
        materials[t::ICE.0 as usize] =
            Material::new(0.92, 0.0, 2.093, 2.2, Phase::Solid, (170, 210, 240));
        materials[t::SAND.0 as usize] =
            Material::new(1.6, 0.0, 0.83, 0.3, Phase::Solid, (200, 175, 105))
                .with_mobility(Mobility::Granular);
        materials[t::STONE.0 as usize] =
            Material::new(2.5, 0.0, 0.8, 2.0, Phase::Solid, (105, 105, 115));
        materials[t::LAVA.0 as usize] =
            Material::new(2.4, 8.0, 1.0, 1.5, Phase::Liquid, (235, 110, 40));
        materials[t::JUNIPER.0 as usize] =
            Material::new(0.5, 0.0, 2.0, 0.2, Phase::Solid, (70, 130, 90));

        let heating = |from, to, threshold_k, latent_heat| Transition {
            from,
            to,
            direction: Direction::Heating,
            threshold_k,
            latent_heat,
        };
        let cooling = |from, to, threshold_k, latent_heat: Scalar| Transition {
            from,
            to,
            direction: Direction::Cooling,
            threshold_k,
            latent_heat: -latent_heat,
        };
        let transitions = vec![
            heating(t::ICE, t::WATER, T_FREEZE, L_FUSION),
            cooling(t::WATER, t::ICE, T_FREEZE, L_FUSION),
            heating(t::WATER, t::STEAM, T_BOIL, L_VAPORISATION),
            cooling(t::STEAM, t::WATER, T_BOIL, L_VAPORISATION),
            heating(t::STONE, t::LAVA, T_ROCK_MELT, L_ROCK_MELT),
            cooling(t::LAVA, t::STONE, T_ROCK_MELT, L_ROCK_MELT),
        ];

        MaterialTable::new(materials).with_transitions(transitions)
    }
}

/// Named [`MaterialId`]s for [`MaterialTable::terrarium`]. Call sites use
/// `terrarium::WATER`, never `MaterialId::new(2)`.
pub mod terrarium {
    use super::MaterialId;

    pub const AIR: MaterialId = MaterialId(0);
    pub const STEAM: MaterialId = MaterialId(1);
    pub const WATER: MaterialId = MaterialId(2);
    pub const ICE: MaterialId = MaterialId(3);
    pub const SAND: MaterialId = MaterialId(4);
    pub const STONE: MaterialId = MaterialId(5);
    pub const LAVA: MaterialId = MaterialId(6);
    pub const JUNIPER: MaterialId = MaterialId(7);

    /// Every id above, in table order — for tests and reporting that want
    /// to iterate the whole table by name.
    pub const ALL: [MaterialId; 8] = [AIR, STEAM, WATER, ICE, SAND, STONE, LAVA, JUNIPER];

    /// Human-readable name for a terrarium id, for JSON reports.
    pub fn name(id: MaterialId) -> &'static str {
        match id.0 {
            0 => "air",
            1 => "steam",
            2 => "water",
            3 => "ice",
            4 => "sand",
            5 => "stone",
            6 => "lava",
            7 => "juniper",
            _ => "unknown",
        }
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
