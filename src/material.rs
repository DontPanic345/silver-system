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
    /// Whether this species is what a gnome actually breathes — the
    /// oxidiser, not "air". Only oxygen carries it.
    ///
    /// This used to be set on air and on steam, and it was the wrong shape
    /// as soon as gas cells became mixtures: a cell labelled "air" can be
    /// three quarters CO₂ and a cell labelled "CO₂" can still hold plenty of
    /// oxygen, so a flag read off the *label* answers a question about the
    /// wrong thing. The rule that reads it now sums the partial pressure of
    /// every breathable species in the mixture — see
    /// [`crate::world::Cell::breathable_pressure`] — which is what makes
    /// suffocation a property of the air a gnome is standing in rather than
    /// of what that air is mostly made of.
    pub breathable: bool,
    /// Fraction of the light falling on this cell that it absorbs, per
    /// cell of depth. Zero for a gas (a room of air is not a shade), high
    /// for rock and for leaves.
    ///
    /// Read by `src/light.rs`, which marches sunlight down each column and
    /// attenuates it by this. It is the one number that makes shade — and
    /// therefore where a plant will grow — a property of what is standing
    /// in the way rather than of a rule naming any particular material.
    pub opacity: Scalar,
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
    /// Volumetric thermal expansion, per kelvin, for a liquid; `0.0` for
    /// anything else (a gas's expansion is already in its gas constant).
    ///
    /// Read by buoyancy only — the Boussinesq approximation. A cell of hot
    /// water still holds a gram; it is only *weighed* as slightly lighter
    /// than a cold one when gravity decides which of the two sinks. That is
    /// the whole of what makes a pot heated from below turn over instead of
    /// sitting as a hot bottom under a cool top, and a still needs its pot
    /// to turn over: with the surface left cool, the vapour over it
    /// condenses straight back onto it. See [`Material::buoyant_density`].
    pub thermal_expansion: Scalar,
}

/// The temperature the material table's densities are quoted at — room
/// temperature, and the one [`Material::thermal_expansion`] is measured
/// from.
pub const DENSITY_REFERENCE_K: Scalar = 291.0;

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
            opacity: match phase {
                // A gas is transparent; everything condensed is not. These
                // are defaults, overridable per material — water is clearer
                // than rock and says so below.
                Phase::Gas => 0.0,
                _ => 1.0,
            },
            gas_constant: 0.0,
            thermal_expansion: 0.0,
        }
    }

    /// Sets [`Material::opacity`], builder-style.
    pub fn with_opacity(mut self, opacity: Scalar) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Sets [`Material::thermal_expansion`], builder-style.
    pub fn with_thermal_expansion(mut self, per_kelvin: Scalar) -> Self {
        self.thermal_expansion = per_kelvin;
        self
    }

    /// `mass` of this material at `temperature`, as buoyancy weighs it:
    /// scaled down by its thermal expansion above the reference
    /// temperature (and up below it). Identical to `mass` for anything that
    /// does not expand.
    pub fn buoyant_density(&self, mass: Scalar, temperature: Scalar) -> Scalar {
        mass * (1.0 - self.thermal_expansion * (temperature - DENSITY_REFERENCE_K))
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

/// One side of a [`Metabolism`]'s books: so many grams of a material, per
/// unit of turnover.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reagent {
    pub material: MaterialId,
    pub grams: Scalar,
}

/// Shorthand for a [`Reagent`].
pub const fn reagent(material: MaterialId, grams: Scalar) -> Reagent {
    Reagent { material, grams }
}

/// A living process: a cell of `host` draws `intake` out of itself and its
/// neighbours and puts `output` back, in **declared mass proportions**.
///
/// This is the biology tier of `NORTH_STARS.md` #2's ordering (physics →
/// chemistry → biology → game layer), and it exists because the tier below
/// it cannot express what life does. A [`Reaction`] is a *relabelling*: each
/// of two touching cells keeps its own mass and only changes what it is
/// called, which is why `src/chemistry.rs`'s doc comment says flatly that it
/// has no stoichiometry. But `6 CO₂ + 6 H₂O → C₆H₁₂O₆ + 6 O₂` is nothing
/// *but* stoichiometry — a gram of plant is made of 1.47 g of carbon dioxide
/// and 0.6 g of water, and gives 1.07 g of oxygen back, and if those ratios
/// are not obeyed the carbon in a jar is not conserved even when the total
/// mass is.
///
/// So a metabolism moves real, unequal masses between cells. Conservation is
/// still by construction, in two parts:
///
/// - **Mass.** `Σ intake == Σ output` is asserted when the table is built,
///   and the runtime moves exactly those grams: what leaves the donors
///   arrives in the host, and what leaves the host arrives in the acceptors.
/// - **Energy.** Every parcel of mass carries its own enthalpy with it, and
///   the *host* settles the difference — so the heat of the process shows up
///   as the host cell's temperature changing, and cannot be anything else.
///   See `src/life.rs`.
///
/// `heat` is the energy released per unit of turnover, J, at `heat_at_k`,
/// and it is a declaration in exactly the sense [`Arm::heat`] is: it pins
/// the participating materials' [`Material::latent_energy`] offsets, through
/// the same single relaxation that already solves them from transitions and
/// reactions. Declaring it on one direction of a process therefore fixes the
/// other direction for free — photosynthesis and respiration cannot disagree
/// about the energy in a gram of plant, because only one of them is allowed
/// to say.
#[derive(Debug, Clone, PartialEq)]
pub struct Metabolism {
    /// For reports and test failures; never matched on.
    pub name: &'static str,
    /// The living cell that runs the process. Reagents are drawn from it
    /// and its four neighbours, and products are put back the same way.
    pub host: MaterialId,
    /// Grams consumed per unit of turnover.
    pub intake: Vec<Reagent>,
    /// Grams produced per unit of turnover.
    pub output: Vec<Reagent>,
    /// Units of turnover per second, at full drive.
    pub rate: Scalar,
    /// The band of incident light this process runs in, as a fraction of
    /// full sun — see `src/light.rs`. Photosynthesis wants the bright half,
    /// a plant's own respiration the dark half, and a process that does not
    /// care takes the whole range.
    pub light_min: Scalar,
    pub light_max: Scalar,
    /// The temperature band it runs in. Outside it, nothing happens: a
    /// frozen bush does not grow and a boiled one does not either.
    pub min_k: Scalar,
    pub max_k: Scalar,
    /// Energy released per unit of turnover, J, or `None` for "whatever the
    /// rest of the table already implies" — see [`Arm::heat`], which this
    /// follows exactly.
    pub heat: Option<Scalar>,
    /// The temperature `heat` is quoted at.
    pub heat_at_k: Scalar,
}

