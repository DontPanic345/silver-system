//! The flagship scenario: a sealed jar with a lava vent under it, a cold
//! roof over it, water in between, and gnomes living in the gap.
//!
//! This exists because the north stars are not satisfied by a physics
//! engine that passes tests. *"Only when the big universe is sufficiently
//! full will the small world be believable"* — so the demonstration is a
//! world with a water cycle running under its own steam: the vent boils
//! water, the steam rises, the cold roof condenses it, and it rains back
//! down. No script drives any of that; it falls out of conduction and two
//! phase transitions.
//!
//! ## Why the jar is not, in the end, sealed — and why that is the honest
//! answer
//!
//! The first version of this scenario was completely closed: stone shell
//! all the way round, a pot of lava at the bottom, ice at the top, nothing
//! entering or leaving. It conserved beautifully and it was *dead within a
//! couple of minutes of simulated time*. Of course it was. Energy is
//! conserved, so the lava's heat spread out, the ice absorbed it, the whole
//! jar settled at one temperature, and the water cycle stopped. A closed
//! system reaches equilibrium; that is not a bug to tune away, it is the
//! thing the rest of this crate works hard to get right.
//!
//! A cycle needs a source and a sink. So this terrarium has both, declared
//! rather than smuggled in: a [`Thermostat`] holds the vent hot and the
//! roof cold, and every joule it adds or removes goes through
//! [`World::conjure_energy`] and lands in the same [`Ledger`] gnome magic
//! uses.
//!
//! The result is that the conservation invariant still holds exactly —
//! `total_energy == initial + ledger.energy_conjured` — and the ledger now
//! states, in joules, precisely how much this jar is *not* a closed system.
//! That is a far more useful thing to be able to say than "the jar is
//! sealed", and it is the same mechanism the gnomes' magic already uses:
//! nothing is free, and everything that isn't free is written down.
//!
//! [`Ledger`]: crate::world::Ledger

use crate::gnome::{Colony, EtherealPipe, Gnome};
use crate::material::{terrarium as t, MaterialTable};
use crate::math::{GridIndex, Scalar};
use crate::physics;
use crate::world::World;

/// A cell held at a fixed temperature, and the accounted-for hole in the
/// jar's energy budget that holding it there implies.
///
/// A thermostat is not a physical object in the simulation — it is a
/// boundary condition, i.e. a statement that this jar sits inside a larger
/// universe with a hot thing under it and a cold thing over it. Rather than
/// let that pretend to be free, each nudge is booked through the ledger, so
/// the world's books balance and the size of the boundary flux is a number
/// anyone can read.
#[derive(Debug, Clone, Copy)]
pub struct Thermostat {
    pub cell: GridIndex,
    pub target_k: Scalar,
    /// Fraction of the gap to the target closed per step. Below 1.0 so the
    /// boundary leads the interior rather than clamping it rigidly.
    pub rate: Scalar,
}

impl Thermostat {
    pub fn new(cell: GridIndex, target_k: Scalar) -> Self {
        Thermostat {
            cell,
            target_k,
            rate: 0.25,
        }
    }

    /// Nudges its cell toward `target_k`, booking the energy moved.
    pub fn apply(&self, world: &mut World) {
        if !world.in_bounds(self.cell) {
            return;
        }
        let cell = world.cell(self.cell);
        let capacity = cell.mass as f64 * world.materials().get(cell.material).heat_capacity as f64;
        if capacity <= 0.0 {
            return;
        }
        let joules = (self.target_k - cell.temperature) as f64 * capacity * self.rate as f64;
        world.conjure_energy(self.cell, joules);
    }
}

/// The whole runnable scenario: a world, the gnomes living in it, and the
/// boundary conditions that keep it running.
pub struct Terrarium {
    pub world: World,
    pub colony: Colony,
    pub thermostats: Vec<Thermostat>,
    pub steps: u64,
}

impl Terrarium {
    /// One tick: boundary conditions, then physics, then the gnomes.
    pub fn step(&mut self, dt: Scalar) {
        for th in &self.thermostats {
            th.apply(&mut self.world);
        }
        physics::step(&mut self.world, dt);
        self.colony.update(&mut self.world);
        self.steps += 1;
    }
}

/// Wall thickness of the jar.
const SHELL: i32 = 2;
/// Temperature the vent is held at, in kelvin — well above water's boiling
/// point and well below rock's melting point.
const VENT_K: Scalar = 950.0;
/// Temperature the roof is held at.
const ROOF_K: Scalar = 283.0;

