//! The still: juniper and water in a heated pot on the left, a cold
//! condenser on the right, and gin dripping out of the middle of it.
//!
//! `NORTH_STARS.md` #4 names brewing and distilling as *core to the whole
//! thing working, not flavour* — Gin is the mana that pays for every
//! sanctioned exception to conservation, and up to tonight it came out of
//! raw berries because there was no chemistry to make anything else with.
//! This scenario is the answer, and the claim it exists to make is that
//! nothing in it is scripted:
//!
//! - **Mashing** is one row of the reaction table (`src/chemistry.rs`):
//!   juniper touching water, warm enough, becomes wash. It is limited by
//!   the botanicals in the pot, which is what makes gin scarce.
//! - **Distilling** is not a special mechanic at all. Wash and water each
//!   evaporate along a vapour-pressure curve read off their boiling points
//!   (`src/vapour.rs`) — 351.5 K for the spirit, 373.15 K for water — so
//!   at the pot's 366 K the air over it holds several times as much spirit
//!   as steam, and wash below the surface boils outright. That 22 K gap is
//!   the entire trick, and it is real physical numbers in the material
//!   table, not a rule. It is not a perfect separation, and does not
//!   pretend to be: some water evaporates too and ends up in the receiver,
//!   as it does from a real pot still.
//! - **The lyne arm** is a gap in the pot wall just above the liquid, and
//!   the vapour finds it without being told to, because ethanol vapour is
//!   *denser than air* (46 g/mol against 29) and therefore crawls sideways
//!   along the top of the wash rather than rising. That, too, is one number
//!   — `8.314/46.07` — in `Material::gas_constant`.
//! - **Condensing** is the same curve run the other way: against the cold
//!   floor and wall the vapour is past saturation, forms dew, and falls as
//!   drops of gin that pool where they land.
//!
//! ## The energy bill is a number, not a shrug
//!
//! A gram of spirit costs 846 J to boil off and gives it back on
//! condensing, so a still is a heat pump with a hole in the middle: the hob
//! puts energy in, the condenser takes it out, and if either stops, the
//! process stops. Both are [`Thermostat`]s, which means every joule of it
//! goes through [`World::conjure_energy`] and lands in the [`Ledger`] — the
//! same declared hole the terrarium's vent uses and the same one gnome
//! magic uses. So "how much did this still cost to run" is a figure anyone
//! can read off the report, and the world's conservation residuals stay at
//! floating-point noise the whole time.
//!
//! [`Ledger`]: crate::world::Ledger
//! [`World::conjure_energy`]: crate::world::World::conjure_energy

use crate::chamber::Relief;
use crate::gnome::{Colony, Gnome};
use crate::material::{terrarium as t, MaterialTable, Phase};
use crate::math::{GridIndex, Scalar};
use crate::physics;
use crate::terrarium::Thermostat;
use crate::world::World;

/// The temperature the hob holds the pot floor at.
///
/// The single most load-bearing constant in the scenario, and it is chosen
/// to sit *between* two numbers taken from real chemistry: above ethanol's
/// 351.5 K boiling point so the wash boils, below water's 373.15 K so the
/// water beside it does not. Push it past 373 and the still starts
/// producing steam alongside the spirit, which condenses as water in the
/// receiver and dilutes the run — the same mistake a real distiller makes,
/// for the same reason, with no code anywhere modelling "dilution".
pub const HOB_K: Scalar = 366.0;
/// The temperature the condenser floor and wall are held at.
pub const CONDENSER_K: Scalar = 288.0;
/// Where the pot's wall stands.
const POT_WALL: i32 = 22;
/// The rows of the gap in that wall — the lyne arm, inclusive.
///
/// It starts at the liquid's surface, not above it: spirit vapour is
/// *denser than air*, so it lies on top of the wash rather than rising off
/// it, and a lyne arm set too high was a still that never yielded anything.
///
/// It is four rows rather than the two it began as, and that is a lesson
/// about vapour, not a tuning knob. A gram of spirit boiled off the wash is
/// several times the volume of the whole head space above it at one
/// atmosphere; all of it has to leave through this gap as fast as the hob
/// makes it, and gas in this grid moves by levelling pressure between
/// neighbours, which through a two-cell gap is slow. With gases that could
/// not share a cell, whole-cell swaps hid that. Once they could, a two-row
/// lyne arm pressurised the pot to several atmospheres, its head space went
/// to the boiling point of spirit *at that pressure* — hotter than the hob
/// — and the vapour condensed straight back onto the cooler wash.
const LYNE: (i32, i32) = (6, 9);
/// Depth of liquid in the pot, in rows.
const CHARGE_ROWS: i32 = 5;
/// The temperature the pot is charged at — already past mashing
/// temperature, so fermentation starts on the first step.
const CHARGE_K: Scalar = 320.0;
/// How often the botanist's hatch drops a fresh bush into the pot.
///
/// Without it the scenario is a single batch: the ten bushes it is charged
/// with ferment on the first step, run through the still over a couple of
/// thousand steps, and then it is a warm pot of water with two full gnomes
/// beside it forever. A hatch makes it a *process*, which is the thing
/// actually worth showing — and it is a declared hole in the books like
/// every other one here, so the conservation figures still balance.
const HOPPER_PERIOD: u64 = 600;

