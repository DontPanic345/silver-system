//! The flagship scenario: a sealed jar with a warm spring under a pool, a
//! cold lid over it, and gnomes living on a meadow beside the water.
//!
//! This exists because the north stars are not satisfied by a physics
//! engine that passes tests. *"Only when the big universe is sufficiently
//! full will the small world be believable"* — so the demonstration is a
//! world with a water cycle running on its own: the spring warms the pool,
//! the pool evaporates into the air over it, the humid air meets the cold
//! lid, dew forms there and falls back as drops. No script drives any of
//! that; it falls out of conduction, buoyancy, and one vapour-pressure
//! curve (`src/vapour.rs`).
//!
//! Until night 4 it ran on boiling instead — a 950 K vent beside the pool,
//! the jar near 400 K, the gnomes paying Gin every step to survive — because
//! until night 4 water could only become vapour by boiling. A real
//! terrarium's water cycle is dew on the glass, and now so is this one's.
//! It is also slow, the way a real one is: a few grams of rain per few
//! minutes of simulated time, because a jar of air at room temperature
//! holds only a few hundredths of a gram of water.
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
        let capacity = cell.capacity(world.materials());
        if capacity <= 0.0 {
            return;
        }
        let joules = (self.target_k - cell.temperature) * capacity * self.rate;
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
/// Temperature the spring under the pool is held at, in kelvin — warm, not
/// boiling: see `gnome_terrarium`.
const VENT_K: Scalar = 365.0;
/// Temperature the roof is held at.
const ROOF_K: Scalar = 283.0;