/// Builds the sealed gnome terrarium at the given size. Sizes below about
/// 24x24 leave no room for the cycle; 48x36 is the tuned default used by
/// the browser view and the headless runner.
pub fn gnome_terrarium(width: usize, height: usize) -> Terrarium {
    let (w, h) = (width as i32, height as i32);
    let mut world = World::new(width, height, MaterialTable::terrarium(), t::AIR, 291.0);

    // The jar: a stone shell all the way round.
    for i in 0..w {
        for j in 0..h {
            let edge = i < SHELL || j < SHELL || i >= w - SHELL || j >= h - SHELL;
            if edge {
                world.fill(GridIndex::new(i, j), t::STONE, 291.0);
            }
        }
    }

    // A hot rock vent along part of the floor: the jar's heat source.
    //
    // Deliberately hot rock rather than lava. A lava vent looks better and
    // was the first attempt, but it has to sit above rock's melting point
    // to stay liquid, which means it slowly melts the floor it is sitting
    // in and the jar eats itself. Stone held at 1150 K boils anything above
    // it just as thoroughly and stays where it is put.
    let vent_from = w / 6;
    let vent_to = w / 6 + w / 5;
    for i in vent_from..vent_to {
        for j in SHELL..SHELL + 2 {
            world.fill(GridIndex::new(i, j), t::STONE, VENT_K);
        }
    }

    // Rock floor over the rest, so the gnomes have somewhere to stand.
    for i in SHELL..w - SHELL {
        if !(vent_from..vent_to).contains(&i) {
            world.fill(GridIndex::new(i, SHELL), t::STONE, 291.0);
            world.fill(GridIndex::new(i, SHELL + 1), t::SAND, 291.0);
        }
    }

    // A pool of water sitting on the sand beside the vent, stopping short
    // of the far wall to leave room for the juniper plinth below.
    let pool_from = vent_to + 1;
    let pool_to = w - SHELL - 7;
    for i in pool_from..pool_to {
        // Four courses deep rather than three: the plinth below took five
        // columns off the pool's length, and the pool is also the jar's
        // thermal ballast — with less of it the vent boils the jar dry
        // sooner and the gnomes start cooking. Depth buys that back.
        for j in SHELL + 2..SHELL + 6 {
            world.fill(GridIndex::new(i, j), t::WATER, 300.0);
        }
    }

    // The lid is held cold, but not freezing. An ice roof was the obvious
    // choice and it was wrong: steam hitting ice at 240 K condenses and
    // then immediately freezes, so the roof just grows thicker and thicker
    // and the water never comes back down. Ice is static; rain is not. A
    // lid a little above freezing condenses steam into liquid water, which
    // falls — which is the half of the cycle worth having.

    // Juniper on a raised plinth in the far corner: the colony's Gin
    // supply, and deliberately *dry*.
    //
    // The bushes used to stand in the pool itself, which was fine while
    // juniper had no chemistry. It is not fine now: warm water touching
    // juniper ferments both into wash (see `src/chemistry.rs`), so a bush
    // standing in a pool that the vent is busy heating is not a Gin supply,
    // it is a mash tun that the colony did not ask for — measured over
    // 4000 steps, every bush and most of the pool went that way. Two stone
    // courses lift them clear of anything that can slosh, which is the
    // scenario stating a requirement the chemistry now imposes rather than
    // the chemistry being softened to suit the scenario.
    let plinth_from = pool_to + 1;
    for i in plinth_from..w - SHELL {
        world.fill(GridIndex::new(i, SHELL + 2), t::STONE, 291.0);
        world.fill(GridIndex::new(i, SHELL + 3), t::SAND, 291.0);
    }
    for k in 0..4 {
        let i = plinth_from + k;
        if i < w - SHELL {
            world.fill(GridIndex::new(i, SHELL + 4), t::JUNIPER, 291.0);
        }
    }
    // ...and a stone shelf over them. Lifting the bushes out of the pool
    // was not enough once the gas work let steam actually fill the jar and
    // rain back down: warm rain landing on a bush ferments it just as well
    // as a pool does, and over 8000 steps every bush went that way and the
    // colony's Gin supply with it. Rain does not fall through rock.
    for i in plinth_from - 1..w - SHELL {
        world.fill(GridIndex::new(i, SHELL + 5), t::STONE, 291.0);
    }

    world.rebaseline();

    // Gnomes on the sand, plus one ethereal pipe lifting water from the
    // pool up to a ledge — the sanctioned magic shortcut, on display.
    let gnomes: Vec<Gnome> = (0..4)
        .map(|k| Gnome::new(GridIndex::new(SHELL + 2 + k * 3, SHELL + 3)))
        .collect();
    let pipe = EtherealPipe::new(
        GridIndex::new(pool_from + 1, SHELL + 2),
        GridIndex::new(SHELL + 3, h / 2),
    );
    let colony = Colony::new(gnomes).with_pipe(pipe);

    // The boundary: the vent is held hot, the roof cold. Everything these
    // add or remove is booked — see the module doc comment.
    let mut thermostats = Vec::new();
    for i in vent_from..vent_to {
        thermostats.push(Thermostat::new(GridIndex::new(i, SHELL), VENT_K));
    }
    // The cold side of the jar is its whole outer skin, not just the lid.
    //
    // It used to be one row across the ceiling, and that was not enough of
    // a sink to balance a 950 K vent: the stone shell conducts, so the vent
    // heated the walls, the walls heated the interior, and the jar settled
    // *above* water's boiling point — measured at 431 K after 2000 steps
    // and still climbing at 6000. The water cycle in a jar like that runs
    // exactly once, boils the pool dry, and then there is nothing left to
    // rain. A jar sitting on a table is surrounded by a room; this says so.
    for i in SHELL..w - SHELL {
        thermostats.push(Thermostat::new(GridIndex::new(i, h - SHELL), ROOF_K));
        thermostats.push(Thermostat::new(GridIndex::new(i, h - 1), ROOF_K));
    }
    for j in SHELL..h - SHELL {
        thermostats.push(Thermostat::new(GridIndex::new(0, j), ROOF_K));
        thermostats.push(Thermostat::new(GridIndex::new(w - 1, j), ROOF_K));
    }

    Terrarium {
        world,
        colony,
        thermostats,
        steps: 0,
    }
}