/// A runnable still: the world, the gnomes waiting on the receiver, and the
/// two declared thermal boundaries that drive the whole thing.
pub struct Still {
    pub world: World,
    pub colony: Colony,
    pub thermostats: Vec<Thermostat>,
    /// The condenser's open end — see [`Relief`].
    pub relief: Relief,
    pub steps: u64,
}

impl Still {
    /// One tick: boundary conditions, then physics (which now includes
    /// chemistry), then the gnomes.
    pub fn step(&mut self, dt: Scalar) {
        for th in &self.thermostats {
            th.apply(&mut self.world);
        }
        self.relief.apply(&mut self.world);
        if self.steps.is_multiple_of(HOPPER_PERIOD) {
            self.charge_botanicals();
        }
        physics::step(&mut self.world, dt);
        self.colony.update(&mut self.world);
        self.steps += 1;
    }

    /// Drops one bush into the pot, moving along the floor so successive
    /// charges do not land on top of each other.
    ///
    /// It replaces a cell of whatever liquid is standing there, and
    /// [`World::conjure_mass`] books both sides of that swap — so "this pot
    /// is being fed from outside" is a figure in the ledger rather than an
    /// unexplained appearance of matter, exactly like the terrarium's vent
    /// and the gas chamber's scrubber.
    ///
    /// [`World::conjure_mass`]: crate::world::World::conjure_mass
    fn charge_botanicals(&mut self) {
        let span = (POT_WALL - 3).max(1);
        let n = (self.steps / HOPPER_PERIOD) as i32;
        let at = GridIndex::new(2 + (n * 5).rem_euclid(span), 1);
        if !self.world.in_bounds(at) {
            return;
        }
        if self.world.materials().get(self.world.material_at(at)).phase != Phase::Liquid {
            return;
        }
        let temperature = self.world.cell(at).temperature;
        let bush = self.world.materials().get(t::JUNIPER).density;
        self.world.conjure_mass(at, t::JUNIPER, bush, temperature);
    }

    /// Grams of gin standing in the world right now — the run's yield, and
    /// the number the headless check and the browser page both read.
    pub fn gin_mass(&self) -> f64 {
        self.world.mass_of(t::GIN)
    }
}