impl Metabolism {
    /// Whether this process runs at all at an incident `light`.
    pub fn lit(&self, light: Scalar) -> bool {
        light >= self.light_min && light <= self.light_max
    }

    /// How hard it runs there, as a fraction of [`Metabolism::rate`].
    ///
    /// A process that declares the *whole* range does not care about light
    /// and runs flat: a decomposer works as well at midnight as at noon, and
    /// before this said so a mould in the dark crawled at a twentieth of its
    /// rate for no stated reason. Anything narrower is driven by how far into
    /// its band the light is, so a plant at the bottom of its band creeps and
    /// one in full sun does not. The floor is there because a band's own edge
    /// is not a stop — a process in its conditions is running.
    pub fn light_drive(&self, light: Scalar) -> Scalar {
        if (self.light_min <= 0.0 && self.light_max >= 1.0) || self.light_max <= self.light_min {
            return 1.0;
        }
        ((light - self.light_min) / (self.light_max - self.light_min))
            .clamp(0.0, 1.0)
            .max(0.05)
    }

    /// Total grams on each side, which the table asserts are equal.
    pub fn intake_grams(&self) -> Scalar {
        self.intake.iter().map(|r| r.grams).sum()
    }

    pub fn output_grams(&self) -> Scalar {
        self.output.iter().map(|r| r.grams).sum()
    }

    /// Net grams of the host material this process gains per unit of
    /// turnover: positive for growth, negative for a plant burning itself.
    pub fn host_gain(&self) -> Scalar {
        let out: Scalar = self
            .output
            .iter()
            .filter(|r| r.material == self.host)
            .map(|r| r.grams)
            .sum();
        let inn: Scalar = self
            .intake
            .iter()
            .filter(|r| r.material == self.host)
            .map(|r| r.grams)
            .sum();
        out - inn
    }
}

/// The most species a single gas cell can hold at once — every gas in the
/// table plus every liquid a gas condenses into (carried as suspended mist).
/// A fixed-size array rather than a map because it is copied with every
/// cell, and the table asserts at construction that it fits.
pub const MIX_SLOTS: usize = 8;

/// A liquid and the vapour it evaporates into, with the two numbers that
/// fix its vapour-pressure curve: the boiling point at the table's
/// [reference pressure](MaterialTable::reference_pressure), and the latent
/// heat of vaporisation.
///
/// Nothing here is declared separately. Every `Volatile` is *read off* a
/// [`Transition`] the table already carries — a liquid's heating transition
/// into a gas is an evaporation, a gas's cooling transition into a liquid is
/// a condensation — so a boiling point written once in the transition table
/// is the same boiling point the humidity of the air is computed from, and
/// the two cannot drift apart. See [`MaterialTable::saturation_pressure`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Volatile {
    pub liquid: MaterialId,
    pub vapour: MaterialId,
    pub boiling_k: Scalar,
    /// Real latent heat of vaporisation, J/g (always positive).
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
    reactions: Vec<Reaction>,
    /// The pressure at which this table's boiling points hold — see
    /// [`MaterialTable::reference_pressure`].
    reference_pressure: Scalar,
    /// Which species each mixture slot holds, in slot order: every gas,
    /// then every liquid some gas condenses into.
    slots: Vec<MaterialId>,
    /// The inverse of `slots`, indexed by material id.
    slot_of: Vec<Option<u8>>,
    /// Liquids that evaporate, read off heating transitions into a gas.
    evaporating: Vec<Volatile>,
    /// Vapours that condense, read off cooling transitions into a liquid.
    condensing: Vec<Volatile>,
    /// What a cell of open air in this world is made of — see
    /// [`MaterialTable::atmosphere`].
    atmosphere: Vec<(MaterialId, Scalar)>,
    /// Living processes — see [`Metabolism`].
    metabolisms: Vec<Metabolism>,
    /// Per-slot copies of the properties every mixture sum reads — see
    /// [`SlotProps`].
    slot_props: SlotProps,
}

/// The properties of each mixture slot's species, laid out as arrays so
/// the sums a gas cell is made of (`Σ m·c`, `Σ m·L`, `Σ m·R`) are tight
/// loops rather than a table lookup per term. A cache of the materials,
/// rebuilt whenever they change; never a second source of truth.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SlotProps {
    pub heat_capacity: [Scalar; MIX_SLOTS],
    pub latent_energy: [Scalar; MIX_SLOTS],
    pub gas_constant: [Scalar; MIX_SLOTS],
    pub is_gas: [bool; MIX_SLOTS],
    /// Whether this slot's species is one a gnome can breathe — see
    /// [`Material::breathable`].
    pub breathable: [bool; MIX_SLOTS],
}

