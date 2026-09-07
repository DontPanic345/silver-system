//! The gas demonstration: a pressurised chamber, a CO₂ vent, and a
//! scrubber — the smallest world in which every claim `src/gas.rs` makes is
//! visible at once.
//!
//! `NORTH_STARS.md` #4 lists ONI's gas handling among the things this
//! project exists to fix: gas that does not behave like gas, CO₂ that does
//! not settle, and players building gimmicks around the gaps. This scenario
//! is the counter-claim, running:
//!
//! - A **pressurised bottle** on the left, sealed behind a stone wall with
//!   one hole in it: ten atmospheres of air that jets out into the room and
//!   equalises, because pressure is a real quantity now and not a label.
//! - A **CO₂ vent** in the floor and a **scrubber** in the ceiling, both
//!   declared holes in the world's books, in exactly the shape
//!   `src/terrarium.rs` already uses for its thermostats: every gram they
//!   add or remove goes through [`World::conjure_mass`] and lands in the
//!   [`Ledger`], so "this room is not closed" is a number rather than a
//!   shrug. They are what keeps the demo alive after the bottle has
//!   emptied.
//! - The consequence worth watching: the CO₂ **sinks and pools on the
//!   floor**, in a layer, with no rule anywhere that names CO₂ or says
//!   "heavy gases sink". It sinks because at the room's pressure a gram of
//!   CO₂ takes less space than a gram of air, and the one density rule that
//!   drops sand through water drops CO₂ through air unchanged.
//!
//! [`Ledger`]: crate::world::Ledger

use crate::gas;
use crate::material::{terrarium as t, MaterialId, MaterialTable};
use crate::math::{GridIndex, Scalar};
use crate::physics;
use crate::world::World;

/// A cell that injects or removes a material, booking every gram.
///
/// A vent is not a physical object in the simulation — it is the statement
/// that this room sits inside a bigger world with something breathing into
/// it and something scrubbing it out. Same reasoning as
/// [`crate::terrarium::Thermostat`], and the same ledger.
#[derive(Debug, Clone, Copy)]
pub struct Vent {
    pub cell: GridIndex,
    pub material: MaterialId,
    /// Grams moved per step: positive injects, negative removes.
    pub rate: Scalar,
    /// The vent only fires while its cell's pressure is below this, so a
    /// source cannot pump a room up without limit. Ignored by a scrubber
    /// (negative `rate`), which only ever fires on its own material.
    pub max_pressure: Scalar,
}

impl Vent {
    pub fn source(cell: GridIndex, material: MaterialId, rate: Scalar) -> Self {
        Vent {
            cell,
            material,
            rate,
            max_pressure: 0.25,
        }
    }

    pub fn scrubber(cell: GridIndex, material: MaterialId, rate: Scalar) -> Self {
        Vent {
            cell,
            material,
            rate: -rate.abs(),
            max_pressure: Scalar::INFINITY,
        }
    }

    pub fn apply(&self, world: &mut World) {
        if !world.in_bounds(self.cell) {
            return;
        }
        let cell = world.cell(self.cell);
        let temperature = cell.temperature;
        if self.rate > 0.0 {
            if gas::pressure(world, self.cell) > self.max_pressure {
                return;
            }
            if cell.material == self.material {
                // Already this gas: just push more of it in, which is a
                // pressure rise rather than a material change.
                world.conjure_mass(self.cell, self.material, cell.mass + self.rate, temperature);
            } else {
                // Replace whatever was there; `conjure_mass` books the
                // displaced mass out as well as the new mass in.
                let density = world.materials().get(self.material).density;
                world.conjure_mass(self.cell, self.material, density, temperature);
            }
        } else if cell.material == self.material {
            world.conjure_mass(self.cell, self.material, self.rate, temperature);
        }
    }
}

/// The runnable gas demo: a world and the declared holes that keep it
/// interesting.
pub struct GasChamber {
    pub world: World,
    pub vents: Vec<Vent>,
    pub steps: u64,
}

impl GasChamber {
    pub fn step(&mut self, dt: Scalar) {
        for vent in &self.vents {
            vent.apply(&mut self.world);
        }
        physics::step(&mut self.world, dt);
        self.steps += 1;
    }
}

/// Builds the chamber at the given size. 48x32 is the tuned default.
pub fn gas_chamber(width: usize, height: usize) -> GasChamber {
    let (w, h) = (width as i32, height as i32);
    let mut world = World::new(width, height, MaterialTable::terrarium(), t::AIR, 291.0);
    let room_k = 291.0;

    // Stone shell.
    for i in 0..w {
        for j in 0..h {
            if i == 0 || j == 0 || i == w - 1 || j == h - 1 {
                world.fill(GridIndex::new(i, j), t::STONE, room_k);
            }
        }
    }

    // The bottle: a sealed box in the lower left with a single hole in its
    // right-hand wall, two rows up from the floor.
    let bottle_right = w / 4;
    let bottle_top = h / 3;
    for j in 1..=bottle_top {
        world.fill(GridIndex::new(bottle_right, j), t::STONE, room_k);
    }
    for i in 1..bottle_right {
        world.fill(GridIndex::new(i, bottle_top), t::STONE, room_k);
    }
    let hole = 3;
    world.fill(GridIndex::new(bottle_right, hole), t::AIR, room_k);

    // A shelf jutting into the room, so the CO2 layer has something to flow
    // around and the flow is visible rather than uniform.
    for i in w / 2..w - w / 6 {
        world.fill(GridIndex::new(i, h / 2), t::STONE, room_k);
    }

    world.rebaseline();

    // Ten atmospheres inside the bottle. Painted after the rebaseline would
    // register as a conservation violation, so it is painted before it and
    // the baseline is taken again below.
    for i in 1..bottle_right {
        for j in 1..bottle_top {
            let index = GridIndex::new(i, j);
            let mut cell = world.cell(index);
            cell.mass *= 10.0;
            world.set_cell(index, cell);
        }
    }
    // A slab of CO2 released up near the ceiling, on the right. Nothing
    // holds it there; the point of the demonstration is watching it fall
    // through the air and settle into a flat layer on the floor.
    for i in w / 2..w - 2 {
        for j in h - 5..h - 2 {
            world.fill(GridIndex::new(i, j), t::CO2, room_k);
        }
    }

    world.rebaseline();

    // The declared holes: CO2 in at the floor on the right, out at the
    // floor on the left. Both at floor level on purpose — a scrubber in the
    // ceiling never sees any CO2, because the CO2 is on the floor, which is
    // the whole point of the demonstration. Put it where the gas actually
    // goes and the room reaches a steady state with a visible current
    // running along the floor from one to the other.
    let vents = vec![
        Vent::source(GridIndex::new(w - 4, 1), t::CO2, 0.0004),
        Vent::scrubber(GridIndex::new(2, 1), t::CO2, 0.00015),
    ];

    GasChamber {
        world,
        vents,
        steps: 0,
    }
}