/// Builds the still at the given size. 52x30 is the tuned default.
pub fn still(width: usize, height: usize) -> Still {
    let (w, h) = (width as i32, height as i32);
    let cold = 291.0;
    let mut world = World::new_open(width, height, MaterialTable::terrarium(), cold);

    // Shell.
    for i in 0..w {
        for j in 0..h {
            if i == 0 || j == 0 || i == w - 1 || j == h - 1 {
                world.fill(GridIndex::new(i, j), t::STONE, cold);
            }
        }
    }

    // The pot wall, with the lyne arm cut through it.
    for j in 1..h - 1 {
        if (LYNE.0..=LYNE.1).contains(&j) {
            continue;
        }
        world.fill(GridIndex::new(POT_WALL, j), t::STONE, cold);
    }

    // The charge: water, with juniper standing in it. Every other column,
    // so each bush has water on both sides and the mash spreads through the
    // pot rather than sitting in a clump.
    //
    // Charged warm, not cold, and for the same reason a brewer does it:
    // the tun has to be past mashing temperature before anything ferments,
    // and starting it at room temperature means several thousand steps of
    // watching a pot heat up before the interesting part begins.
    for i in 1..POT_WALL {
        for j in 1..1 + CHARGE_ROWS {
            world.fill(GridIndex::new(i, j), t::WATER, CHARGE_K);
        }
        // ...and the lagged pot's head space starts at the hob's
        // temperature, not the room's. Now that wash evaporates below its
        // boiling point, a cold head space is a condenser: the first thing
        // a still charged into cold air does is distil into its own lid and
        // rain the gin back into the pot, which is a real still being run
        // badly rather than a still.
        for j in 1 + CHARGE_ROWS..h - 1 {
            world.fill(GridIndex::new(i, j), t::AIR, HOB_K);
        }
    }
    for i in (2..POT_WALL - 1).step_by(4) {
        world.fill(GridIndex::new(i, 1), t::JUNIPER, CHARGE_K);
    }

    world.rebaseline();

    // The hob under the pot, and the cold floor and wall of the condenser.
    // Both are declared holes in the world's energy books — see the module
    // doc comment.
    let mut thermostats = Vec::new();
    for i in 1..POT_WALL {
        thermostats.push(Thermostat::new(GridIndex::new(i, 0), HOB_K));
        // The pot is lagged, not just heated from below. Without it the
        // vapour that fills the pot's head space touches a cold ceiling,
        // condenses there and rains straight back into the wash — real
        // reflux, and measured at three times as much gin ending up in the
        // pot as in the receiver. A still keeps its column hot for exactly
        // this reason; the only cold thing in the room should be the
        // condenser.
        thermostats.push(Thermostat::new(GridIndex::new(i, h - 1), HOB_K));
    }
    for j in 1..h - 1 {
        thermostats.push(Thermostat::new(GridIndex::new(0, j), HOB_K));
    }
    for i in POT_WALL + 1..w - 1 {
        thermostats.push(Thermostat::new(GridIndex::new(i, 0), CONDENSER_K));
        thermostats.push(Thermostat::new(GridIndex::new(i, h - 1), CONDENSER_K));
    }
    for j in 1..h - 1 {
        thermostats.push(Thermostat::new(GridIndex::new(w - 1, j), CONDENSER_K));
    }

    // Two gnomes on the receiver's floor, and deliberately short of Gin:
    // a gnome with a full flask never goes looking for a drink, so a still
    // nobody is thirsty enough to visit demonstrates only half of what it
    // is for.
    let gnomes = vec![
        Gnome::new(GridIndex::new(w - 8, 2)).with_gin(18.0),
        Gnome::new(GridIndex::new(w - 4, 2)).with_gin(12.0),
    ];

    // The far top corner of the condenser is open to the room, at one
    // atmosphere — see `Relief` for why a still cannot be a sealed box.
    let relief = Relief::new(
        GridIndex::new(w - 2, h - 2),
        world.materials().reference_pressure(),
    );

    Still {
        world,
        colony: Colony::new(gnomes),
        thermostats,
        relief,
        steps: 0,
    }
}

/// The size the browser view and the headless runner both use.
pub const DEFAULT_SIZE: (usize, usize) = (52, 30);