impl MaterialTable {
    /// Builds a table from an explicit list of materials. Plain field
    /// construction, no decision to make, so implemented directly — same
    /// reasoning as `Material::new`. `materials[n]`'s position in the
    /// vector is that material's [`MaterialId`] under the id convention —
    /// see [`MaterialTable::get`].
    pub fn new(materials: Vec<Material>) -> Self {
        let mut table = MaterialTable {
            materials,
            transitions: Vec::new(),
            reactions: Vec::new(),
            reference_pressure: 0.0,
            slots: Vec::new(),
            slot_of: Vec::new(),
            evaporating: Vec::new(),
            condensing: Vec::new(),
            atmosphere: Vec::new(),
            metabolisms: Vec::new(),
            slot_props: SlotProps::default(),
        };
        table.index_mixtures();
        table
    }

    /// Declares what a cell of open air in this world holds, in grams per
    /// species — see [`MaterialTable::atmosphere`].
    pub fn with_atmosphere(mut self, mix: Vec<(MaterialId, Scalar)>) -> Self {
        self.atmosphere = mix;
        self
    }

    /// What a cell of open air in this world is made of, in grams per
    /// species.
    ///
    /// Before night 5 there was no such thing: "air" was a single material
    /// and a cell of it was pure. That was fine while nothing consumed any
    /// part of the air, and wrong the moment something did — a gnome
    /// breathes *oxygen*, not air, and a plant gives oxygen back, so the two
    /// have to be different entries in the same mixture or neither can
    /// happen. The composition is the real one, by mole fraction, and the
    /// masses are derived from it rather than typed in, so a cell of open
    /// air sits at exactly [`MaterialTable::reference_pressure`] and a world
    /// painted with it starts with no pressure step anywhere in it.
    ///
    /// Empty for a table that never declared one, in which case a caller
    /// asking for open air gets a single full cell of the first gas.
    pub fn atmosphere(&self) -> &[(MaterialId, Scalar)] {
        &self.atmosphere
    }

    /// This table's living processes — see [`Metabolism`].
    pub fn metabolisms(&self) -> &[Metabolism] {
        &self.metabolisms
    }

    /// Per-slot species properties — see [`SlotProps`].
    pub fn slot_props(&self) -> &SlotProps {
        &self.slot_props
    }

    /// Sets the pressure this table's boiling points are quoted at,
    /// builder-style.
    pub fn with_reference_pressure(mut self, pressure: Scalar) -> Self {
        self.reference_pressure = pressure;
        self
    }

    /// The pressure at which every boiling point in this table holds: one
    /// atmosphere, in this simulation's units.
    ///
    /// It is also the pressure every gas's nominal density is quoted at,
    /// which is why buoyancy between two gas cells can be compared at it —
    /// see `physics::pick_target`.
    pub fn reference_pressure(&self) -> Scalar {
        self.reference_pressure
    }

    /// Whether `id` is a gas species — a material that lives in a gas
    /// cell's mixture rather than filling a cell on its own.
    pub fn is_gas(&self, id: MaterialId) -> bool {
        let m = self.get(id);
        m.phase == Phase::Gas && m.gas_constant > 0.0
    }

    /// The mixture slot `id` occupies in a gas cell, if it can be part of a
    /// mixture at all.
    pub fn slot(&self, id: MaterialId) -> Option<usize> {
        self.slot_of
            .get(id.0 as usize)
            .copied()
            .flatten()
            .map(|s| s as usize)
    }

    /// Species in slot order — see [`MaterialTable::slot`].
    pub fn slots(&self) -> &[MaterialId] {
        &self.slots
    }

    /// The label a gas cell with nothing in it carries: the first gas in
    /// the table. Only a label — a cell holding no mass has no energy and
    /// no pressure, and the first gas to flow into it relabels it.
    pub fn vacuum_label(&self) -> MaterialId {
        self.slots
            .iter()
            .copied()
            .find(|&id| self.is_gas(id))
            .expect("a table with gas cells in it needs at least one gas species")
    }

    /// What `liquid` evaporates into, if anything.
    pub fn evaporation_of(&self, liquid: MaterialId) -> Option<&Volatile> {
        self.evaporating.iter().find(|v| v.liquid == liquid)
    }

    /// Whether any liquid in the table evaporates into `vapour`.
    pub fn evaporation_of_any_into(&self, vapour: MaterialId) -> bool {
        self.evaporating.iter().any(|v| v.vapour == vapour)
    }

    /// What `vapour` condenses into, if anything.
    pub fn condensation_of(&self, vapour: MaterialId) -> Option<&Volatile> {
        self.condensing.iter().find(|v| v.vapour == vapour)
    }

