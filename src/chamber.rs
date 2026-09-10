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
//! - A **CO₂ vent** and a **scrubber**, both on the floor and both
//!   declared holes in the world's books, in exactly the shape
//!   `src/terrarium.rs` already uses for its thermostats: every gram they
//!   add or remove goes through [`World::conjure_mass`] and lands in the
//!   [`Ledger`], so "this room is not closed" is a number rather than a
//!   shrug. They are what keeps the demo alive after the bottle has
//!   emptied.
//! - The consequence worth watching: a slab of CO₂ released at the ceiling
//!   **sinks**, with no rule anywhere that names CO₂ or says "heavy gases
//!   sink" — at the room's pressure a gram of CO₂ takes less space than a
//!   gram of air, and the one density rule that drops sand through water
//!   drops a CO₂-rich parcel through air unchanged — and then **it does not
//!   stay a layer**. It mixes upward into the air above it, and the room
//!   settles with more CO₂ near the floor than at the ceiling but plenty at
//!   the ceiling too.
//!
//! That second half is new, and it reverses what this page used to show.
//! Night 2 built it to demonstrate CO₂ settling into a flat layer on the
//! floor with clean air above — read from `NORTH_STARS.md` #4's "CO2
//! doesn't actually settle the way ONI's simplified layers show it". The
//! dictation behind that line says the opposite of what the layer showed:
//! *"in a typical room you'd have a layer of CO2 at the bottom and O2 on top
//! of that ... surely we can do better than that — gas is mix. CO2 is
//! heavier but it doesn't all fall to the bottom of a room."* The layer was
//! ONI's behaviour, reproduced faithfully. Gases that can share a cell are
//! what it took to do better (`src/gas.rs`).
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

    /// Breathes `rate` grams into its cell's air, or scrubs them out.
    ///
    /// Both are now a change to one species of the cell's mixture rather
    /// than a swap of the whole cell, which is simpler and more honest than
    /// what it replaced: a source used to *replace* whatever gas was in its
    /// cell with a full cell of its own, and a scrubber only fired on cells
    /// that happened to be pure CO₂. A real scrubber takes CO₂ out of
    /// whatever air reaches it.
    pub fn apply(&self, world: &mut World) {
        if !world.in_bounds(self.cell) {
            return;
        }
        let temperature = world.cell(self.cell).temperature;
        if self.rate > 0.0 && gas::pressure(world, self.cell) > self.max_pressure {
            return;
        }
        world.conjure_mass(self.cell, self.material, self.rate, temperature);
    }
}

/// A relief valve: a cell that lets gas out of the world whenever its
/// pressure is above `max_pressure`, taking a parcel of whatever mixture is
/// there and booking every gram and joule of it.
///
/// This is the boundary a sealed box needs as soon as something inside it
/// makes gas faster than anything consumes it. A still is the example: it
/// boils wash into vapour, and the air that was in the pot and condenser
/// when it was built has nowhere to go and cannot condense, so a sealed
/// still pressurises, its boiling points drift (the fixed ones in the
/// material table are quoted at one atmosphere) and it stops working. A real
/// still is open to the room at the far end of its condenser for exactly
/// this reason, and so is this one.
#[derive(Debug, Clone, Copy)]
pub struct Relief {
    pub cell: GridIndex,
    pub max_pressure: Scalar,
}

impl Relief {
    pub fn new(cell: GridIndex, max_pressure: Scalar) -> Self {
        Relief { cell, max_pressure }
    }