/// [`still`] at [`DEFAULT_SIZE`].
pub fn default_still() -> Still {
    still(DEFAULT_SIZE.0, DEFAULT_SIZE.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(steps: u64) -> Still {
        let mut s = default_still();
        for _ in 0..steps {
            s.step(0.05);
        }
        s
    }

    #[test]
    fn the_pot_ferments_its_botanicals_into_wash() {
        let s = run(400);
        assert!(
            s.world.count_of(t::WASH) > 0,
            "nothing fermented:\n{}",
            crate::report::ascii_map(&s.world)
        );
        assert!(
            s.world.count_of(t::JUNIPER) < 10,
            "the botanicals were not consumed"
        );
    }

    #[test]
    fn the_still_yields_gin_and_the_water_stays_behind() {
        let s = run(3000);
        assert!(
            s.gin_mass() > 0.05,
            "the still yielded {} g of gin:\n{}",
            s.gin_mass(),
            crate::report::ascii_map(&s.world)
        );
        // The point of holding the pot below 373 K: the water charge is
        // still water, not steam in the receiver.
        assert!(
            s.world.mass_of(t::WATER) > 5.0,
            "the water boiled off too — the hob is above 373 K somewhere"
        );
    }

    /// Where the gin ends up is the claim a picture would make, so it is
    /// worth making in numbers: most of it on the *condenser* side of the
    /// wall, and still arriving there while the pot's share stays put.
    ///
    /// Not all of it, and the test used to say "four times as much". The pot
    /// keeps some from its opening burst, when the whole charge of wash
    /// comes to the boil together. Boiling in this grid turns one cell of
    /// liquid into one cell of vapour at hundreds of atmospheres, and the
    /// cloud that leaves behind as it bursts through the surface is briefly
    /// at several atmospheres over the pot — enough to condense onto the
    /// wash. It is the one-cell-bubble artifact, not a still being run
    /// badly, and it stops once the still settles down to what the
    /// botanist's hatch feeds it: from then on, what the still makes goes
    /// to the receiver.

    #[test]
    fn the_gin_collects_on_the_cold_side_of_the_wall() {
        let split = |s: &Still| {
            let (w, h) = (s.world.width(), s.world.height());
            let mut left = 0.0f64;
            let mut right = 0.0f64;
            for j in 0..h as i32 {
                for i in 0..w as i32 {
                    let c = s.world.cell(GridIndex::new(i, j));
                    if c.material != t::GIN {
                        continue;
                    }
                    if i < POT_WALL {
                        left += c.mass;
                    } else {
                        right += c.mass;
                    }
                }
            }
            (left, right)
        };
        let mut s = run(1500);
        let (left_mid, right_mid) = split(&s);
        // Two thousand more, not fifteen hundred: the pot's share plateaus
        // near four and a half grams while the receiver climbs steadily, so
        // the *ratio* this asserts is a question of how long you watch. It
        // crosses 1.5 a few hundred steps after 3000, and gnome respiration
        // — new since night 5, and a real effect in a box this small —
        // moved it far enough to matter.
        for _ in 0..2000 {
            s.step(0.05);
        }
        let (left, right) = split(&s);
        assert!(
            right > 1.5 * left && right > 1.0,
            "gin split {left} g in the pot against {right} g in the receiver"
        );
        assert!(
            right - right_mid > 2.0 * (left - left_mid).max(0.0) && right > right_mid + 0.5,
            "the receiver should keep gaining while the pot does not: pot {left_mid} -> \
             {left}, receiver {right_mid} -> {right}"
        );
    }

    #[test]
    fn a_thirsty_gnome_drinks_what_the_still_made() {
        let s = run(3000);
        let drank = s
            .colony
            .gnomes
            .iter()
            .any(|g| g.gin > 40.0 || g.last_act == crate::gnome::Act::Drank);
        assert!(
            drank,
            "no gnome ever got a drink; flasks at {:?}, gin in the world {} g",
            s.colony.gnomes.iter().map(|g| g.gin).collect::<Vec<_>>(),
            s.gin_mass()
        );
    }

    #[test]
    fn running_a_still_conserves_mass_and_energy_against_the_ledger() {
        let s = run(3000);
        let r = s.world.conservation_residuals();
        assert!(
            r.mass_relative.abs() < 1e-6,
            "mass residual {:e}",
            r.mass_relative
        );
        assert!(
            r.energy_relative.abs() < 1e-5,
            "energy residual {:e}",
            r.energy_relative
        );
        // ...and the hob's bill is a real, readable number rather than a
        // free lunch.
        assert!(
            s.world.ledger().energy_conjured.abs() > 0.0,
            "the boundary should have moved real energy, and said so"
        );
    }
}