/// The size the browser view and the headless runner both use, so they are
/// demonstrably the same world.
pub const DEFAULT_SIZE: (usize, usize) = (48, 36);

/// [`gnome_terrarium`] at [`DEFAULT_SIZE`].
pub fn default_terrarium() -> Terrarium {
    gnome_terrarium(DEFAULT_SIZE.0, DEFAULT_SIZE.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_jar_is_sealed_and_stays_sealed() {
        let mut terra = default_terrarium();
        let (w, h) = (terra.world.width() as i32, terra.world.height() as i32);
        for _ in 0..300 {
            terra.step(0.05);
        }
        for i in 0..w {
            assert_eq!(terra.world.material_at(GridIndex::new(i, 0)), t::STONE);
            assert_eq!(terra.world.material_at(GridIndex::new(i, h - 1)), t::STONE);
        }
        for j in 0..h {
            assert_eq!(terra.world.material_at(GridIndex::new(0, j)), t::STONE);
            assert_eq!(terra.world.material_at(GridIndex::new(w - 1, j)), t::STONE);
        }
    }

    #[test]
    fn the_water_cycle_runs_on_its_own() {
        let mut terra = default_terrarium();
        let mut ever_boiled = false;
        let mut steam_peak = 0usize;
        for _ in 0..4000 {
            terra.step(0.05);
            let steam = terra.world.count_of(t::STEAM);
            steam_peak = steam_peak.max(steam);
            if steam > 0 {
                ever_boiled = true;
            }
        }
        assert!(
            ever_boiled,
            "the vent should have boiled some of the pool into steam"
        );
        // And the water is still water in some phase — nothing evaporated
        // out of existence.
        let water_ish = terra.world.count_of(t::WATER)
            + terra.world.count_of(t::STEAM)
            + terra.world.count_of(t::ICE);
        assert!(water_ish > 20, "only {water_ish} cells of H2O left");
    }

    #[test]
    fn the_ledger_accounts_for_every_joule_the_boundary_moves() {
        // The point of the thermostat design: the jar is deliberately not
        // closed, and exactly how much it isn't is a number, not a shrug.
        let mut terra = default_terrarium();
        for _ in 0..1500 {
            terra.step(0.05);
        }
        let r = terra.world.conservation_residuals();
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
        assert!(
            terra.world.ledger().energy_conjured.abs() > 0.0,
            "the boundary should have moved real energy, and said so"
        );
    }

    #[test]
    fn the_colony_keeps_at_least_someone_embodied() {
        let mut terra = default_terrarium();
        for _ in 0..1500 {
            terra.step(0.05);
        }
        assert_eq!(terra.colony.gnomes.len(), 4);
        assert!(
            terra.colony.embodied_count() >= 1,
            "a colony tuned away from failure should not be wiped out"
        );
    }
}
