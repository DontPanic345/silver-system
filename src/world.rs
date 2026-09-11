//! The simulated world: a grid of cells that each carry a material, a real
//! mass, a temperature, and their progress through a pending phase change.
//!
//! This is deliberately a different thing from [`crate::grid::Grid`], which
//! holds a material id per cell and nothing else. `Grid` is fine substrate
//! for a renderer or a scenario fixture; it cannot express "this cell holds
//! 0.7 g of water at 291 K, one third of the way through freezing", and
//! every conservation property this crate cares about needs exactly that.
//!
//! ## What conservation means here
//!
//! Two quantities are tracked, both summed in `f64` regardless of
//! [`Scalar`]:
//!
//! - **Mass** — `Σ cell.mass`.
//! - **Energy** — `Σ cell.mass * (c * T + latent_offset + progress)`, where
//!   `c` and `latent_offset` come from the cell's material (see
//!   [`Material::specific_energy`]) and `progress` is the latent energy per
//!   unit mass already committed to a phase change in flight.
//!
//! Every operation in [`crate::physics`] conserves both *by construction*,
//! not by a correction step: movement is a swap of whole cell records,
//! conduction is a symmetric pairwise transfer, and a phase change is an
//! algebraic rewrite that holds `mass * specific_energy` fixed across the
//! material switch.
//!
//! The single sanctioned exception is gnome magic. It does not sneak mass
//! or energy in — it goes through [`World::conjure_mass`] /
//! [`World::conjure_energy`], which record exactly what they injected in
//! the [`Ledger`]. So the standing invariant is not "mass never changes"
//! but the stronger, checkable:
//!
//! ```text
//! total_mass_now == total_mass_at_start + ledger.mass_conjured
//! ```
//!
//! with the same shape for energy. That is the Gnomes north star (magic as
//! the one accounted-for hole in an otherwise closed world) expressed as an
//! assertion a test can make — see `physics::tests` and
//! [`World::conservation_residuals`].

use crate::material::{Direction, Material, MaterialId, MaterialTable, Transition, MIX_SLOTS};
use crate::math::{GridIndex, Scalar};

/// Sentinel for [`World::pending`]: this cell has no phase change in
/// flight.
pub const NO_PENDING: u8 = u8::MAX;

/// A gas cell's composition: grams of each species, indexed by
/// [`MaterialTable::slot`].
pub type Mix = [Scalar; MIX_SLOTS];

/// An empty composition — what every non-gas cell carries.
pub const NO_MIX: Mix = [0.0; MIX_SLOTS];

/// A whole cell's state, as a value — what movement swaps and what
/// [`World::cell`] hands back.
///
/// ## Two kinds of cell
///
/// A cell of liquid or solid holds one material, and `mass` is how much of
/// it. A **gas cell** holds a *mixture*: `mix` carries the grams of every
/// species in it — air, steam, CO₂, spirit vapour, and any liquid
/// condensed out of them and still hanging in the air as mist — and for a
/// gas cell:
///
/// - `mass` is the total of `mix`, kept in step by [`Cell::refresh`];
/// - `material` is only a *label*: the gas species with the most mass in
///   the cell. Rendering, reports, a gnome's "can I breathe this", and a
///   reaction's "is there air here" read it. Nothing that moves mass or
///   energy does — they all read `mix`.
///
/// This is what lets two gases share a cell, and it is the whole of what
/// was missing from night 2's gas: a cell of CO₂ next to a cell of air
/// could only ever swap with it, never mix into it, so a heavy gas could do
/// nothing but pile up on the floor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    pub material: MaterialId,
    /// Mass held by this cell, in grams (see
    /// [`MaterialTable::terrarium`]'s note on units). For a gas cell, the
    /// sum of `mix`.
    pub mass: Scalar,
    /// Temperature in kelvin.
    pub temperature: Scalar,
    /// Latent energy per unit mass already committed toward the pending
    /// phase change. Positive while melting or boiling, negative while
    /// freezing or condensing, zero when [`Cell::pending`] is
    /// [`NO_PENDING`].
    pub progress: Scalar,
    /// Index into [`MaterialTable::transitions`] of the phase change this
    /// cell is part-way through, or [`NO_PENDING`].
    pub pending: u8,
    /// Which way this cell was last flowing horizontally: `+1`, `-1`, or
    /// `0` for "no memory".
    ///
    /// The cheapest possible stand-in for horizontal momentum, and it
    /// earns its byte. Without it a fluid cell offered free space on both
    /// sides picks by whatever tie-break the loop happens to use, and a
    /// cell sitting at a junction spends forever stepping one way and
    /// straight back the other, so water never traverses a pipe. With it,
    /// water that entered a cell moving right tries right again first.
    pub flow_dir: i8,
    /// Grams of each species in a gas cell — see the type's doc comment.
    /// All zero for anything else.
    pub mix: Mix,
}