/// The size the browser view and the tests both use.
pub const DEFAULT_SIZE: (usize, usize) = (48, 32);

/// [`gas_chamber`] at [`DEFAULT_SIZE`].
pub fn default_chamber() -> GasChamber {
    gas_chamber(DEFAULT_SIZE.0, DEFAULT_SIZE.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(chamber: &mut GasChamber, steps: u32) {
        for _ in 0..steps {
            chamber.step(0.05);
        }
    }

    /// The bottle empties into the room: what starts at ten atmospheres
    /// behind one hole must bleed down through it.
    #[test]
    fn a_pressurised_bottle_empties_through_its_hole() {
        let mut c = default_chamber();
        let inside = GridIndex::new(2, 2);
        let outside = GridIndex::new(c.world.width() as i32 - 3, 6);
        let before_in = gas::pressure(&c.world, inside);
        let before_out = gas::pressure(&c.world, outside);
        assert!(
            before_in > before_out * 5.0,
            "the bottle should start pressurised: {before_in} vs {before_out}"
        );
        run(&mut c, 3000);
        let after_in = gas::pressure(&c.world, inside);
        assert!(
            after_in < before_in * 0.4,
            "the bottle should have vented: {before_in} -> {after_in}"
        );
        // And the air it let out is now at one pressure throughout,
        // bottle and room together — the bottle is not a separate world
        // with its own pressure any more.
        let (lo, hi) = gas::pressure_range_of(&c.world, t::AIR).unwrap();
        assert!(
            hi - lo < 0.2 * hi,
            "the air should share one pressure once vented, {lo} .. {hi}"
        );
    }

    /// The ONI complaint, in the flagship scenario: vented CO₂ ends up in a
    /// layer on the floor, under the air, on its own.
    #[test]
    fn vented_co2_pools_on_the_floor_under_the_air() {
        let mut c = default_chamber();
        let start = gas::mean_height_of(&c.world, t::CO2).expect("the slab is painted in");
        assert!(
            start > 20.0,
            "the CO2 starts up near the ceiling, at {start}"
        );
        run(&mut c, 1200);
        let co2 = gas::mean_height_of(&c.world, t::CO2).expect("the vent should have run");
        let air = gas::mean_height_of(&c.world, t::AIR).expect("there is always air");
        assert!(
            co2 < air,
            "CO2 should sit below the air: CO2 at {co2}, air at {air}"
        );
        assert!(
            co2 < 3.0,
            "CO2 should have settled into a layer on the floor, mean height \
             {start} -> {co2}"
        );
        // A layer, not a dune: it must have spread out across the floor
        // rather than piling up where it landed.
        let width = (0..c.world.width() as i32)
            .filter(|&i| c.world.material_at(GridIndex::new(i, 1)) == t::CO2)
            .count();
        assert!(
            width > c.world.width() / 2,
            "the layer should cover most of the floor, covered {width} cells"
        );
    }

    /// The standing invariant, with two declared holes open: the world's
    /// books balance against the ledger, not against zero.
    #[test]
    fn every_gram_the_vents_move_is_on_the_ledger() {
        let mut c = default_chamber();
        run(&mut c, 1200);
        let r = c.world.conservation_residuals();
        assert!(
            r.mass_relative.abs() < 1e-5,
            "mass residual {:e} (absolute {})",
            r.mass_relative,
            r.mass
        );
        assert!(
            r.energy_relative.abs() < 1e-5,
            "energy residual {:e}",
            r.energy_relative
        );
        assert!(
            c.world.ledger().mass_conjured.abs() > 0.0,
            "the vents should have moved real mass, and said so"
        );
    }

    /// And it has to settle rather than shimmer: after the transient, the
    /// air in the room is at one pressure.
    ///
    /// Air only, and deliberately. Two different gases cannot share a cell
    /// in this model, so the CO₂ layer and the air above it cannot equalise
    /// with each other the way partial pressures really would — see
    /// `src/gas.rs`'s note on that limitation. Asserting a single pressure
    /// across both species would be asserting something this simulation
    /// does not claim.
    #[test]
    fn the_room_reaches_a_nearly_uniform_pressure() {
        let mut c = default_chamber();
        let (lo0, hi0) = gas::pressure_range_of(&c.world, t::AIR).unwrap();
        run(&mut c, 3000);
        let (lo, hi) = gas::pressure_range_of(&c.world, t::AIR).unwrap();
        assert!(
            hi - lo < (hi0 - lo0) * 0.1,
            "pressure spread should collapse: {} -> {}",
            hi0 - lo0,
            hi - lo
        );
    }
}
