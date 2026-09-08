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
    /// A compressible gas. Unlike every other phase, a `Gas` cell's
    /// [`crate::world::Cell::mass`] is genuinely variable: it is the amount
    /// of gas packed into that cell, and with [`Material::gas_constant`] it
    /// gives the cell a real pressure (`P = m·R·T`). `src/gas.rs` moves gas
    /// down pressure gradients; `src/physics.rs` sinks and floats it by its
    /// actual mass, so a compressed pocket is heavier than a thin one.
    ///
    /// (`src/grid.rs`'s older material-only physics still treats gas as
    /// immobile background — that grid has nowhere to put a mass.)
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
    /// Specific gas constant `R` in J/(g·K), or `0.0` for anything that
    /// isn't a gas.
    ///
    /// This is the one number that turns [`Phase::Gas`] from "a light thing
    /// that gets displaced" into a substance with a **pressure**: a gas cell
    /// holding `m` grams at `T` kelvin is at `P = m·R·T` (per cell volume,
    /// which is 1 in this simulation's units). `src/gas.rs` moves mass down
    /// the gradient of that quantity, which is what makes a gas fill the
    /// space it is given instead of sitting where it was painted.
    ///
    /// Real values: air 0.287, steam 0.4615, CO₂ 0.1889 — a lighter
    /// molecule has a bigger `R`, which is why, at equal pressure and
    /// temperature, CO₂ ends up the densest gas in the jar and settles
    /// under the others without any rule saying so.
    pub gas_constant: Scalar,
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
                Phase::Granular => Mobility::Granular,
                Phase::Liquid | Phase::Gas => Mobility::Flowing,
            },
            breathable: false,
            gas_constant: 0.0,
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

    /// Sets [`Material::gas_constant`], builder-style.
    pub fn with_gas_constant(mut self, gas_constant: Scalar) -> Self {
        self.gas_constant = gas_constant;
        self
    }

    /// This material's pressure when `mass` grams of it sit in one cell at
    /// `temperature` — the ideal gas law, `P = m·R·T`. Zero for anything
    /// with no [`Material::gas_constant`], which is every non-gas.
    pub fn pressure(&self, mass: Scalar, temperature: Scalar) -> Scalar {
        self.gas_constant * mass * temperature
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

/// One participant's half of a [`Reaction`]: the cell holding `from`
/// becomes `to`, keeping its own mass.
///
/// `heat` is the energy this half releases, in joules per gram of that
/// cell's mass, measured at the reaction's threshold temperature. It is
/// **optional**, and the distinction matters:
///
/// - `Some(q)` is a *declaration*: it pins `to`'s [`Material::latent_energy`]
///   relative to `from`'s, exactly the way a [`Transition`]'s `latent_heat`
///   does. Use it for the arm whose chemistry you are actually specifying —
///   the fuel that burns, the botanical that ferments.
/// - `None` means "whatever the rest of the table already implies". Use it
///   when both materials' offsets are already fixed by other rules;
///   declaring a number there would be redundant at best and, since the
///   implied figure depends on both heat capacities and the threshold, wrong
///   at worst.
///
/// Either way the *runtime* is unaffected: `src/chemistry.rs` conserves
/// energy by solving the reacting pair's shared temperature from its total
/// energy, so a reaction's heat is emergent, not applied. `heat` only
/// decides what the enthalpy offsets are, and therefore how much heat that
/// emergent solve produces.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Arm {
    pub from: MaterialId,
    pub to: MaterialId,
    pub heat: Option<Scalar>,
}

impl Arm {
    /// An arm whose enthalpy change is declared: `heat` J/g released.
    pub fn releasing(from: MaterialId, to: MaterialId, heat: Scalar) -> Self {
        Arm {
            from,
            to,
            heat: Some(heat),
        }
    }

    /// An arm whose enthalpy change is already implied by the rest of the
    /// table — see [`Arm::heat`].
    pub fn implied(from: MaterialId, to: MaterialId) -> Self {
        Arm {
            from,
            to,
            heat: None,
        }
    }
}