impl Cell {
    /// A cell of `material` at `temperature`, massing whatever a full cell
    /// of that material masses, with no phase change in flight. A gas is
    /// painted as a pure mixture of one species at one atmosphere.
    pub fn full(materials: &MaterialTable, material: MaterialId, temperature: Scalar) -> Self {
        let mass = materials.get(material).density;
        let mut cell = Cell {
            material,
            mass,
            temperature,
            progress: 0.0,
            pending: NO_PENDING,
            flow_dir: 0,
            mix: NO_MIX,
        };
        if let (true, Some(s)) = (materials.is_gas(material), materials.slot(material)) {
            cell.mix[s] = mass;
        }
        cell
    }

    /// A gas cell holding `parts` (species and grams) at `temperature`.
    pub fn gas(
        materials: &MaterialTable,
        parts: &[(MaterialId, Scalar)],
        temperature: Scalar,
    ) -> Self {
        let mut cell = Cell {
            material: materials.vacuum_label(),
            mass: 0.0,
            temperature,
            progress: 0.0,
            pending: NO_PENDING,
            flow_dir: 0,
            mix: NO_MIX,
        };
        for &(id, grams) in parts {
            let s = materials
                .slot(id)
                .unwrap_or_else(|| panic!("material {} cannot be part of a gas mixture", id.0));
            cell.mix[s] += grams;
        }
        cell.refresh(materials);
        cell
    }

    /// A cell of open air — whatever [`MaterialTable::atmosphere`] says that
    /// is — at `temperature`. Falls back to a full cell of the table's first
    /// gas for a table that never declared one.
    pub fn open_air(materials: &MaterialTable, temperature: Scalar) -> Self {
        let mix = materials.atmosphere();
        if mix.is_empty() {
            return Cell::full(materials, materials.vacuum_label(), temperature);
        }
        Cell::gas(materials, mix, temperature)
    }

    /// Whether this is a gas cell, whose contents live in [`Cell::mix`].
    pub fn is_gas(&self, materials: &MaterialTable) -> bool {
        materials.is_gas(self.material)
    }

    /// The partial pressure of everything in this cell that a gnome can
    /// actually breathe — see [`crate::material::Material::breathable`].
    ///
    /// This is the reading that replaced "is this cell labelled with a
    /// breathable material". A mixture has no single answer to that
    /// question: a cell three-quarters full of CO₂ is labelled CO₂ and still
    /// holds a quarter of a lungful, and a cell of fog is labelled steam and
    /// holds a full one. What a gnome cares about is how much oxygen is
    /// pressing on it, which is a sum, not a label.
    pub fn breathable_pressure(&self, materials: &MaterialTable) -> Scalar {
        if !self.is_gas(materials) {
            return 0.0;
        }
        let props = materials.slot_props();
        self.mix
            .iter()
            .enumerate()
            .filter(|&(s, _)| props.breathable[s])
            .map(|(s, &m)| m * props.gas_constant[s])
            .sum::<Scalar>()
            * self.temperature
    }

    /// Re-derives a gas cell's total mass and its label from its mixture.
    /// Every operation that edits `mix` finishes with this, so the two
    /// never disagree. A cell with no gas in it at all keeps its label.
    pub fn refresh(&mut self, materials: &MaterialTable) {
        if !self.is_gas(materials) {
            return;
        }
        self.mass = self.mix.iter().sum();
        let is_gas = &materials.slot_props().is_gas;
        let mut best: Option<(usize, Scalar)> = None;
        for (s, (&grams, &gas)) in self.mix.iter().zip(is_gas.iter()).enumerate() {
            if !gas || grams <= 0.0 {
                continue;
            }
            if best.is_none_or(|(_, m)| grams > m) {
                best = Some((s, grams));
            }
        }
        if let Some((s, _)) = best {
            self.material = materials.slots()[s];
        }
    }

    /// Heat capacity of the whole cell, J/K: `Σ m·c` over its contents.
    pub fn capacity(&self, materials: &MaterialTable) -> f64 {
        if self.is_gas(materials) {
            dot(&self.mix, &materials.slot_props().heat_capacity)
        } else {
            self.mass * materials.get(self.material).heat_capacity
        }
    }

    /// Energy the cell holds over and above its sensible heat, J: `Σ m·L`
    /// over its contents, plus any latent progress already committed to a
    /// phase change.
    pub fn stored(&self, materials: &MaterialTable) -> f64 {
        if self.is_gas(materials) {
            dot(&self.mix, &materials.slot_props().latent_energy)
        } else {
            self.mass * (materials.get(self.material).latent_energy + self.progress)
        }
    }