    /// Lets out just enough of the cell's contents — every species in the
    /// same proportion — to bring it back down to `max_pressure`.
    pub fn apply(&self, world: &mut World) {
        if !world.in_bounds(self.cell) {
            return;
        }
        let pressure = gas::pressure(world, self.cell);
        if pressure <= self.max_pressure || pressure <= 0.0 {
            return;
        }
        let fraction = (pressure - self.max_pressure) / pressure;
        let cell = world.cell(self.cell);
        let species: Vec<MaterialId> = world.materials().slots().to_vec();
        for (s, id) in species.into_iter().enumerate() {
            let grams = cell.mix[s] * fraction;
            if grams > 0.0 {
                world.conjure_mass(self.cell, id, -grams, cell.temperature);
            }
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
            for m in cell.mix.iter_mut() {
                *m *= 10.0;
            }
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
    // floor on the left, at the same rate, so the room reaches a steady
    // state rather than filling up. Both at floor level because that is
    // where the CO2 is richest. (The scrubber used to sit inside the sealed
    // bottle, where almost no CO2 ever reached it — invisible while it had
    // to wait for a whole cell of pure CO2 to fire on at all.)
    let vents = vec![
        Vent::source(GridIndex::new(w - 4, 1), t::CO2, 0.0002),
        Vent::scrubber(GridIndex::new(w / 4 + 2, 1), t::CO2, 0.0002),
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
    /// behind one hole must bleed down through it, and bottle and room end
    /// up at one pressure.
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
        // Every gas cell — air and CO2 alike, since they now share cells —
        // at one pressure, give or take the vent's own small jet.
        let (lo, hi) = gas::pressure_range(&c.world).unwrap();
        assert!(
            hi - lo < 0.05 * hi,
            "the room should share one pressure once vented, {lo} .. {hi}"
        );
    }

    /// The dictation's claim, in the flagship gas scenario: the CO2 slab
    /// sinks because it is heavier, and then it does not lie on the floor as
    /// a layer — the room ends up leaning toward the floor with CO2 all the
    /// way up.
    #[test]
    fn released_co2_sinks_and_then_mixes_instead_of_layering() {
        let mut c = default_chamber();
        let start = gas::mean_height_of(&c.world, t::CO2).expect("the slab is painted in");
        assert!(
            start > 20.0,
            "the CO2 starts up near the ceiling, at {start}"
        );
        run(&mut c, 600);
        let sunk = gas::mean_height_of(&c.world, t::CO2).expect("co2");
        let air = gas::mean_height_of(&c.world, t::AIR).expect("air");
        assert!(
            sunk < start / 2.0 && sunk < air,
            "CO2 is heavier than air and should have sunk below it: CO2 {start} -> {sunk}, \
             air at {air}"
        );
        run(&mut c, 3400);
        let profile = gas::fraction_profile(&c.world, t::CO2);
        let floor = profile[1].unwrap();
        let ceiling = profile[c.world.height() - 2].unwrap();
        assert!(
            floor > 2.0 * ceiling,
            "it should still lean toward the floor: floor {floor}, ceiling {ceiling}"
        );
        assert!(
            ceiling > 0.1 * floor,
            "but not as a layer — the ceiling should carry CO2 too: floor {floor}, \
             ceiling {ceiling}"
        );
        let later = gas::mean_height_of(&c.world, t::CO2).expect("co2");
        assert!(
            later > sunk,
            "having sunk, it should have mixed back upward: {sunk} -> {later}"
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
            r.mass_relative.abs() < 1e-9,
            "mass residual {:e} (absolute {})",
            r.mass_relative,
            r.mass
        );
        assert!(
            r.energy_relative.abs() < 1e-9,
            "energy residual {:e}",
            r.energy_relative
        );
        assert!(
            c.world.ledger().mass_conjured.abs() > 0.0,
            "the vents should have moved real mass, and said so"
        );
    }

    /// And it has to settle rather than shimmer: after the transient, the
    /// whole room is at one pressure.
    #[test]
    fn the_room_reaches_a_nearly_uniform_pressure() {
        let mut c = default_chamber();
        let (lo0, hi0) = gas::pressure_range(&c.world).unwrap();
        run(&mut c, 3000);
        let (lo, hi) = gas::pressure_range(&c.world).unwrap();
        assert!(
            hi - lo < (hi0 - lo0) * 0.02,
            "pressure spread should collapse: {} -> {}",
            hi0 - lo0,
            hi - lo
        );
    }
}