/// A data-driven chemical reaction between two touching cells: "when a cell
/// of `subject.from` is next to a cell of `partner.from` and the pair is
/// past `threshold_k`, they become `subject.to` and `partner.to`".
///
/// This is the chemistry tier of `NORTH_STARS.md` #2's stated ordering
/// (physics → chemistry → biology → game layer), and it is deliberately the
/// same *shape* as [`Transition`]: data in this file, behaviour in one
/// generic pass (`src/chemistry.rs`), no material named anywhere in code.
///
/// ## What it does and does not model
///
/// Each cell keeps its own mass across the reaction, so mass is conserved
/// per cell — stronger than the global invariant the rest of the crate
/// promises. Energy is conserved per *pair*: the two cells' total energy is
/// computed before, and the shared temperature after is solved from it.
///
/// What it therefore does **not** model is stoichiometry. A cell holds one
/// material and one mass, so there is no way to say "two grams of this and
/// one of that make three of the other"; a reaction says which materials a
/// touching pair turns into, not in what proportion. Where a real
/// proportion matters — a liquid boiling off into a vapour that occupies a
/// thousand times the space — that belongs to the phase-change machinery,
/// which does move mass between materials honestly, and the terrarium's
/// distilling uses it for exactly that reason.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reaction {
    pub subject: Arm,
    pub partner: Arm,
    /// Temperature the reacting pair must be past, in kelvin.
    pub threshold_k: Scalar,
    /// Whether the reaction fires above the threshold (`Heating`) or below
    /// it (`Cooling`) — the same convention [`Transition`] uses.
    pub direction: Direction,
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
    reactions: Vec<Reaction>,
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
            reactions: Vec::new(),
        }
    }

    /// Attaches this table's phase-transition rules and derives every
    /// material's [`Material::latent_energy`] offset from them.
    ///
    /// Shorthand for [`MaterialTable::with_chemistry`] with no reactions.
    pub fn with_transitions(self, transitions: Vec<Transition>) -> Self {
        self.with_chemistry(transitions, Vec::new())
    }

    /// Attaches this table's phase transitions *and* its reactions, and
    /// solves every material's [`Material::latent_energy`] offset from both
    /// at once.
    ///
    /// ## Why the offsets are derived rather than declared
    ///
    /// Total world energy is defined as `Σ mass * (c·T + latent_energy)`.
    /// For a material change A → B at temperature `Θ` absorbing `L` J/g,
    /// that definition only produces the right answer if
    ///
    /// ```text
    /// (c_B·Θ + L_B) - (c_A·Θ + L_A) == L
    /// ```
    ///
    /// i.e. `L_B == L_A + L + (c_A - c_B)·Θ`. Hand-written offsets satisfy
    /// that only by accident — the `(c_A - c_B)·Θ` term is easy to forget,
    /// and forgetting it silently makes every melt, boil or burn create or
    /// destroy energy. So the offsets are solved for here instead: one
    /// material per connected component of the constraint graph is pinned
    /// at zero (an arbitrary but harmless choice, since only differences
    /// are ever observed) and the rest are propagated out from it.
    ///
    /// ## Why transitions and reactions share one solve
    ///
    /// The equation above does not care whether the material change was a
    /// phase transition or half of a reaction — both are "A becomes B at Θ,
    /// with this much enthalpy change". Solving them separately would let
    /// the two disagree about a material they share (water is both the
    /// thing that freezes and the thing that ferments), and the disagreement
    /// would show up as energy quietly appearing every time a brewer's tun
    /// warmed up. Fed to one relaxation, a contradiction is instead a panic
    /// at table-construction time.
    ///
    /// A [`Reaction`] arm with no declared `heat` contributes no equation —
    /// see [`Arm::heat`] — and an arm whose `from` and `to` are the same
    /// material contributes none either, since it changes nothing.
    ///
    /// Panics if two rules imply contradictory offsets for the same
    /// material — that is a badly specified table, and it should be loud.
    pub fn with_chemistry(
        mut self,
        transitions: Vec<Transition>,
        reactions: Vec<Reaction>,
    ) -> Self {
        let n = self.materials.len();
        let capacity = |i: usize, materials: &[Material]| materials[i].heat_capacity;

        // One constraint per declared material change: `offset[to] ==
        // offset[from] + shift`. Both rule kinds reduce to this.
        struct Constraint {
            from: usize,
            to: usize,
            shift: Scalar,
            source: &'static str,
        }
        let mut constraints: Vec<Constraint> = Vec::new();
        let mut push = |from: MaterialId,
                        to: MaterialId,
                        absorbed: Scalar,
                        threshold_k: Scalar,
                        source: &'static str,
                        materials: &[Material]| {
            let (a, b) = (from.0 as usize, to.0 as usize);
            if a == b {
                return;
            }
            constraints.push(Constraint {
                from: a,
                to: b,
                shift: absorbed + (capacity(a, materials) - capacity(b, materials)) * threshold_k,
                source,
            });
        };
        for tr in &transitions {
            push(
                tr.from,
                tr.to,
                tr.latent_heat,
                tr.threshold_k,
                "transition",
                &self.materials,
            );
        }
        for r in &reactions {
            for arm in [r.subject, r.partner] {
                // `heat` is energy released, the opposite sign to a
                // transition's absorbed `latent_heat`.
                if let Some(heat) = arm.heat {
                    push(
                        arm.from,
                        arm.to,
                        -heat,
                        r.threshold_k,
                        "reaction",
                        &self.materials,
                    );
                }
            }
        }

        let mut offset: Vec<Option<Scalar>> = vec![None; n];
        // Relaxation: propagate from whatever is already pinned, seeding a
        // fresh component at zero whenever a pass makes no progress.
        loop {
            let mut progressed = false;
            for c in &constraints {
                match (offset[c.from], offset[c.to]) {
                    (Some(la), None) => {
                        offset[c.to] = Some(la + c.shift);
                        progressed = true;
                    }
                    (None, Some(lb)) => {
                        offset[c.from] = Some(lb - c.shift);
                        progressed = true;
                    }
                    (Some(la), Some(lb)) => {
                        assert!(
                            (lb - (la + c.shift)).abs() <= 1e-2 * (1.0 + lb.abs()),
                            "{} {} -> {} implies a latent-energy offset of {} for material {}, \
                             but another rule already fixed it at {} — the table's chemistry \
                             contradicts itself",
                            c.source,
                            c.from,
                            c.to,
                            la + c.shift,
                            c.to,
                            lb
                        );
                    }
                    (None, None) => {}
                }
            }
            if !progressed {
                // Seed the next unresolved component, if any remains.
                match constraints.iter().find(|c| offset[c.from].is_none()) {
                    Some(c) => offset[c.from] = Some(0.0),
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
        self.reactions = reactions;
        self
    }

    /// This table's reaction rules — see [`Reaction`].
    pub fn reactions(&self) -> &[Reaction] {
        &self.reactions
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
        // Ethanol's real latent heat of vaporisation and real boiling
        // point. The boiling point is the load-bearing number in the whole
        // brewing chain: it sits 22 K below water's, which is the only
        // reason heating a tun separates anything at all.
        const L_SPIRIT: Scalar = 846.0;
        const T_FREEZE: Scalar = 273.15;
        const T_BOIL: Scalar = 373.15;
        const T_SPIRIT_BOIL: Scalar = 351.5;
        const T_ROCK_MELT: Scalar = 1500.0;
        /// Warm enough for a tun to work: below this, juniper sitting in
        /// water is just a wet bush.
        const T_MASH: Scalar = 310.0;
        /// Ignition point for a botanical.
        const T_IGNITE: Scalar = 620.0;
        /// Heat released burning a gram of juniper.
        ///
        /// A real dry botanical is nearer 16 000 J/g, and that figure is
        /// wrong *here* for a stated reason: a burning cell has nowhere to
        /// put its combustion gases, because one cell holds one material at
        /// one volume. Real flame is ~1200 K because it expands and mixes;
        /// an adiabatic cell that cannot do either would reach 16 000 K and
        /// melt the jar it is in. So this is a tuned number, in the same
        /// category as `conductivity` below and flagged the same way, chosen
        /// to put a flame front around 1300 K.
        const Q_BURN: Scalar = 700.0;

        // The gases' densities are *derived*, not chosen: each is the mass
        // of that gas which sits at one atmosphere at room temperature,
        // `ρ = P₀/(R·T₀)`. Picking them by hand instead would mean a world
        // painted with steam in it starts out with a pressure step across
        // every steam/air boundary, and `gas::diffuse` would immediately go
        // to work levelling a discontinuity that was only ever a typo.
        const T0: Scalar = 291.0;
        const R_AIR: Scalar = 0.287;
        const R_STEAM: Scalar = 0.4615;
        const R_CO2: Scalar = 0.1889;
        // Ethanol vapour: a heavier molecule than air (46 g/mol against
        // 29), so a smaller specific gas constant, so — at the same
        // pressure and temperature — a denser gas. That single number is
        // why spirit vapour crawls sideways out of a still along the top of
        // the wash instead of rising to the ceiling, and it is not a rule
        // anyone wrote: it is 8.314/46.07.
        const R_SPIRIT: Scalar = 0.1805;
        const RHO_AIR: Scalar = 0.0012;
        const P0: Scalar = RHO_AIR * R_AIR * T0;

        let mut materials = vec![Material::new(0.0, 0.0, 0.0, 0.0, Phase::Gas, (0, 0, 0)); 13];
        materials[t::AIR.0 as usize] =
            Material::new(RHO_AIR, 0.02, 1.005, 0.05, Phase::Gas, (16, 18, 28))
                .breathable()
                .with_gas_constant(R_AIR);
        materials[t::STEAM.0 as usize] = Material::new(
            P0 / (R_STEAM * T0),
            0.01,
            2.08,
            0.04,
            Phase::Gas,
            (170, 180, 200),
        )
        .breathable()
        .with_gas_constant(R_STEAM);
        // Carbon dioxide: heavier than air at the same pressure, and not
        // breathable. `NORTH_STARS.md` #4 names ONI's gas handling as one of
        // the things this project exists to fix — "CO2 doesn't actually
        // settle the way ONI's simplified layers show it". Here it settles
        // because it is genuinely heavier, by the same one rule that sinks
        // sand through air.
        materials[t::CO2.0 as usize] = Material::new(
            P0 / (R_CO2 * T0),
            0.03,
            0.844,
            0.04,
            Phase::Gas,
            (92, 74, 108),
        )
        .with_gas_constant(R_CO2);
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
        // The brewing chain. `wash` is what a warm tun makes of juniper and
        // water; it boils into `spirit` 22 K below water's boiling point,
        // and `spirit` condenses back into `gin`. Nothing in that chain is
        // special-cased anywhere: mashing is a row in the reaction table,
        // and both halves of distilling are ordinary phase transitions.
        // Wash's conductivity is high on purpose, and it is the one number
        // in the brewing chain that is tuned rather than physical (the same
        // licence this table's note above already takes with `conductivity`
        // generally). Ethanol's 846 J/g really does take a quarter of an
        // hour of simulated time to move through a water-like conductor,
        // which is correct and unwatchable. A pot that is stirred, and made
        // of metal rather than water, is the story; 6.0 puts the first drop
        // off the still inside half a minute of simulated time.
        materials[t::WASH.0 as usize] =
            Material::new(1.02, 0.6, 3.9, 6.0, Phase::Liquid, (150, 112, 62));
        materials[t::SPIRIT.0 as usize] = Material::new(
            P0 / (R_SPIRIT * T0),
            0.01,
            1.42,
            0.03,
            Phase::Gas,
            (196, 226, 200),
        )
        .with_gas_constant(R_SPIRIT);
        materials[t::GIN.0 as usize] =
            Material::new(0.94, 0.35, 2.44, 0.6, Phase::Liquid, (196, 224, 236));
        materials[t::CHARCOAL.0 as usize] =
            Material::new(0.45, 0.0, 0.84, 0.25, Phase::Solid, (38, 34, 32))
                .with_mobility(Mobility::Granular);

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
            // Distilling, as two ordinary phase changes. Wash boils into
            // spirit; spirit condenses into gin, not back into wash — which
            // is what makes the still a one-way concentrator rather than a
            // loop, and is the only asymmetry in the whole chain.
            heating(t::WASH, t::SPIRIT, T_SPIRIT_BOIL, L_SPIRIT),
            heating(t::GIN, t::SPIRIT, T_SPIRIT_BOIL, L_SPIRIT),
            cooling(t::SPIRIT, t::GIN, T_SPIRIT_BOIL, L_SPIRIT),
        ];

        // Chemistry. Two rows, and between them they cover both things a
        // botanical can be turned into.
        let reactions = vec![
            // Mashing: juniper steeping in warm water makes wash of both.
            // Declared enthalpy-neutral on the water arm, which is what
            // ties the water component's arbitrary zero to the brewing
            // component's — without one declared number joining them, the
            // heat of mashing would be whatever the solver's seeding
            // happened to make it.
            Reaction {
                subject: Arm::releasing(t::WATER, t::WASH, 0.0),
                partner: Arm::releasing(t::JUNIPER, t::WASH, 0.0),
                threshold_k: T_MASH,
                direction: Direction::Heating,
            },
            // Burning: a botanical hot enough to ignite, with air to burn
            // in, leaves charcoal and a cell of CO2 behind.
            Reaction {
                subject: Arm::releasing(t::JUNIPER, t::CHARCOAL, Q_BURN),
                partner: Arm::releasing(t::AIR, t::CO2, 0.0),
                threshold_k: T_IGNITE,
                direction: Direction::Heating,
            },
        ];

        MaterialTable::new(materials).with_chemistry(transitions, reactions)
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
    pub const CO2: MaterialId = MaterialId(8);
    /// Fermented juniper mash: what water and juniper become in a warm
    /// tun, and the only thing a still can usefully be charged with.
    pub const WASH: MaterialId = MaterialId(9);
    /// Alcohol vapour — what wash boils into, at a lower temperature than
    /// water boils at, which is the whole of why distilling separates
    /// anything.
    pub const SPIRIT: MaterialId = MaterialId(10);
    /// The drink itself: condensed spirit, and the colony's mana.
    pub const GIN: MaterialId = MaterialId(11);
    /// What is left of a botanical that burned instead of being brewed.
    pub const CHARCOAL: MaterialId = MaterialId(12);

    /// Every id above, in table order — for tests and reporting that want
    /// to iterate the whole table by name.
    pub const ALL: [MaterialId; 13] = [
        AIR, STEAM, WATER, ICE, SAND, STONE, LAVA, JUNIPER, CO2, WASH, SPIRIT, GIN, CHARCOAL,
    ];

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
            8 => "co2",
            9 => "wash",
            10 => "spirit",
            11 => "gin",
            12 => "charcoal",
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