    /// This cell's total energy in joules, the quantity
    /// [`World::total_energy`] sums: `capacity·T + stored`.
    pub fn energy(&self, materials: &MaterialTable) -> f64 {
        self.capacity(materials) * self.temperature + self.stored(materials)
    }

    /// Sets the temperature that makes this cell's energy exactly `joules`,
    /// given what it now contains. The one move every mixing, evaporating
    /// and reacting operation ends with, and the reason each of them
    /// conserves energy by construction: whatever the contents became, the
    /// temperature is *solved* from the energy rather than assigned.
    pub fn solve_temperature(&mut self, materials: &MaterialTable, joules: f64) {
        let capacity = self.capacity(materials);
        if capacity > 0.0 {
            self.temperature = ((joules - self.stored(materials)) / capacity) as Scalar;
        }
    }

    /// Gas pressure, `P = T · Σ m·R` over the mixture. Liquid mist in the
    /// mix has no gas constant and so adds mass without adding pressure,
    /// which is what a droplet does. Zero for any non-gas cell.
    pub fn pressure(&self, materials: &MaterialTable) -> Scalar {
        if !self.is_gas(materials) {
            return 0.0;
        }
        dot(&self.mix, &materials.slot_props().gas_constant) * self.temperature
    }

    /// Grams of `species` in this cell: its share of a gas cell's mixture,
    /// or the whole cell if it is a cell of that material.
    pub fn grams_of(&self, materials: &MaterialTable, species: MaterialId) -> Scalar {
        if self.is_gas(materials) {
            materials.slot(species).map_or(0.0, |s| self.mix[s])
        } else if self.material == species {
            self.mass
        } else {
            0.0
        }
    }
}

/// `Σ a·b` over a mixture and a per-slot property.
fn dot(mix: &Mix, per_slot: &[Scalar; MIX_SLOTS]) -> Scalar {
    mix.iter().zip(per_slot.iter()).map(|(&m, &x)| m * x).sum()
}

/// Running total of everything gnome magic has added to (or removed from)
/// the world. The world is closed except through this.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Ledger {
    /// Net mass conjured into the world, in grams. Negative when magic has
    /// banished more mass than it created.
    pub mass_conjured: f64,
    /// Net energy conjured into the world, in joules.
    pub energy_conjured: f64,
    /// Total Gin spent to pay for the two figures above — the resource
    /// side of the same ledger.
    pub gin_spent: f64,
}

/// How far the world has drifted from "closed, except for what the ledger
/// admits to". Both fields should stay at floating-point noise for any
/// number of steps; a non-trivial value is a physics bug, not a tuning
/// question.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Residuals {
    pub mass: f64,
    pub energy: f64,
    /// `mass` as a fraction of the world's starting mass.
    pub mass_relative: f64,
    /// `energy` as a fraction of the world's starting energy.
    pub energy_relative: f64,
}

/// The grid of cells, structure-of-arrays, plus the material table it is
/// interpreted through and the magic ledger.
pub struct World {
    width: usize,
    height: usize,
    material: Vec<MaterialId>,
    mass: Vec<Scalar>,
    temperature: Vec<Scalar>,
    progress: Vec<Scalar>,
    pending: Vec<u8>,
    flow_dir: Vec<i8>,
    mix: Vec<Mix>,
    materials: MaterialTable,
    ledger: Ledger,
    /// What the water cycle has done — see [`crate::vapour::Tally`].
    pub(crate) tally: crate::vapour::Tally,
    /// What life has done — see [`crate::life::Tally`].
    pub(crate) life: crate::life::Tally,
    /// How much of the sky's light reaches each cell, as a fraction of full
    /// sun — recomputed every step by `src/light.rs`.
    pub(crate) light: Vec<Scalar>,
    /// How bright the sky over this world is, 0 to 1. Set by whatever is
    /// driving the day — see [`crate::light::Sun`] — and read by nothing
    /// except the light march.
    ///
    /// A bare `World` starts at full daylight that *illuminates but does
    /// not warm*: the joules sunlight carries are a scenario's business, the
    /// same way a [`crate::terrarium::Thermostat`]'s are, because they go
    /// through the ledger and the ledger belongs to the scenario.
    pub sky: Scalar,
    /// A floor under the light field, 0 to 1: light that reaches every cell
    /// regardless of what is in the way.
    ///
    /// It exists because "how much of the *sky* can this cell see" is the
    /// wrong question for a scene that is indoors. The still and the gas
    /// chamber are sealed stone boxes standing in a workshop; marching
    /// daylight down through their roofs correctly answers that nothing
    /// inside them can see the sky, and then renders both pages a quarter
    /// darker than they were, for a reason that has nothing to do with
    /// either demonstration. A scenario that is lit says so, in a number.
    pub ambient: Scalar,
    /// Gas mass flux across each cell's right-hand face, g/s, positive
    /// toward `+i` — the gas's momentum. See `gas::flow`.
    pub(crate) flux_x: Vec<Scalar>,
    /// The same across each cell's upper face, positive toward `+j`.
    pub(crate) flux_y: Vec<Scalar>,
    initial_mass: f64,
    initial_energy: f64,
    /// Steps taken, used to alternate the left/right bias in flow so
    /// liquids don't drift consistently one way.
    pub(crate) step_count: u64,
}

