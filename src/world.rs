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

use crate::material::{Direction, Material, MaterialId, MaterialTable, Transition};
use crate::math::{GridIndex, Scalar};

/// Sentinel for [`World::pending`]: this cell has no phase change in
/// flight.
pub const NO_PENDING: u8 = u8::MAX;

/// A whole cell's state, as a value — what movement swaps and what
/// [`World::cell`] hands back.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    pub material: MaterialId,
    /// Mass held by this cell, in grams (see
    /// [`MaterialTable::terrarium`]'s note on units).
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
}

impl Cell {
    /// A cell of `material` at `temperature`, massing whatever a full cell
    /// of that material masses, with no phase change in flight.
    pub fn full(materials: &MaterialTable, material: MaterialId, temperature: Scalar) -> Self {
        Cell {
            material,
            mass: materials.get(material).density,
            temperature,
            progress: 0.0,
            pending: NO_PENDING,
            flow_dir: 0,
        }
    }

    /// This cell's total energy in joules, the quantity
    /// [`World::total_energy`] sums.
    pub fn energy(&self, materials: &MaterialTable) -> f64 {
        let m = materials.get(self.material);
        self.mass as f64 * (m.specific_energy(self.temperature) + self.progress) as f64
    }
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
    materials: MaterialTable,
    ledger: Ledger,
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
        let mass = materials.get(fill).density;
        let mut world = World {
            width,
            height,
            material: vec![fill; n],
            mass: vec![mass; n],
            temperature: vec![temperature; n],
            progress: vec![0.0; n],
            pending: vec![NO_PENDING; n],
            flow_dir: vec![0; n],
            materials,
            ledger: Ledger::default(),
            initial_mass: 0.0,
            initial_energy: 0.0,
            step_count: 0,
        };
        world.rebaseline();
        world
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
        }
    }

    pub fn set_cell(&mut self, index: GridIndex, cell: Cell) {
        let p = self.linear_index(index);
        self.set_cell_at(p, cell);
    }

    pub(crate) fn set_cell_at(&mut self, p: usize, cell: Cell) {
        self.material[p] = cell.material;
        self.mass[p] = cell.mass;
        self.temperature[p] = cell.temperature;
        self.progress[p] = cell.progress;
        self.pending[p] = cell.pending;
        self.flow_dir[p] = cell.flow_dir;
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
        self.mass.iter().map(|&m| m as f64).sum()
    }

    pub fn total_energy(&self) -> f64 {
        (0..self.material.len())
            .map(|p| self.cell_at(p).energy(&self.materials))
            .sum()
    }

    /// How much mass of `material` the world currently holds.
    pub fn mass_of(&self, material: MaterialId) -> f64 {
        (0..self.material.len())
            .filter(|&p| self.material[p] == material)
            .map(|p| self.mass[p] as f64)
            .sum()
    }

    /// How many cells currently hold `material`.
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
            .map(|p| self.mass[p] as f64 * self.temperature[p] as f64)
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
        let capacity = self.mass[p] as f64 * self.material_of(p).heat_capacity as f64;
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
    /// brought with it. Negative `grams` banishes mass instead (removing
    /// the cell's own material, whatever it is, and leaving air behind if
    /// the cell empties).
    ///
    /// Returns the mass actually moved.
    pub fn conjure_mass(
        &mut self,
        index: GridIndex,
        material: MaterialId,
        grams: Scalar,
        temperature: Scalar,
    ) -> Scalar {
        let p = self.linear_index(index);
        if grams > 0.0 {
            let existing = self.cell_at(p);
            let existing_energy = existing.energy(&self.materials);
            let added_energy =
                grams as f64 * self.materials.get(material).specific_energy(temperature) as f64;
            // Conjuring into an occupied cell replaces what was there; the
            // displaced mass and energy leave the world through the same
            // ledger that let the new mass in, so the books still balance.
            self.ledger.mass_conjured += grams as f64 - existing.mass as f64;
            self.ledger.energy_conjured += added_energy - existing_energy;
            self.set_cell_at(
                p,
                Cell {
                    material,
                    mass: grams,
                    temperature,
                    progress: 0.0,
                    pending: NO_PENDING,
                    flow_dir: 0,
                },
            );
            grams
        } else {
            let existing = self.cell_at(p);
            let removed = (-grams).min(existing.mass);
            if removed <= 0.0 {
                return 0.0;
            }
            let fraction = removed as f64 / existing.mass as f64;
            self.ledger.mass_conjured -= removed as f64;
            self.ledger.energy_conjured -= existing.energy(&self.materials) * fraction;
            let left = existing.mass - removed;
            if left <= 0.0 {
                let air = crate::material::terrarium::AIR;
                let air_mass = self.materials.get(air).density;
                // The vacated cell fills with air out of nowhere, so that
                // too is conjured and booked.
                self.ledger.mass_conjured += air_mass as f64;
                self.ledger.energy_conjured += air_mass as f64
                    * self
                        .materials
                        .get(air)
                        .specific_energy(existing.temperature) as f64;
                self.set_cell_at(
                    p,
                    Cell {
                        material: air,
                        mass: air_mass,
                        temperature: existing.temperature,
                        progress: 0.0,
                        pending: NO_PENDING,
                        flow_dir: 0,
                    },
                );
            } else {
                self.mass[p] = left;
            }
            -removed
        }
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
        World::new(4, 4, MaterialTable::terrarium(), t::AIR, 290.0)
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