/// Builds the sealed gnome terrarium at the given size. Sizes below about
/// 24x24 leave no room for the cycle; 48x36 is the tuned default used by
/// the browser view and the headless runner.
pub fn gnome_terrarium(width: usize, height: usize) -> Terrarium {
    let (w, h) = (width as i32, height as i32);
    let mut world = World::new_open(width, height, MaterialTable::terrarium(), 291.0);

    // The jar: a stone shell all the way round.
    for i in 0..w {
        for j in 0..h {
            let edge = i < SHELL || j < SHELL || i >= w - SHELL || j >= h - SHELL;
            if edge {
                world.fill(GridIndex::new(i, j), t::STONE, 291.0);
            }
        }
    }

    // The pool, and the warm spring under it: the jar's heat source.
    //
    // This used to be a patch of rock held at 950 K *beside* the pool, and
    // the jar was built around boiling: the vent boiled the pool, the steam
    // rose, the lid rained it back. It worked, and it ran the whole jar at
    // around 400 K — sixty kelvin past what a gnome can survive without
    // paying Gin to chill itself, which the colony did, every step, for
    // ever. It had to be that hot because until night 4 water could only
    // become vapour by boiling.
    //
    // Now water evaporates at any temperature (`src/vapour.rs`), so the
    // jar can run the cycle a real terrarium runs: a pool warmed from
    // below, evaporating into the air over it; the humid air reaching the
    // cold lid; dew forming there and falling as drops. The spring is warm
    // rather than hot — warm enough that the pool turns over (hot water is
    // lighter than cold, `Material::buoyant_density`) and gives up vapour
    // briskly, cool enough that nothing boils and the meadow the gnomes
    // live on stays near room temperature.
    // Left to right: the juniper, the meadow the gnomes live on, and the
    // pool running to the far wall. The bushes are on the gnomes' side of
    // the water on purpose — gnomes will not wade, and in every earlier
    // layout of this jar the colony's only Gin supply stood on the far bank
    // of a pool nobody could cross.
    let pool_from = w / 3;
    let pool_to = w - SHELL;
    for i in pool_from..pool_to {
        for j in SHELL..SHELL + 2 {
            world.fill(GridIndex::new(i, j), t::STONE, VENT_K);
        }
        // Four courses deep: the pool is also the jar's thermal ballast.
        for j in SHELL + 2..SHELL + 6 {
            world.fill(GridIndex::new(i, j), t::WATER, 300.0);
        }
    }
    // The meadow: sand over rock, walled off from the pool by a stone bank
    // a course higher than the water. A gnome will not wade, but it will
    // happily stroll out across warm wet rock wherever the water has drawn
    // back, and then need chilling; and a bank lower than the pool is a
    // weir, which floods the meadow.
    for i in SHELL..pool_from {
        world.fill(GridIndex::new(i, SHELL), t::STONE, 291.0);
        world.fill(GridIndex::new(i, SHELL + 1), t::SAND, 291.0);
    }
    for j in SHELL + 2..SHELL + 7 {
        world.fill(GridIndex::new(pool_from - 1, j), t::STONE, 291.0);
    }

    // The lid is held cold, but not freezing. An ice roof was the obvious
    // choice and it was wrong: steam hitting ice at 240 K condenses and
    // then immediately freezes, so the roof just grows thicker and thicker
    // and the water never comes back down. Ice is static; rain is not. A
    // lid a little above freezing condenses vapour into liquid water, which
    // falls — which is the half of the cycle worth having.

    // Juniper at the meadow's far end: the colony's Gin supply, standing on
    // the meadow itself, where a gnome can walk up beside a bush and pick
    // it. Under a stone shelf, because it has to stay *dry*: warm water
    // touching juniper ferments both into wash (see `src/chemistry.rs`), so
    // a bush that catches warm rain is a mash tun the colony did not ask
    // for. (It used to stand on a raised plinth past the far end of the
    // pool — out of reach twice over, since gnomes will not wade and a
    // gnome cannot pick a bush a row above the floor it stands on.)
    let plinth_to = SHELL + 4;
    for i in SHELL..plinth_to {
        world.fill(GridIndex::new(i, SHELL + 2), t::JUNIPER, 291.0);
    }
    for i in SHELL..=plinth_to + 1 {
        world.fill(GridIndex::new(i, SHELL + 4), t::STONE, 291.0);
    }

    world.rebaseline();

    // Gnomes on the meadow, plus one ethereal pipe — the sanctioned magic
    // shortcut, on display — running as a fountain: it lifts water from the
    // near end of the pool and lets it fall back in at the far end from
    // halfway up the jar. It used to drop its water onto a ledge over the
    // meadow, which was harmless beside a 950 K vent that boiled it away,
    // and floods the gnomes' home beside a warm spring that does not.
    let gnomes: Vec<Gnome> = (0..4)
        .map(|k| Gnome::new(GridIndex::new(plinth_to + 2 + k * 2, SHELL + 2)))
        .collect();
    let pipe = EtherealPipe::new(
        GridIndex::new(pool_from + 2, SHELL + 2),
        GridIndex::new(pool_to - 3, h / 2),
    )
    .every(60);
    let colony = Colony::new(gnomes).with_pipe(pipe);

    // The boundary: the vent is held hot, the roof cold. Everything these
    // add or remove is booked — see the module doc comment.
    let mut thermostats = Vec::new();
    for i in pool_from..pool_to {
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

    /// The cycle, in numbers: the pool evaporates, the air gives it back
    /// as dew and rain, and none of it is boiling — water becomes vapour
    /// here because warm water does, not because anything reached 373 K.
    #[test]
    fn the_water_cycle_runs_on_its_own() {
        let mut terra = default_terrarium();
        let water0 = terra.world.mass_of(t::WATER) + terra.world.mass_of(t::STEAM);
        let mut boiled = 0usize;
        for _ in 0..4000 {
            terra.step(0.05);
            boiled = boiled.max(terra.world.count_of(t::STEAM));
        }
        let tally = terra.world.tally();
        assert!(
            tally.evaporated_g > 1.0,
            "the pool barely evaporated: {tally:?}"
        );
        assert!(
            tally.condensed_g > 1.0,
            "the air never gave it back: {tally:?}"
        );
        assert!(tally.rained_g > 1.0, "no dew ever fell: {tally:?}");
        assert!(
            (tally.evaporated_g - tally.rained_g).abs() < 0.5 * tally.evaporated_g,
            "what evaporates should come back down, not pile up in the air: {tally:?}"
        );
        assert_eq!(boiled, 0, "nothing should be boiling in a terrarium");
        // And the water is still water — in the pool, in the air, or on its
        // way down — except for exactly what life took apart or put back.
        //
        // This used to be a flat "water is conserved", and it stopped being
        // true the night plants arrived, correctly: photosynthesis splits
        // water to build a bush and the bush's own respiration puts it back,
        // so the free water in a jar rises and falls with how much plant is
        // standing in it. What is still exact is the *proportion*, which is
        // what this now asserts — the same 108/180 g per gram of plant that
        // the material table declares, measured at scenario scale.
        let water = terra.world.mass_of(t::WATER) + terra.world.mass_of(t::STEAM);
        let life = terra.world.life_tally();
        let h2o_per_g = 108.0 / 180.0;
        let expected = (life.respired_g - life.grown_g) * h2o_per_g;
        assert!(
            (water - water0 - expected).abs() < 1e-6 * water0,
            "water went {water0} -> {water} g, but life only moved {expected} g of it \
             ({life:?})"
        );
    }

    /// A terrarium the gnomes can live in: the jar sits inside a gnome's
    /// comfortable band, so nobody spends Gin just to survive the weather.
    /// The jar used to run near 400 K, and the colony bled Gin chilling
    /// itself every step it existed.
    #[test]
    fn the_jar_is_somewhere_a_gnome_can_live() {
        let mut terra = default_terrarium();
        for _ in 0..4000 {
            terra.step(0.05);
        }
        let mean = terra.world.mean_temperature();
        assert!(
            mean < crate::gnome::COMFORT_MAX,
            "the jar settled at {mean} K, past a gnome's comfort"
        );
        for g in &terra.colony.gnomes {
            let here = terra.world.cell(g.pos).temperature;
            assert!(
                here < crate::gnome::LETHAL_MAX,
                "a gnome is standing somewhere lethal: {here} K at {:?}",
                g.pos
            );
        }
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