impl World {
    /// Builds a `width` x `height` world filled with `fill` at
    /// `temperature`, and snapshots its mass and energy as the baseline
    /// every later conservation check is measured against.
    pub fn new(
        width: usize,
        height: usize,
        materials: MaterialTable,
        fill: MaterialId,
        temperature: Scalar,
    ) -> Self {
        let n = width * height;
        let template = Cell::full(&materials, fill, temperature);
        let mut world = World {
            width,
            height,
            material: vec![fill; n],
            mass: vec![template.mass; n],
            temperature: vec![temperature; n],
            progress: vec![0.0; n],
            pending: vec![NO_PENDING; n],
            flow_dir: vec![0; n],
            mix: vec![template.mix; n],
            materials,
            ledger: Ledger::default(),
            tally: crate::vapour::Tally::default(),
            life: crate::life::Tally::default(),
            light: vec![1.0; n],
            sky: 1.0,
            ambient: 0.0,
            flux_x: vec![0.0; n],
            flux_y: vec![0.0; n],
            initial_mass: 0.0,
            initial_energy: 0.0,
            step_count: 0,
        };
        world.rebaseline();
        world
    }

    /// A `width` x `height` world full of **open air** at `temperature` —
    /// the table's declared [atmosphere](MaterialTable::atmosphere), a
    /// mixture, rather than a single pure gas.
    ///
    /// This is what every scenario and every test uses, and the distinction
    /// from [`World::new`] with a gas fill is not cosmetic: a world filled
    /// with pure "air" has no oxygen in it at all, so nothing can breathe and
    /// nothing can burn. `World::new` still exists for a world that really
    /// is meant to be full of one substance — a tank of CO₂, a block of ice.
    pub fn new_open(
        width: usize,
        height: usize,
        materials: MaterialTable,
        temperature: Scalar,
    ) -> Self {
        let fill = materials.vacuum_label();
        let mut world = World::new(width, height, materials, fill, temperature);
        let air = Cell::open_air(&world.materials, temperature);
        for p in 0..width * height {
            world.set_cell_at(p, air);
        }
        world.rebaseline();
        world
    }

    /// Paints one cell with open air at `temperature` — [`World::fill`]'s
    /// counterpart for the atmosphere, which is a mixture and so has no
    /// single material to name.
    pub fn fill_atmosphere(&mut self, index: GridIndex, temperature: Scalar) {
        let air = Cell::open_air(&self.materials, temperature);
        self.set_cell(index, air);
    }

    /// How much of the sky's light reaches the cell at flat position `p`, as
    /// a fraction of full sun — see `src/light.rs`.
    pub fn light_at(&self, p: usize) -> Scalar {
        self.light[p]
    }

    /// The light field, in [`World::linear_index`] order.
    pub fn light_cells(&self) -> &[Scalar] {
        &self.light
    }

    /// Re-snapshots the conservation baseline from the world's current
    /// state, and clears the ledger. Call this once after scenario setup
    /// has painted cells in (setup is world-building, not physics, so it
    /// shouldn't register as a conservation violation) and never again
    /// during a run — the whole point is that the baseline is fixed while
    /// physics runs.
    pub fn rebaseline(&mut self) {
        self.initial_mass = self.total_mass();
        self.initial_energy = self.total_energy();
        self.ledger = Ledger::default();
        self.tally = crate::vapour::Tally::default();
        self.life = crate::life::Tally::default();
    }

    /// What the water cycle has done since the baseline — grams evaporated,
    /// condensed and rained. See [`crate::vapour::Tally`].
    pub fn tally(&self) -> crate::vapour::Tally {
        self.tally
    }