    /// The partial pressure of `volatile`'s vapour that is in equilibrium
    /// with its liquid at `temperature` — the most of it the air can hold.
    ///
    /// The Clausius–Clapeyron relation, integrated with a constant latent
    /// heat:
    ///
    /// ```text
    /// p_sat(T) = P_ref · exp( (L / R_v) · (1/T_b − 1/T) )
    /// ```
    ///
    /// Every number in it was already in the table: the boiling point `T_b`
    /// and latent heat `L` come off the transition, the vapour's specific gas
    /// constant `R_v` off the gas, and `P_ref` is the pressure the boiling
    /// point was quoted at. So at the boiling point the air can hold exactly
    /// one atmosphere of vapour, which is what boiling *means*, and below it
    /// the curve falls off the way real vapour pressure does — water at
    /// 283 K comes out at 0.015 atmospheres against a measured 0.012. That
    /// one curve is what makes a pond evaporate without boiling, humid air
    /// fog on a cold lid, and a still's spirit leave its water behind.
    pub fn saturation_pressure(&self, volatile: &Volatile, temperature: Scalar) -> Scalar {
        if temperature <= 0.0 {
            return 0.0;
        }
        let r_v = self.get(volatile.vapour).gas_constant;
        if r_v <= 0.0 {
            return 0.0;
        }
        let exponent =
            (volatile.latent_heat / r_v) * (1.0 / volatile.boiling_k - 1.0 / temperature);
        // Clamped so an absurd temperature cannot overflow `f32`; e^60 is
        // already some 10^26 atmospheres, far past anything a cell reaches.
        self.reference_pressure * exponent.clamp(-80.0, 60.0).exp()
    }