    /// What life has done since the baseline — grams of living matter built
    /// and spent. See [`crate::life::Tally`].
    pub fn life_tally(&self) -> crate::life::Tally {
        self.life
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn materials(&self) -> &MaterialTable {
        &self.materials
    }

    pub fn ledger(&self) -> Ledger {
        self.ledger
    }

    /// Whether `index` names a cell inside this world.
    pub fn in_bounds(&self, index: GridIndex) -> bool {
        index.i >= 0
            && index.j >= 0
            && (index.i as usize) < self.width
            && (index.j as usize) < self.height
    }

    /// Flat storage position of `index`, row-major with `i` fastest — the
    /// same convention [`crate::grid::Grid`] uses.
    pub fn linear_index(&self, index: GridIndex) -> usize {
        debug_assert!(self.in_bounds(index), "{index:?} out of bounds");
        index.j as usize * self.width + index.i as usize
    }

    pub fn cell(&self, index: GridIndex) -> Cell {
        self.cell_at(self.linear_index(index))
    }

    pub(crate) fn cell_at(&self, p: usize) -> Cell {
        Cell {
            material: self.material[p],
            mass: self.mass[p],
            temperature: self.temperature[p],
            progress: self.progress[p],
            pending: self.pending[p],
            flow_dir: self.flow_dir[p],
            mix: self.mix[p],
        }
    }

    /// Writes a whole cell. A gas cell's total mass and label are
    /// re-derived from its mixture on the way in, so a caller that edits
    /// `mix` cannot leave `mass` disagreeing with it.
    pub fn set_cell(&mut self, index: GridIndex, cell: Cell) {
        let p = self.linear_index(index);
        self.set_cell_at(p, cell);
    }

    pub(crate) fn set_cell_at(&mut self, p: usize, mut cell: Cell) {
        if cell.is_gas(&self.materials) {
            // A gas cell's contents are its mixture; `mass` is derived from
            // it. A caller that relabels a cell as a gas but forgets to say
            // what is in it would silently delete its mass here, so that is
            // loud instead.
            assert!(
                cell.mass <= 0.0 || cell.mix.iter().any(|&m| m > 0.0),
                "gas cell written with {} g of mass but an empty mixture — set `mix`, \
                 not `mass`, on a gas cell",
                cell.mass
            );
            cell.refresh(&self.materials);
            // Gas cells never carry a phase change in flight: their
            // condensing and evaporating is done by `src/vapour.rs` on the
            // mixture, not by the per-cell latent accumulator.
            cell.pending = NO_PENDING;
            cell.progress = 0.0;
        } else {
            cell.mix = NO_MIX;
        }
        self.material[p] = cell.material;
        self.mass[p] = cell.mass;
        self.temperature[p] = cell.temperature;
        self.progress[p] = cell.progress;
        self.pending[p] = cell.pending;
        self.flow_dir[p] = cell.flow_dir;
        self.mix[p] = cell.mix;
    }

    /// Whether the cell at flat position `p` is a gas cell.
    pub(crate) fn is_gas_at(&self, p: usize) -> bool {
        self.materials.is_gas(self.material[p])
    }

    /// The gas pressure at flat position `p` — [`Cell::pressure`] without
    /// copying the cell out first, since flow asks it of every pair.
    pub(crate) fn pressure_at(&self, p: usize) -> Scalar {
        if !self.is_gas_at(p) {
            return 0.0;
        }
        dot(&self.mix[p], &self.materials.slot_props().gas_constant) * self.temperature[p]
    }

    /// Sets one cell's temperature and nothing else — for conduction, which
    /// changes nothing else, and runs on every adjacent pair every step.
    pub(crate) fn set_temperature_at(&mut self, p: usize, temperature: Scalar) {
        self.temperature[p] = temperature;
    }

    /// Paints a full cell of `material` at `temperature` into `index` —
    /// scenario setup's workhorse.
    pub fn fill(&mut self, index: GridIndex, material: MaterialId, temperature: Scalar) {
        let cell = Cell::full(&self.materials, material, temperature);
        self.set_cell(index, cell);
    }

    /// The material at `index` — the read a renderer makes.
    pub fn material_at(&self, index: GridIndex) -> MaterialId {
        self.material[self.linear_index(index)]
    }

    /// Raw material array, in [`World::linear_index`] order.
    pub fn material_cells(&self) -> &[MaterialId] {
        &self.material
    }

    /// Raw temperature array, in [`World::linear_index`] order.
    pub fn temperature_cells(&self) -> &[Scalar] {
        &self.temperature
    }

    /// The material data for the cell at flat position `p`.
    pub(crate) fn material_of(&self, p: usize) -> &Material {
        self.materials.get(self.material[p])
    }

    /// Exchanges two cells wholesale. The only movement primitive physics
    /// has, which is why movement cannot lose mass or energy: nothing is
    /// created or destroyed, only relocated.
    pub(crate) fn swap_cells(&mut self, a: usize, b: usize) {
        self.material.swap(a, b);
        self.mass.swap(a, b);
        self.temperature.swap(a, b);
        self.progress.swap(a, b);
        self.pending.swap(a, b);
        self.flow_dir.swap(a, b);
        self.mix.swap(a, b);
    }

    /// Records which way the cell at flat position `p` is flowing — see
    /// [`Cell::flow_dir`].
    pub(crate) fn set_flow_dir(&mut self, p: usize, dir: i8) {
        self.flow_dir[p] = dir;
    }

    pub(crate) fn flow_dir(&self, p: usize) -> i8 {
        self.flow_dir[p]
    }

    pub fn total_mass(&self) -> f64 {
        self.mass.iter().copied().sum()
    }

    pub fn total_energy(&self) -> f64 {
        (0..self.material.len())
            .map(|p| self.cell_at(p).energy(&self.materials))
            .sum()
    }

    /// How much mass of `material` the world currently holds, wherever it
    /// is: whole cells of it, and its share of every gas cell's mixture
    /// (as vapour, or as mist if it is a liquid something condenses into).
    pub fn mass_of(&self, material: MaterialId) -> f64 {
        (0..self.material.len())
            .map(|p| self.grams_of_at(p, material))
            .sum()
    }

    /// Grams of `material` in the cell at flat position `p` — see
    /// [`Cell::grams_of`].
    pub fn grams_of_at(&self, p: usize, material: MaterialId) -> Scalar {
        if self.is_gas_at(p) {
            self.materials
                .slot(material)
                .map_or(0.0, |s| self.mix[p][s])
        } else if self.material[p] == material {
            self.mass[p]
        } else {
            0.0
        }
    }

    /// How many cells currently hold `material` — for a gas, how many gas
    /// cells it is the largest part of.
    pub fn count_of(&self, material: MaterialId) -> usize {
        self.material.iter().filter(|&&m| m == material).count()
    }

    /// Mass-weighted mean temperature of the whole world, in kelvin.
    pub fn mean_temperature(&self) -> f64 {
        let m: f64 = self.total_mass();
        if m == 0.0 {
            return 0.0;
        }
        (0..self.material.len())
            .map(|p| self.mass[p] * self.temperature[p])
            .sum::<f64>()
            / m
    }

    /// The conservation check: current totals against the starting
    /// snapshot, with everything the ledger admits magic added subtracted
    /// back out. Both absolute figures should be float noise.
    pub fn conservation_residuals(&self) -> Residuals {
        let mass = self.total_mass() - (self.initial_mass + self.ledger.mass_conjured);
        let energy = self.total_energy() - (self.initial_energy + self.ledger.energy_conjured);
        Residuals {
            mass,
            energy,
            mass_relative: if self.initial_mass != 0.0 {
                mass / self.initial_mass
            } else {
                0.0
            },
            energy_relative: if self.initial_energy != 0.0 {
                energy / self.initial_energy.abs()
            } else {
                0.0
            },
        }
    }

    // --- The sanctioned exception: magic ---

    /// Adds `joules` of energy to the cell at `index` and records it in the
    /// ledger. The *only* way energy enters or leaves the world, and the
    /// reason a gnome's warming spell doesn't quietly break conservation:
    /// it is conservation-violating on purpose, in public, with a receipt.
    ///
    /// Returns the temperature change actually applied (zero for a
    /// massless or zero-heat-capacity cell, in which case nothing is
    /// charged to the ledger either).
    pub fn conjure_energy(&mut self, index: GridIndex, joules: f64) -> Scalar {
        let p = self.linear_index(index);
        self.conjure_energy_at(p, joules)
    }

    /// [`World::conjure_energy`] by flat position — for anything that walks
    /// the whole grid, such as sunlight landing on it.
    pub fn conjure_energy_at(&mut self, p: usize, joules: f64) -> Scalar {
        let capacity = self.cell_at(p).capacity(&self.materials);
        if capacity <= 0.0 {
            return 0.0;
        }
        let delta_t = joules / capacity;
        self.temperature[p] += delta_t as Scalar;
        self.ledger.energy_conjured += joules;
        delta_t as Scalar
    }

    /// Conjures `grams` of `material` into the cell at `index`, at
    /// `temperature`, and records both the mass and the energy that mass
    /// brought with it. Negative `grams` banishes mass instead.
    ///
    /// What "into" means depends on the two things meeting:
    ///
    /// - **A gas into a gas cell** is *added* to the mixture — a vent
    ///   breathing CO₂ into a room raises the CO₂ in it rather than
    ///   replacing the room. The cell's temperature is re-solved from its
    ///   new energy, so gas conjured cold into warm air cools it.
    /// - **Anything else** *replaces* what was there, and the displaced mass
    ///   and energy leave through the same ledger entry that let the new
    ///   mass in.
    /// - **Banishing** takes `material` out of a gas cell's mixture (a
    ///   scrubber), or takes mass out of a cell of that material (a gnome
    ///   eating a berry). A cell emptied completely becomes an empty gas
    ///   cell — no mass, so no energy, so nothing to book — and the
    ///   atmosphere flows in to fill it on the next step.
    ///
    /// Returns the mass actually moved: positive in, negative out.
    pub fn conjure_mass(
        &mut self,
        index: GridIndex,
        material: MaterialId,
        grams: Scalar,
        temperature: Scalar,
    ) -> Scalar {
        let p = self.linear_index(index);
        let existing = self.cell_at(p);
        let slot = self.materials.slot(material);
        let in_gas_cell = existing.is_gas(&self.materials);

        if grams > 0.0 {
            let added_energy = grams * self.materials.get(material).specific_energy(temperature);
            if let (true, true, Some(s)) = (in_gas_cell, self.materials.is_gas(material), slot) {
                let energy = existing.energy(&self.materials) + added_energy;
                let mut cell = existing;
                cell.mix[s] += grams;
                cell.refresh(&self.materials);
                cell.solve_temperature(&self.materials, energy);
                self.ledger.mass_conjured += grams;
                self.ledger.energy_conjured += added_energy;
                self.set_cell_at(p, cell);
                return grams;
            }
            let existing_energy = existing.energy(&self.materials);
            self.ledger.mass_conjured += grams - existing.mass;
            self.ledger.energy_conjured += added_energy - existing_energy;
            let mut cell = Cell {
                material,
                mass: grams,
                temperature,
                progress: 0.0,
                pending: NO_PENDING,
                flow_dir: 0,
                mix: NO_MIX,
            };
            if let (true, Some(s)) = (self.materials.is_gas(material), slot) {
                cell.mix[s] = grams;
            }
            self.set_cell_at(p, cell);
            return grams;
        }

        // Banishing. Out of a mixture, only the named species leaves (a gas,
        // or mist of a liquid); the rest of the cell is untouched and so is
        // its temperature, because what is left behind still holds exactly
        // its own energy.
        if let (true, Some(s)) = (in_gas_cell, slot) {
            let removed = (-grams).min(existing.mix[s]);
            if removed <= 0.0 {
                return 0.0;
            }
            self.ledger.mass_conjured -= removed;
            self.ledger.energy_conjured -= removed
                * self
                    .materials
                    .get(material)
                    .specific_energy(existing.temperature);
            let mut cell = existing;
            cell.mix[s] -= removed;
            self.set_cell_at(p, cell);
            return -removed;
        }

        let removed = (-grams).min(existing.mass);
        if removed <= 0.0 {
            return 0.0;
        }
        let fraction = removed / existing.mass;
        self.ledger.mass_conjured -= removed;
        self.ledger.energy_conjured -= existing.energy(&self.materials) * fraction;
        let left = existing.mass - removed;
        if left <= 0.0 {
            // Emptied: nothing left to hold energy, so the cell becomes an
            // empty gas cell and needs no booking of its own. It used to be
            // refilled with a full cell of air conjured out of nowhere (and
            // booked); now that gas cells can hold any amount, the room's
            // own air flows in instead, which is what would really happen.
            self.set_cell_at(
                p,
                Cell {
                    material: self.materials.vacuum_label(),
                    mass: 0.0,
                    temperature: existing.temperature,
                    progress: 0.0,
                    pending: NO_PENDING,
                    flow_dir: 0,
                    mix: NO_MIX,
                },
            );
        } else if existing.is_gas(&self.materials) {
            // Banishing a gas that is not in this cell's mixture — the
            // fraction above was taken off the whole cell, so every species
            // gives up the same share.
            let mut cell = existing;
            for m in cell.mix.iter_mut() {
                *m *= (1.0 - fraction) as Scalar;
            }
            self.set_cell_at(p, cell);
        } else {
            self.mass[p] = left;
        }
        -removed
    }

    /// Records Gin spent, for the ledger's resource side. The gnome layer
    /// calls this; physics never does.
    pub fn charge_gin(&mut self, gin: f64) {
        self.ledger.gin_spent += gin;
    }

    /// The transition rules this world's material table carries.
    pub(crate) fn transitions(&self) -> &[Transition] {
        self.materials.transitions()
    }

    /// Latent heat of `transition`, in joules per gram: the enthalpy
    /// difference between its two materials at its threshold. Positive for
    /// a heating transition (energy absorbed), negative for a cooling one.
    pub(crate) fn latent_of(&self, transition: &Transition) -> Scalar {
        let from = self.materials.get(transition.from);
        let to = self.materials.get(transition.to);
        to.specific_energy(transition.threshold_k) - from.specific_energy(transition.threshold_k)
    }

    /// Whether `transition` fires on the way up in temperature.
    pub(crate) fn is_heating(transition: &Transition) -> bool {
        transition.direction == Direction::Heating
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::terrarium as t;

    fn world() -> World {
        World::new_open(4, 4, MaterialTable::terrarium(), 290.0)
    }

    #[test]
    fn a_fresh_world_starts_with_zero_residuals() {
        let w = world();
        let r = w.conservation_residuals();
        assert_eq!(r.mass, 0.0);
        assert_eq!(r.energy, 0.0);
    }

    #[test]
    fn filling_cells_then_rebaselining_resets_the_conservation_baseline() {
        let mut w = world();
        w.fill(GridIndex::new(1, 1), t::WATER, 300.0);
        // Setup moved mass without a ledger entry, so before rebaselining
        // the residual is non-zero — proof the check has teeth.
        assert!(w.conservation_residuals().mass.abs() > 0.5);
        w.rebaseline();
        assert_eq!(w.conservation_residuals().mass, 0.0);
    }

    #[test]
    fn conjured_energy_is_booked_so_the_residual_stays_zero() {
        let mut w = world();
        w.fill(GridIndex::new(2, 2), t::WATER, 300.0);
        w.rebaseline();
        let before = w.cell(GridIndex::new(2, 2)).temperature;
        w.conjure_energy(GridIndex::new(2, 2), 418.6);
        let after = w.cell(GridIndex::new(2, 2)).temperature;
        assert!((after - before - 100.0).abs() < 0.5, "{before} -> {after}");
        assert_eq!(w.ledger().energy_conjured, 418.6);
        assert!(w.conservation_residuals().energy.abs() < 1e-3);
    }

    #[test]
    fn conjured_mass_is_booked_so_the_residual_stays_zero() {
        let mut w = world();
        w.rebaseline();
        w.conjure_mass(GridIndex::new(0, 0), t::WATER, 1.0, 290.0);
        let r = w.conservation_residuals();
        assert!(r.mass.abs() < 1e-6, "mass residual {}", r.mass);
        assert!(r.energy.abs() < 1e-3, "energy residual {}", r.energy);
        assert!(w.ledger().mass_conjured > 0.9);
    }

    #[test]
    fn banished_mass_is_booked_so_the_residual_stays_zero() {
        let mut w = world();
        w.fill(GridIndex::new(0, 0), t::STONE, 290.0);
        w.rebaseline();
        w.conjure_mass(GridIndex::new(0, 0), t::STONE, -2.5, 290.0);
        assert_eq!(w.material_at(GridIndex::new(0, 0)), t::AIR);
        let r = w.conservation_residuals();
        assert!(r.mass.abs() < 1e-6, "mass residual {}", r.mass);
        assert!(r.energy.abs() < 1e-2, "energy residual {}", r.energy);
    }

    #[test]
    fn swapping_two_cells_moves_every_field_together() {
        let mut w = world();
        w.fill(GridIndex::new(0, 0), t::WATER, 350.0);
        let a = w.linear_index(GridIndex::new(0, 0));
        let b = w.linear_index(GridIndex::new(1, 0));
        w.swap_cells(a, b);
        let moved = w.cell(GridIndex::new(1, 0));
        assert_eq!(moved.material, t::WATER);
        assert_eq!(moved.temperature, 350.0);
        assert_eq!(w.material_at(GridIndex::new(0, 0)), t::AIR);
    }

    #[test]
    fn latent_heat_of_melting_ice_is_the_real_physical_number() {
        let w = world();
        let melt = w
            .transitions()
            .iter()
            .find(|tr| tr.from == t::ICE && tr.to == t::WATER)
            .copied()
            .expect("ice -> water transition should exist");
        assert!(
            (w.latent_of(&melt) - 334.0).abs() < 1.0,
            "latent heat of fusion should be ~334 J/g, got {}",
            w.latent_of(&melt)
        );
        let freeze = w
            .transitions()
            .iter()
            .find(|tr| tr.from == t::WATER && tr.to == t::ICE)
            .copied()
            .expect("water -> ice transition should exist");
        assert!(
            (w.latent_of(&freeze) + 334.0).abs() < 1.0,
            "freezing should release what melting absorbs, got {}",
            w.latent_of(&freeze)
        );
    }
}