    /// Assigns mixture slots and reads the liquid–vapour pairs off the
    /// transition table. Re-run whenever transitions change, so the two can
    /// never disagree.
    fn index_mixtures(&mut self) {
        let n = self.materials.len();
        let is_gas = |m: &Material| m.phase == Phase::Gas && m.gas_constant > 0.0;
        let mut slots: Vec<MaterialId> = (0..n)
            .filter(|&i| is_gas(&self.materials[i]))
            .map(|i| MaterialId(i as u16))
            .collect();

        self.evaporating.clear();
        self.condensing.clear();
        for tr in &self.transitions {
            let (from, to) = (
                &self.materials[tr.from.0 as usize],
                &self.materials[tr.to.0 as usize],
            );
            let volatile = |liquid, vapour| Volatile {
                liquid,
                vapour,
                boiling_k: tr.threshold_k,
                latent_heat: tr.latent_heat.abs(),
            };
            match tr.direction {
                Direction::Heating if from.phase == Phase::Liquid && is_gas(to) => {
                    self.evaporating.push(volatile(tr.from, tr.to));
                }
                Direction::Cooling if is_gas(from) && to.phase == Phase::Liquid => {
                    self.condensing.push(volatile(tr.to, tr.from));
                    if !slots.contains(&tr.to) {
                        slots.push(tr.to);
                    }
                }
                _ => {}
            }
        }
        assert!(
            slots.len() <= MIX_SLOTS,
            "{} species can share a gas cell in this table, but a cell only has {MIX_SLOTS} slots",
            slots.len()
        );
        let mut slot_of = vec![None; n];
        let mut props = SlotProps::default();
        for (s, id) in slots.iter().enumerate() {
            slot_of[id.0 as usize] = Some(s as u8);
            let m = &self.materials[id.0 as usize];
            props.heat_capacity[s] = m.heat_capacity;
            props.latent_energy[s] = m.latent_energy;
            props.gas_constant[s] = m.gas_constant;
            props.is_gas[s] = is_gas(m);
            props.breathable[s] = m.breathable;
        }
        self.slots = slots;
        self.slot_of = slot_of;
        self.slot_props = props;
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
    pub fn with_chemistry(self, transitions: Vec<Transition>, reactions: Vec<Reaction>) -> Self {
        self.with_biology(transitions, reactions, Vec::new())
    }

    /// [`MaterialTable::with_chemistry`], plus the table's living processes
    /// — see [`Metabolism`]. All three rule kinds feed the same one
    /// relaxation, for the reason spelled out there: a material that takes
    /// part in a phase change, a reaction *and* a metabolism must come out
    /// of it with one enthalpy offset, not three.
    pub fn with_biology(
        mut self,
        transitions: Vec<Transition>,
        reactions: Vec<Reaction>,
        metabolisms: Vec<Metabolism>,
    ) -> Self {
        let n = self.materials.len();

        // Every declared rule reduces to one linear equation in the
        // materials' enthalpy offsets:
        //
        //     Σ_terms  grams · L[material]  ==  rhs
        //
        // with `grams` positive for what the rule produces and negative for
        // what it consumes, and
        //
        //     rhs = -released - Θ · Σ_terms grams · c[material].
        //
        // A two-material transition or reaction arm is just the two-term
        // case of it — which is why this replaced the pairwise solver rather
        // than sitting beside it. A metabolism is the same equation with
        // five or six terms and unequal coefficients.
        struct Constraint {
            terms: Vec<(usize, Scalar)>,
            rhs: Scalar,
            source: &'static str,
        }
        let mut constraints: Vec<Constraint> = Vec::new();
        let mut push = |terms: Vec<(MaterialId, Scalar)>,
                        released: Scalar,
                        threshold_k: Scalar,
                        source: &'static str,
                        materials: &[Material]| {
            // Combine repeated materials, so water appearing on both sides
            // of a process contributes its net coefficient once.
            let mut combined: Vec<(usize, Scalar)> = Vec::new();
            for (id, grams) in terms {
                let i = id.0 as usize;
                match combined.iter_mut().find(|(m, _)| *m == i) {
                    Some(entry) => entry.1 += grams,
                    None => combined.push((i, grams)),
                }
            }
            combined.retain(|&(_, grams)| grams != 0.0);
            if combined.is_empty() {
                return;
            }
            let sensible: Scalar = combined
                .iter()
                .map(|&(m, grams)| grams * materials[m].heat_capacity)
                .sum();
            constraints.push(Constraint {
                terms: combined,
                rhs: -released - threshold_k * sensible,
                source,
            });
        };
        for tr in &transitions {
            // A transition's `latent_heat` is energy *absorbed*.
            push(
                vec![(tr.to, 1.0), (tr.from, -1.0)],
                -tr.latent_heat,
                tr.threshold_k,
                "transition",
                &self.materials,
            );
        }
        for r in &reactions {
            for arm in [r.subject, r.partner] {
                if let Some(heat) = arm.heat {
                    push(
                        vec![(arm.to, 1.0), (arm.from, -1.0)],
                        heat,
                        r.threshold_k,
                        "reaction",
                        &self.materials,
                    );
                }
            }
        }
        for m in &metabolisms {
            if let Some(heat) = m.heat {
                let terms = m
                    .output
                    .iter()
                    .map(|r| (r.material, r.grams))
                    .chain(m.intake.iter().map(|r| (r.material, -r.grams)))
                    .collect();
                push(terms, heat, m.heat_at_k, "metabolism", &self.materials);
            }
        }

        let mut offset: Vec<Option<Scalar>> = vec![None; n];
        // Relaxation: solve any constraint down to its last unknown, seeding
        // a fresh component at zero whenever a pass makes no progress.
        loop {
            let mut progressed = false;
            for c in &constraints {
                let unknown: Vec<usize> = c
                    .terms
                    .iter()
                    .filter(|&&(m, _)| offset[m].is_none())
                    .map(|&(m, _)| m)
                    .collect();
                let known_sum: Scalar = c
                    .terms
                    .iter()
                    .filter_map(|&(m, grams)| offset[m].map(|l| grams * l))
                    .sum();
                match unknown.len() {
                    0 => assert!(
                        (known_sum - c.rhs).abs() <= 1e-2 * (1.0 + c.rhs.abs()),
                        "this {} implies Σ grams·L == {}, but the offsets another rule already \
                         fixed make it {} — the table's chemistry contradicts itself \
                         (materials {:?})",
                        c.source,
                        c.rhs,
                        known_sum,
                        c.terms.iter().map(|&(m, _)| m).collect::<Vec<_>>()
                    ),
                    1 => {
                        let m = unknown[0];
                        let grams = c.terms.iter().find(|&&(x, _)| x == m).map(|&(_, g)| g);
                        if let Some(grams) = grams {
                            offset[m] = Some((c.rhs - known_sum) / grams);
                            progressed = true;
                        }
                    }
                    _ => {}
                }
            }
            if !progressed {
                // Seed the next unresolved component, if any remains.
                match constraints
                    .iter()
                    .flat_map(|c| c.terms.iter())
                    .find(|&&(m, _)| offset[m].is_none())
                {
                    Some(&(m, _)) => offset[m] = Some(0.0),
                    None => break,
                }
            }
        }

        for (i, o) in offset.into_iter().enumerate() {
            if let Some(value) = o {
                self.materials[i].latent_energy = value;
            }
        }

        for m in &metabolisms {
            assert!(
                (m.intake_grams() - m.output_grams()).abs() <= 1e-9 * m.intake_grams().max(1.0),
                "metabolism {:?} takes in {} g and gives back {} g — a living process moves \
                 mass around, it does not make any",
                m.name,
                m.intake_grams(),
                m.output_grams()
            );
        }

        // A reaction arm acting on a gas rewrites one species *inside* a
        // mixture (`src/chemistry.rs`), so it must turn a gas into a gas —
        // there is no honest way to turn one component of a shared cell into
        // a solid without saying where the rest of the cell goes.
        let is_gas = |id: MaterialId| {
            let m = &self.materials[id.0 as usize];
            m.phase == Phase::Gas && m.gas_constant > 0.0
        };
        for r in &reactions {
            for arm in [r.subject, r.partner] {
                assert_eq!(
                    is_gas(arm.from),
                    is_gas(arm.to),
                    "reaction arm {} -> {} crosses between a gas and a non-gas; a gas arm \
                     converts one species of a mixture and must produce another gas",
                    arm.from.0,
                    arm.to.0
                );
            }
        }

        self.transitions = transitions;
        self.reactions = reactions;
        self.metabolisms = metabolisms;
        self.index_mixtures();
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
        ///
        /// It was 310 K until night 5, and 310 K is simply too low — a real
        /// mash is held at 63–70 °C, which is 336–343 K, and a terrarium's
        /// meadow sits near 313. So the first garden with a water bed under
        /// it turned into eight grams of wash overnight, took the gnomes'
        /// footing out from under them (wash is a liquid; juniper is not),
        /// and had them gasping Gin to keep from drowning in their own
        /// allotment. Moving it to the real figure fixes the garden and
        /// costs the still nothing: its hob runs at 365 K.
        const T_MASH: Scalar = 330.0;
        /// Ignition point for a botanical.
        const T_IGNITE: Scalar = 620.0;
        /// Below this a bush is not dormant, it is dead, and what is left of
        /// it is litter.
        const T_FROST_KILL: Scalar = 274.0;
        /// Heat given up by a bush freezing to death, J/g. A leaf is mostly
        /// water and it is a little of that water freezing that kills it, so
        /// this is a few per cent of water's 334.
        ///
        /// It has to be non-zero, and the reason is worth knowing:
        /// `physics::apply_phase_changes` runs a transition by accumulating
        /// the cell's *departure* from the threshold as latent progress, and
        /// bails out of any transition whose latent heat is zero. So a
        /// zero-cost death would never happen at all. Given that, a small
        /// figure is the useful one — it buys frost-hardiness, because a dip
        /// below the threshold for a step or two no longer kills a bush,
        /// while sustained cold does.
        const L_FROST_KILL: Scalar = 20.0;
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
        // Oxygen: 8.314/32. Heavier than the air it is mixed into, which is
        // why a still room's oxygen sits very slightly low — and why a
        // gnome in a cellar is in more trouble than one on a ladder.
        const R_O2: Scalar = 0.2598;
        const RHO_AIR: Scalar = 0.0012;
        const P0: Scalar = RHO_AIR * R_AIR * T0;
        /// Mole fraction of oxygen in open air.
        const X_O2: Scalar = 0.21;

        let mut materials = vec![Material::new(0.0, 0.0, 0.0, 0.0, Phase::Gas, (0, 0, 0)); 17];
        // "Air" is now the *inert* bulk of the atmosphere — the nitrogen and
        // argon that a gnome breathes in and straight back out, and that a
        // fire leaves alone. What a gnome actually needs is `OXYGEN`, which
        // is a separate species sharing the same cells. Air keeps the
        // standard air gas constant rather than nitrogen's 0.2968: every
        // scenario in this crate is tuned against the density it implies,
        // and the 3% difference buys nothing that is worth re-tuning three
        // scenarios for.
        materials[t::AIR.0 as usize] =
            Material::new(RHO_AIR, 0.02, 1.005, 0.05, Phase::Gas, (16, 18, 28))
                .with_gas_constant(R_AIR);
        materials[t::OXYGEN.0 as usize] = Material::new(
            P0 / (R_O2 * T0),
            0.02,
            0.918,
            0.05,
            Phase::Gas,
            (60, 110, 170),
        )
        .breathable()
        .with_gas_constant(R_O2);
        materials[t::STEAM.0 as usize] = Material::new(
            P0 / (R_STEAM * T0),
            0.01,
            2.08,
            0.04,
            Phase::Gas,
            (170, 180, 200),
        )
        .with_gas_constant(R_STEAM);
        // Carbon dioxide: heavier than air at the same pressure, and not
        // breathable. `NORTH_STARS.md` #4 names ONI's gas handling as one of
        // the things this project exists to fix — "CO2 doesn't actually
        // settle the way ONI's simplified layers show it". Here it settles
        // because it is genuinely heavier, by the same one rule that sinks
        // sand through air.
        // Drawn amber, and bright. Every gas here is false-coloured, and now
        // that a gas cell is drawn as the blend of what is in it, a gas
        // only shows up in a mixture if its colour is far from the air's:
        // the dusky purple CO₂ used to be was invisible at a quarter of a
        // room's air, which is exactly the concentration worth seeing.
        materials[t::CO2.0 as usize] = Material::new(
            P0 / (R_CO2 * T0),
            0.03,
            0.844,
            0.04,
            Phase::Gas,
            (235, 150, 55),
        )
        .with_gas_constant(R_CO2);
        // Thermal expansion: water's real coefficient is 2e-4 per kelvin at
        // room temperature, rising to 7e-4 near boiling; ethanol's is about
        // 1.1e-3. Taken near the top of water's range, because what it
        // decides here is whether a pot heated from below turns over, and a
        // real one very much does.
        materials[t::WATER.0 as usize] =
            Material::new(1.0, 0.5, 4.186, 0.6, Phase::Liquid, (40, 90, 200))
                .with_thermal_expansion(5e-4)
                .with_opacity(0.2);
        materials[t::ICE.0 as usize] =
            Material::new(0.92, 0.0, 2.093, 2.2, Phase::Solid, (170, 210, 240)).with_opacity(0.15);
        materials[t::SAND.0 as usize] =
            Material::new(1.6, 0.0, 0.83, 0.3, Phase::Solid, (200, 175, 105))
                .with_mobility(Mobility::Granular);
        materials[t::STONE.0 as usize] =
            Material::new(2.5, 0.0, 0.8, 2.0, Phase::Solid, (105, 105, 115));
        materials[t::LAVA.0 as usize] =
            Material::new(2.4, 8.0, 1.0, 1.5, Phase::Liquid, (235, 110, 40));
        // A bush shades what is under it — nearly, but not quite, wholly:
        // enough that a second bush directly beneath a first one grows
        // slowly, which is the whole of why plants have a shape.
        materials[t::JUNIPER.0 as usize] =
            Material::new(0.5, 0.0, 2.0, 0.2, Phase::Solid, (70, 130, 90)).with_opacity(0.8);
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
            Material::new(1.02, 0.6, 3.9, 6.0, Phase::Liquid, (150, 112, 62))
                .with_thermal_expansion(5e-4)
                .with_opacity(0.45);
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
            Material::new(0.94, 0.35, 2.44, 0.6, Phase::Liquid, (196, 224, 236))
                .with_thermal_expansion(1.1e-3)
                .with_opacity(0.12);
        // Glass: stone that light goes through. Every number here is
        // stone's except `opacity`, deliberately — a lid that behaved
        // differently in any other way would be a second mechanism where one
        // data field will do.
        materials[t::GLASS.0 as usize] =
            Material::new(2.5, 0.0, 0.84, 1.0, Phase::Solid, (150, 172, 184)).with_opacity(0.05);
        materials[t::CHARCOAL.0 as usize] =
            Material::new(0.45, 0.0, 0.84, 0.25, Phase::Solid, (38, 34, 32))
                .with_mobility(Mobility::Granular);
        // Dead plant matter, and the mould that eats it. Both keep juniper's
        // heat capacity, and that is not laziness: a gram of dead leaf holds
        // the same chemical energy as the gram of live leaf it was, so with
        // `c` equal the enthalpy solver gives them equal offsets too, and a
        // bush dying releases nothing. Death is not a source of heat.
        //
        // Both are far lighter than a bush (0.5) — a cell of leaf litter is
        // mostly air, and the jar's whole carbon budget is a couple of grams,
        // so a compost heap you could actually see had to be able to exist
        // without eating the garden that fed it.
        materials[t::LITTER.0 as usize] =
            Material::new(0.03, 0.0, 2.0, 0.15, Phase::Solid, (124, 84, 48))
                .with_mobility(Mobility::Granular)
                .with_opacity(0.6);
        materials[t::FUNGUS.0 as usize] =
            Material::new(0.03, 0.0, 2.0, 0.18, Phase::Solid, (200, 170, 235)).with_opacity(0.5);

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
            // Frost kill. A bush below freezing is not a bush any more, and
            // what is left is litter for something else to eat. One row, and
            // it is the same machinery ice uses; nothing in code knows that
            // this particular transition is a death.
            cooling(t::JUNIPER, t::LITTER, T_FROST_KILL, L_FROST_KILL),
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

        // Biology. Two rows, and they are each other's exact reverse — the
        // carbon cycle, written once.
        //
        // The proportions are the real ones. Photosynthesis is
        // `6 CO₂ + 6 H₂O → C₆H₁₂O₆ + 6 O₂`, and per gram of sugar (180
        // g/mol) that is 264/180 g of carbon dioxide and 108/180 g of water
        // in, 192/180 g of oxygen out. Juniper stands in for the sugar,
        // which is the one liberty taken: a bush is not glucose, but its
        // carbon came in through this door and leaves through it.
        const CO2_PER_G: Scalar = 264.0 / 180.0;
        const H2O_PER_G: Scalar = 108.0 / 180.0;
        const O2_PER_G: Scalar = 192.0 / 180.0;
        /// Energy stored per gram of plant built, J. Tuned, and flagged the
        /// same way `Q_BURN` above is: real photosynthesis banks about
        /// 15 600 J in a gram of sugar, and a cell of plant respiring that
        /// back at any watchable rate would cook itself, because a cell
        /// cannot shed heat the way a leaf in moving air does. This figure
        /// is the same order as the latent heats already in this table,
        /// which is the scale the rest of the simulation is tuned at.
        const Q_PHOTO: Scalar = 2000.0;
        /// The band a plant works in: above freezing, below the temperature
        /// at which its own water would be leaving faster than it arrives.
        const T_GROW_MIN: Scalar = 279.0;
        const T_GROW_MAX: Scalar = 330.0;
        /// Light, as a fraction of full sun, that divides a plant's day
        /// from its night.
        const DAYLIGHT: Scalar = 0.15;
        /// Share of a gram of dead matter that a decomposer keeps as its own
        /// body; the rest it burns. Real fungal growth efficiency on leaf
        /// litter is 30–50%.
        const FUNGUS_YIELD: Scalar = 0.4;
        /// Cold enough to stop rot. A compost heap does not work in a
        /// freezer, which is the whole reason a freezer works.
        const T_ROT_MIN: Scalar = 273.0;

        let metabolisms = vec![
            // Photosynthesis: the one declared energy figure in the cycle.
            // Everything downstream — how much heat a rotting bush gives
            // back, how warm a gnome's breath is — follows from it.
            Metabolism {
                name: "photosynthesis",
                host: t::JUNIPER,
                intake: vec![reagent(t::CO2, CO2_PER_G), reagent(t::WATER, H2O_PER_G)],
                output: vec![reagent(t::JUNIPER, 1.0), reagent(t::OXYGEN, O2_PER_G)],
                rate: 8.0e-3,
                light_min: DAYLIGHT,
                light_max: 1.0,
                min_k: T_GROW_MIN,
                max_k: T_GROW_MAX,
                heat: Some(-Q_PHOTO),
                heat_at_k: T0,
            },
            // ...and a plant's own respiration, which is that run backwards
            // in the dark. Declaring nothing is the point: its heat is
            // forced to be exactly what photosynthesis banked, so a bush
            // cannot be a battery that gains on the round trip.
            Metabolism {
                name: "plant respiration",
                host: t::JUNIPER,
                intake: vec![reagent(t::JUNIPER, 1.0), reagent(t::OXYGEN, O2_PER_G)],
                output: vec![reagent(t::CO2, CO2_PER_G), reagent(t::WATER, H2O_PER_G)],
                rate: 2.0e-4,
                light_min: 0.0,
                light_max: DAYLIGHT,
                min_k: T_GROW_MIN,
                max_k: T_IGNITE,
                heat: None,
                heat_at_k: T0,
            },
            // Leaf fall. A bush spends a little of itself as dead matter,
            // whatever the weather and whatever the light — which is the
            // only reason this jar has ever had anything dead in it.
            //
            // The rate is the load-bearing number and it is bounded from
            // both sides: fast enough that a compost layer builds up while
            // somebody is watching, slow enough to stay well under what the
            // garden photosynthesises in the same time (about 8e-6 g/step
            // per lit cell in the terrarium, itself limited by how much
            // carbon dioxide the jar has). Shed faster than it grows and the
            // garden composts itself.
            Metabolism {
                name: "leaf fall",
                host: t::JUNIPER,
                intake: vec![reagent(t::JUNIPER, 1.0)],
                output: vec![reagent(t::LITTER, 1.0)],
                rate: 3.0e-6,
                light_min: 0.0,
                light_max: 1.0,
                min_k: T_GROW_MIN,
                max_k: T_IGNITE,
                heat: None,
                heat_at_k: T0,
            },
            // Rot, and the mould that is the visible half of it. Both rows
            // are the same chemistry — `C₆H₁₂O₆ + 6 O₂ → 6 CO₂ + 6 H₂O` at
            // photosynthesis's own proportions, scaled by the share of the
            // dead matter that is actually respired — and they differ only
            // in which cell is the host.
            //
            // The first one is what makes this self-starting: litter rots by
            // itself, so a jar does not have to be seeded with mould for
            // anything to decompose, and the fungus it puts out lands in a
            // neighbouring cell, which is the mould you can see. The second
            // is that mould feeding on the litter *around* it, which is how
            // it spreads through a heap rather than sitting on one cell of
            // it.
            //
            // Carbon balances across both: a gram of litter is 0.4 g of
            // carbon, and 0.4 g of fungus plus 0.88 g of CO₂ is 0.16 + 0.24.
            Metabolism {
                name: "rot",
                host: t::LITTER,
                intake: vec![
                    reagent(t::LITTER, 1.0),
                    reagent(t::OXYGEN, (1.0 - FUNGUS_YIELD) * O2_PER_G),
                ],
                output: vec![
                    reagent(t::FUNGUS, FUNGUS_YIELD),
                    reagent(t::CO2, (1.0 - FUNGUS_YIELD) * CO2_PER_G),
                    reagent(t::WATER, (1.0 - FUNGUS_YIELD) * H2O_PER_G),
                ],
                rate: 1.0e-4,
                light_min: 0.0,
                light_max: 1.0,
                min_k: T_ROT_MIN,
                max_k: T_GROW_MAX,
                // The one declared figure in the rot cycle, and it is not a
                // new one: it is the share of a gram of plant that is
                // actually being burned, times the energy photosynthesis
                // banked in it. So a compost heap warms by exactly what the
                // sun put into the leaves, and cannot warm by more.
                heat: Some((1.0 - FUNGUS_YIELD) * Q_PHOTO),
                heat_at_k: T0,
            },
            Metabolism {
                name: "fungal growth",
                host: t::FUNGUS,
                intake: vec![
                    reagent(t::LITTER, 1.0),
                    reagent(t::OXYGEN, (1.0 - FUNGUS_YIELD) * O2_PER_G),
                ],
                output: vec![
                    reagent(t::FUNGUS, FUNGUS_YIELD),
                    reagent(t::CO2, (1.0 - FUNGUS_YIELD) * CO2_PER_G),
                    reagent(t::WATER, (1.0 - FUNGUS_YIELD) * H2O_PER_G),
                ],
                rate: 1.0e-4,
                light_min: 0.0,
                light_max: 1.0,
                min_k: T_ROT_MIN,
                max_k: T_GROW_MAX,
                heat: None,
                heat_at_k: T0,
            },
            // ...and a fungus with nothing left to eat spends itself, the
            // same way a bush in the dark does. Without this a heap that has
            // finished rotting would stay standing as mould for ever, and
            // the last fifth of the jar's carbon would be locked up in it.
            Metabolism {
                name: "fungal respiration",
                host: t::FUNGUS,
                intake: vec![reagent(t::FUNGUS, 1.0), reagent(t::OXYGEN, O2_PER_G)],
                output: vec![reagent(t::CO2, CO2_PER_G), reagent(t::WATER, H2O_PER_G)],
                rate: 4.0e-5,
                light_min: 0.0,
                light_max: 1.0,
                min_k: T_ROT_MIN,
                max_k: T_IGNITE,
                heat: None,
                heat_at_k: T0,
            },
        ];

        // Open air, by mole fraction: partial pressures add to exactly the
        // reference pressure, so a world painted with it starts flat.
        let atmosphere = vec![
            (t::AIR, (1.0 - X_O2) * P0 / (R_AIR * T0)),
            (t::OXYGEN, X_O2 * P0 / (R_O2 * T0)),
        ];

        MaterialTable::new(materials)
            .with_biology(transitions, reactions, metabolisms)
            .with_reference_pressure(P0)
            .with_atmosphere(atmosphere)
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
    /// The gas a gnome actually breathes and a plant actually makes.
    /// Appended at the end, after the twelve ids every earlier scenario and
    /// test already names.
    pub const OXYGEN: MaterialId = MaterialId(13);
    /// Stone that light goes through: the lid of a terrarium.
    ///
    /// It exists because of a fact discovered the hard way — a jar with a
    /// stone lid is a jar in permanent darkness, and once plants care about
    /// light, that is the difference between a garden and a cellar. Nothing
    /// distinguishes it from stone except [`Material::opacity`], which is the
    /// point: "transparent" is a number in the table, not a rule anywhere.
    pub const GLASS: MaterialId = MaterialId(14);
    /// Dead plant matter: what a bush sheds, and what a frozen one becomes.
    /// Granular, so it falls and piles, and lighter than water, so it floats
    /// on a garden bed rather than sinking through it.
    pub const LITTER: MaterialId = MaterialId(15);
    /// The decomposer. Damp litter goes mouldy (a reaction), and the mould
    /// then eats the litter around it, growing and breathing out the carbon
    /// — which is how carbon gets back into the air without going through
    /// anything's lungs.
    pub const FUNGUS: MaterialId = MaterialId(16);

    /// Every id above, in table order — for tests and reporting that want
    /// to iterate the whole table by name.
    pub const ALL: [MaterialId; 17] = [
        AIR, STEAM, WATER, ICE, SAND, STONE, LAVA, JUNIPER, CO2, WASH, SPIRIT, GIN, CHARCOAL,
        OXYGEN, GLASS, LITTER, FUNGUS,
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
            13 => "oxygen",
            14 => "glass",
            15 => "litter",
            16 => "fungus",
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
