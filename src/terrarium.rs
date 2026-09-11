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
use crate::light::Sun;
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
    /// The window the jar sits in — see [`crate::light::Sun`]. It is a
    /// declared hole in the jar's energy budget just as the thermostats are,
    /// and its joules land in the same ledger.
    pub sun: Sun,
    pub steps: u64,
}

impl Terrarium {
    /// One tick: the sun, then the other boundary conditions, then physics,
    /// then the gnomes.
    pub fn step(&mut self, dt: Scalar) {
        self.sun.shine(&mut self.world, self.steps, dt);
        for th in &self.thermostats {
            th.apply(&mut self.world);
        }
        physics::step(&mut self.world, dt);
        self.colony.update(&mut self.world);
        self.steps += 1;
    }

    /// How bright it is in the jar right now, 0 to 1 — the number the
    /// browser view draws the sky with and the headless report prints.
    pub fn daylight(&self) -> Scalar {
        self.sun.intensity(self.steps)
    }
}

/// Wall thickness of the jar.
const SHELL: i32 = 2;
/// Temperature the spring under the pool is held at, in kelvin — warm, not
/// boiling: see `gnome_terrarium`.
const VENT_K: Scalar = 365.0;
/// Temperature the roof is held at.
const ROOF_K: Scalar = 283.0;
/// Steps in one day. At the 0.05 s step both the browser view and the
/// headless runner use, that is a hundred seconds of simulated time — long
/// enough for the jar to warm and cool visibly, short enough that a run of a
/// few thousand steps sees more than one.
const DAY_STEPS: u64 = 2000;
/// Joules per second the sun delivers to one fully-absorbing cell — see
/// [`Sun::power`]. Tuned against the thermostats already in this jar: over a
/// whole day it contributes roughly a fifth of what they do, which is enough
/// to give the pool a diurnal swing of a few kelvin without the roof having
/// to fight it.
const SUN_POWER: Scalar = 4.0;

/// Builds the sealed gnome terrarium at the given size. Sizes below about
/// 24x24 leave no room for the cycle; 48x36 is the tuned default used by
/// the browser view and the headless runner.
pub fn gnome_terrarium(width: usize, height: usize) -> Terrarium {
    let (w, h) = (width as i32, height as i32);
    let mut world = World::new_open(width, height, MaterialTable::terrarium(), 291.0);

    // The jar: a stone shell all the way round, with a glass lid.
    //
    // The lid used to be stone like the rest, and that was invisible until
    // night 5 gave plants a reason to care where the light was: a jar with a
    // stone lid is a jar in permanent darkness, so its garden could only ever
    // run the night half of its books and spend itself down. Glass is stone
    // with one number changed (`Material::opacity`), which is the right shape
    // for it — see `src/light.rs`.
    for i in 0..w {
        for j in 0..h {
            let edge = i < SHELL || j < SHELL || i >= w - SHELL || j >= h - SHELL;
            if !edge {
                continue;
            }
            let lid = j >= h - SHELL;
            world.fill(
                GridIndex::new(i, j),
                if lid { t::GLASS } else { t::STONE },
                291.0,
            );
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

    // The garden, at the meadow's far end: the colony's Gin supply, and
    // since night 5 a *growing* one.
    //
    // It used to be four bushes under a stone shelf, and the shelf was there
    // for a good reason — warm water touching juniper ferments both into
    // wash (`src/chemistry.rs`), so a bush that catches warm rain is a mash
    // tun nobody asked for. The shelf is gone now, and had to go, because a
    // plant in permanent shade is a plant that never photosynthesises: it
    // only runs the night half of its books, spending itself down. What
    // replaces the shelf is temperature. The meadow sits at the cold end of
    // the jar, a long way from the vent, and mashing needs the *pair* past
    // 310 K; rain that lands here is roof-cold.
    //
    // Under the bushes is a bed of water, walled in so it cannot spill onto
    // the walkway. It is the garden's whole water supply: photosynthesis
    // takes real grams of it apart to build plant, and the bed draws down
    // over a long run — which is the shape of the irrigation problem a
    // colony should have, not a bug to plumb away.
    let garden_to = SHELL + 5;
    for i in SHELL..garden_to {
        world.fill(GridIndex::new(i, SHELL + 1), t::WATER, 291.0);
        world.fill(GridIndex::new(i, SHELL + 2), t::JUNIPER, 291.0);
    }
    // The bed's far wall, one course proud of the water, so the gnomes'
    // walkway stays dry and a gnome standing on it is still next to a bush.
    world.fill(GridIndex::new(garden_to, SHELL + 1), t::STONE, 291.0);

    // A compost heap at the garden gate — two cells of leaf litter, the way
    // anybody planting a terrarium puts a handful of leaf mould in with the
    // cuttings.
    //
    // It is seeded rather than waited for, and the reason is scale, not
    // impatience. The jar's whole biology moves a few micrograms a step, and
    // a bush sheds a small fraction of that; left to build up from leaf fall
    // alone a heap this size would take a hundred thousand steps to appear.
    // Seeded, the rot cycle is running from the first minute — the heap goes
    // mouldy, sinks, and puts its carbon back into the air the garden is
    // breathing — and leaf fall is then what keeps it topped up rather than
    // what has to create it. The gnomes step over it; see
    // `Colony::is_walkable`.
    for i in garden_to..garden_to + 2 {
        world.fill(GridIndex::new(i, SHELL + 2), t::LITTER, 291.0);
    }

    world.rebaseline();

    // Gnomes on the meadow, plus one ethereal pipe — the sanctioned magic
    // shortcut, on display — running as a fountain: it lifts water from the
    // near end of the pool and lets it fall back in at the far end from
    // halfway up the jar. It used to drop its water onto a ledge over the
    // meadow, which was harmless beside a 950 K vent that boiled it away,
    // and floods the gnomes' home beside a warm spring that does not.
    // On the walkway between the garden's far wall and the pool's bank —
    // and *derived* from both, not stepped out by hand. Four gnomes on a
    // fixed stride from a fixed start put two of them inside the bank and
    // the pool the moment the garden got wider, where they spent five
    // hundred Gin and fifty grams of the pool gasping for air.
    // Clear of the compost heap at the garden gate — a gnome that starts
    // inside a heap of leaves is not hurt by it, but it is an odd first
    // frame and an odd first breath.
    let walk_from = garden_to + 3;
    let walk_to = pool_from - 2;
    let gnomes: Vec<Gnome> = (0..4)
        .map(|k| {
            let span = (walk_to - walk_from).max(1);
            let i = walk_from + (k * span) / 4;
            Gnome::new(GridIndex::new(i, SHELL + 2))
        })
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
    // ...and so is the base it stands on, everywhere except under the
    // spring. This was the one face of the shell that touched nothing, and
    // leaving it out had a consequence nobody saw until a run went past ten
    // thousand steps: stone conducts, the floor is one slab of it, and the
    // spring was heating the *whole* of it. The jar came to equilibrium with
    // its meadow at 340 K — over a gnome's lethal limit — so from about step
    // 10 000 onward the colony could not walk its own walkway. Four gnomes
    // spent the next seventy thousand steps confined to the twenty coolest
    // cells in the jar, in the shade at the garden end, eating the bushes
    // they could reach down to the graze floor and then starving beside
    // them. Every previous night read that as a pathing fault, because that
    // is what it looks like from inside the colony.
    //
    // A jar stands on a table, and a table is part of the room. The spring
    // still comes up under the pool, so the jar keeps its hot corner — what
    // it loses is the underfloor heating.
    for i in SHELL..pool_from {
        thermostats.push(Thermostat::new(GridIndex::new(i, 0), ROOF_K));
        thermostats.push(Thermostat::new(GridIndex::new(i, 1), ROOF_K));
    }

    Terrarium {
        world,
        colony,
        thermostats,
        // Starting a fifth of the way into the day, so the first thing a
        // viewer sees is a jar in morning light rather than one in the dark.
        sun: Sun::new(SUN_POWER, DAY_STEPS).starting_at(0.2),
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
            assert_eq!(
                terra.world.material_at(GridIndex::new(i, h - 1)),
                t::GLASS,
                "the lid is glass, so the garden under it can see the sun"
            );
        }
        for j in 0..h - SHELL {
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
        // Both halves of the cycle: what the plants took apart or put back,
        // and what the gnomes breathed out. They run on the same declared
        // proportions, which is the point — one number per side, and the
        // jar's water balance closes to a part in a million.
        // What living processes actually moved, species by species, plus what
        // the gnomes breathed out. This used to be inferred from how much
        // living matter had been built and spent, which worked only while
        // every process moved the same water per gram of host; a bush
        // shedding a dead leaf moves none, and the inference broke the day it
        // could. `World::life_moved` is the grams themselves.
        let h2o_per_g = 108.0 / 180.0;
        let expected = terra.world.life_moved(t::WATER)
            + terra.world.life_moved(t::STEAM)
            + terra.colony.respired_g() * h2o_per_g;
        let life = terra.world.life_tally();
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

    /// The jar's **carbon** goes round. Every gram of plant holds 0.4 g of
    /// carbon (glucose is 72 parts carbon in 180) and every gram of carbon
    /// dioxide holds 12 in 44, and a gnome's belly holds whatever it has
    /// eaten and not yet breathed out. Those three places are the only ones
    /// carbon can be in this jar, so their total is a constant — and it is
    /// constant to a part in a billion, not approximately, because every
    /// process that moves carbon moves it in declared proportions.
    ///
    /// This is the test the whole biology tier exists to be able to pass. A
    /// world where a plant "grows" without taking its mass from somewhere
    /// specific cannot state this quantity at all, let alone hold it.
    #[test]
    fn the_carbon_in_the_jar_goes_round_rather_than_accumulating() {
        let mut terra = default_terrarium();
        let carbon = |terra: &Terrarium| {
            // Everything made of what a bush is made of, alive or dead: a
            // standing bush, a mash, the leaves it has dropped, the mould
            // eating them, and whatever is in a gnome. All of it is glucose
            // as far as this jar is concerned, so all of it is 0.4.
            let organic = terra.world.mass_of(t::JUNIPER)
                + terra.world.mass_of(t::WASH)
                + terra.world.mass_of(t::LITTER)
                + terra.world.mass_of(t::FUNGUS);
            let belly: f64 = terra.colony.gnomes.iter().map(|g| g.belly).sum();
            0.4 * (organic + belly) + (12.0 / 44.0) * terra.world.mass_of(t::CO2)
        };
        let before = carbon(&terra);
        for _ in 0..4000 {
            terra.step(0.05);
        }
        let after = carbon(&terra);
        assert!(
            terra.world.mass_of(t::CO2) > 1e-4,
            "nothing ever breathed: {} g of CO2",
            terra.world.mass_of(t::CO2)
        );
        assert!(
            (after - before).abs() < 1e-9 * before,
            "carbon went {before} -> {after} g"
        );
    }

    /// The garden is a garden: it takes carbon out of the air and puts on
    /// weight, rather than only ever being eaten down.
    #[test]
    fn the_garden_grows_instead_of_only_being_picked() {
        let mut terra = default_terrarium();
        let before = terra.world.mass_of(t::JUNIPER);
        let cells_before = terra.world.count_of(t::JUNIPER);
        let organic_before = before + terra.world.mass_of(t::LITTER);
        // Twelve thousand rather than the six this used to run for, and the
        // reason is the whole of night 8. Once the gnomes could reach the
        // garden they started eating it, and a colony of four eats within a
        // few per cent of what this jar's carbon budget can grow: the
        // standing crop dips while they graze it and is back above where it
        // started a few thousand steps later. Six thousand steps landed in
        // the dip. What holds at every horizon is the pair below it — the
        // garden spreads, and the organic matter in the jar as a whole
        // (crop plus the litter the prunings and the leaf fall become) goes
        // up, which is what "this is a garden and not a larder" means when
        // something is living off it.
        for _ in 0..12000 {
            terra.step(0.05);
        }
        // What a bush builds by day has to beat what it spends at night
        // *and* what it drops. Measured as carbon dioxide taken out of the
        // air against carbon dioxide put back by everything living — the two
        // directions of the same cycle — because "grams of host built and
        // spent" stopped separating them the moment a bush could shed
        // something that takes no carbon dioxide with it.
        let life = terra.world.life_tally();
        let co2_taken = -terra.world.life_moved(t::CO2);
        assert!(
            co2_taken > 0.0,
            "the garden should be a net sink of carbon dioxide: {co2_taken} g ({life:?})"
        );
        // The standing crop *holds its ground*, rather than gaining. That
        // is a weaker claim than this test used to make and it is the true
        // one: since night 8 the gnomes can actually reach the garden, and
        // four of them graze it within a few per cent of what this jar's
        // carbon budget grows, so the standing crop wanders up and down a
        // per cent or two either side of where it started depending on
        // where in a meal the run happens to stop. What is not ambiguous is
        // the three assertions around this one: it is spreading, it is a
        // net sink of carbon dioxide, and the organic matter in the jar as
        // a whole is up.
        let crop = terra.world.mass_of(t::JUNIPER);
        assert!(
            crop > 0.8 * before,
            "the garden was eaten down: {before} -> {crop} g ({life:?})"
        );
        assert!(
            terra.world.count_of(t::JUNIPER) > cells_before,
            "and it should have spread as well as fattened"
        );
        let organic = terra.world.mass_of(t::JUNIPER)
            + terra.world.mass_of(t::LITTER)
            + terra.world.mass_of(t::FUNGUS);
        assert!(
            organic > organic_before,
            "the jar's standing organic matter fell: {organic_before} -> {organic} g"
        );
    }

    /// There is a day and a night, and the jar notices: the pool is warmer
    /// at midday than it is before dawn.
    #[test]
    fn the_jar_has_weather_because_it_has_a_day() {
        let mut terra = default_terrarium();
        let pool = GridIndex::new(terra.world.width() as i32 - 6, SHELL + 5);
        // Two days in, so the jar is past its opening transient.
        for _ in 0..4000 {
            terra.step(0.05);
        }
        let mut warmest: Scalar = 0.0;
        let mut coolest = Scalar::INFINITY;
        let mut brightest: Scalar = 0.0;
        let mut darkest = Scalar::INFINITY;
        for _ in 0..2000 {
            terra.step(0.05);
            let t = terra.world.cell(pool).temperature;
            warmest = warmest.max(t);
            coolest = coolest.min(t);
            brightest = brightest.max(terra.daylight());
            darkest = darkest.min(terra.daylight());
        }
        assert!(
            brightest > 0.9 && darkest == 0.0,
            "the sun should rise and set: {darkest} to {brightest}"
        );
        assert!(
            warmest - coolest > 0.5,
            "the pool should swing over a day, got {coolest} to {warmest} K"
        );
    }

    #[test]
    fn a_gnome_needs_the_oxygen_the_garden_makes() {
        let mut terra = default_terrarium();
        let before = terra.world.mass_of(t::OXYGEN);
        for _ in 0..4000 {
            terra.step(0.05);
        }
        // The colony really does breathe its jar down — that is the whole
        // point of oxygen being a species rather than a flag.
        assert!(
            terra.world.mass_of(t::OXYGEN) < before,
            "nobody breathed anything: {before} g"
        );
        assert!(
            terra.colony.respired_g() > 0.0,
            "the colony never burned any food"
        );
        // ...but not so fast that anyone suffocates in a demonstration.
        for g in &terra.colony.gnomes {
            assert!(g.is_embodied(), "a gnome suffocated inside 4000 steps");
        }
    }

    /// The jar is still *running* late on, not merely still conserving.
    ///
    /// This is the test the seventh night's first measurement asked for. Run
    /// long enough, the colony used to stop: by step 30000 of the version
    /// before tonight it had spent its last Gin, eaten nothing more, and sat
    /// there — four gnomes alive, embodied, and doing nothing, with the
    /// ledger's `mass_conjured` frozen to the microgram for the next 30000
    /// steps. Every invariant held. Nothing was happening. So what is
    /// asserted here is appetite: the colony has eaten in the *last* quarter
    /// of the run, not just in the first.
    #[test]
    fn the_colony_is_still_feeding_itself_late_in_a_long_run() {
        let mut terra = default_terrarium();
        for _ in 0..7500 {
            terra.step(0.05);
        }
        let eaten_by_then = terra.colony.respired_g();
        let rotted_by_then = terra.world.mass_of(t::CO2);
        for _ in 0..2500 {
            terra.step(0.05);
        }
        assert!(
            terra.colony.respired_g() > eaten_by_then,
            "nobody has eaten anything since step 7500 ({eaten_by_then} g)"
        );
        assert_eq!(
            terra.colony.embodied_count(),
            4,
            "somebody left the world: {:?}",
            terra
                .colony
                .gnomes
                .iter()
                .map(|g| g.gin)
                .collect::<Vec<_>>()
        );
        assert!(
            terra.colony.total_gin() > 10.0,
            "the colony is broke: {} Gin",
            terra.colony.total_gin()
        );
        // And the jar still has a working air supply — the garden is putting
        // oxygen back faster than it is being breathed away.
        assert!(
            terra.world.mass_of(t::OXYGEN) > 0.2,
            "the jar has been breathed down to {} g of oxygen",
            terra.world.mass_of(t::OXYGEN)
        );
        assert!(rotted_by_then > 0.0);
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

    /// The glass pane, end to end, in the jar a human actually looks at: the
    /// player writes *dig* on a cell of the meadow, a gnome walks over and
    /// takes it away, the player writes *build* on a cell of air, and the
    /// same grams come back down there.
    ///
    /// The numbers asserted here are the whole argument for doing it this
    /// way. While the sand is in a gnome's hands the world is exactly that
    /// much lighter and the ledger holds exactly that much against it; when
    /// it is put down, both come back. There is no resource counter anywhere
    /// in between — the mass *is* the resource.
    #[test]
    fn a_player_can_dig_a_hole_and_build_the_spoil_somewhere_else() {
        use crate::order::Job;
        let mut terra = default_terrarium();
        // Let the jar settle and the gnomes find their feet first.
        for _ in 0..200 {
            terra.step(0.05);
        }
        // Sand is the tracer: nothing else in this jar moves any, so every
        // gram that leaves the world's sand is a gram in somebody's hands.
        // (The jar's *total* mass is not a constant even without a player —
        // the gnomes are eating, which is its own ledger entry.)
        let sand_before = terra.world.mass_of(t::SAND);

        // A cell of the meadow the gnomes are standing on.
        let hole = GridIndex::new(10, SHELL + 1);
        assert_eq!(terra.world.material_at(hole), t::SAND);
        terra.colony.order(hole, Job::Dig);

        let mut dug_at = None;
        for step in 0..2000 {
            terra.step(0.05);
            if terra.colony.orders.completed() == 1 {
                dug_at = Some(step);
                break;
            }
        }
        let dug_at = dug_at.expect("nobody dug the hole");
        assert_ne!(
            terra.world.material_at(hole),
            t::SAND,
            "the hole should be a hole"
        );
        let carried = terra.colony.carried_g();
        assert!(carried > 0.0, "somebody should be holding the spoil");
        assert!(
            (sand_before - terra.world.mass_of(t::SAND) - carried).abs() < 1e-6,
            "the world should be lighter by exactly what is in hand: {} vs {carried}",
            sand_before - terra.world.mass_of(t::SAND)
        );
        let mid = terra.world.conservation_residuals();
        assert!(
            mid.mass_relative.abs() < 1e-6,
            "the ledger should hold exactly that much against it: {mid:?}"
        );

        // Now put it back down as a block on the walkway. Sand is granular,
        // so a cell of it built in mid-air would fall — a wall has to be
        // built on something, which is the physics doing the game design.
        let wall = GridIndex::new(12, SHELL + 2);
        assert!(terra.world.cell(wall).is_gas(terra.world.materials()));
        terra.colony.order(wall, Job::Build);
        let mut built = false;
        for _ in 0..2000 {
            terra.step(0.05);
            if terra.colony.orders.completed() == 2 {
                built = true;
                break;
            }
        }
        assert!(built, "nobody built the wall (dug at step {dug_at})");
        assert_eq!(terra.world.material_at(wall), t::SAND);
        // And it is still there a while later — built on the ground, not
        // hanging in the air.
        for _ in 0..200 {
            terra.step(0.05);
        }
        assert_eq!(
            terra.world.material_at(wall),
            t::SAND,
            "the wall should stay where it was put"
        );
        assert!(
            terra.colony.carried_g() < 1e-9,
            "hands should be empty again"
        );
        assert!(
            (terra.world.mass_of(t::SAND) - sand_before).abs() < 1e-6,
            "every gram dug should be back in the world"
        );
        let r = terra.world.conservation_residuals();
        assert!(
            r.mass_relative.abs() < 1e-6 && r.energy_relative.abs() < 1e-4,
            "residuals drifted: {r:?}"
        );
    }

    /// Tempering is the magic one, and it is priced like magic: the player
    /// asks for a cell to be warmed, a gnome pays for it out of its flask,
    /// and the joules land in the ledger.
    #[test]
    fn a_warming_order_is_paid_for_in_gin_and_booked() {
        use crate::order::Job;
        let mut terra = default_terrarium();
        for _ in 0..200 {
            terra.step(0.05);
        }
        // The floor of the meadow, not the air over it. A cell of air has
        // almost no heat capacity and is being replaced by its neighbours
        // every step, so "hold this cell of air at 320 K" is a spell against
        // the weather: the gnome casts it for ever and the cell settles
        // wherever convection wants it. That is the right physics and it is
        // worth knowing — tempering works on things that hold heat.
        let at = GridIndex::new(10, SHELL + 1);
        let before_k = terra.world.cell(at).temperature;
        let gin_before = terra.colony.total_gin();
        let booked_before = terra.world.ledger().energy_conjured;
        assert!(
            before_k < crate::gnome::COMFORT_MAX - 5.0,
            "the meadow should start cool enough to be worth warming"
        );

        terra.colony.order(
            at,
            Job::Temper {
                target_k: crate::gnome::COMFORT_MAX,
            },
        );
        for _ in 0..600 {
            terra.step(0.05);
            if terra.colony.orders.completed() == 1 {
                break;
            }
        }
        assert_eq!(terra.colony.orders.completed(), 1, "the spell never landed");
        assert!(
            terra.colony.total_gin() < gin_before,
            "magic on demand should cost the colony Gin"
        );
        assert!(
            terra.world.ledger().energy_conjured > booked_before,
            "and every joule of it should be on the books"
        );
        let r = terra.world.conservation_residuals();
        assert!(r.energy_relative.abs() < 1e-4, "residuals drifted: {r:?}");
    }

    /// An order on a cell nobody can get to is given up on rather than
    /// leaving a gnome walking into a wall for the rest of the run.
    #[test]
    fn an_order_nobody_can_reach_is_eventually_abandoned() {
        use crate::order::Job;
        let mut terra = default_terrarium();
        // The middle of the jar's own outer shell, on the far side of a
        // sealed wall and well over a gnome's head.
        let unreachable = GridIndex::new(1, terra.world.height() as i32 / 2);
        terra.colony.order(unreachable, Job::Dig);
        let budget = crate::order::PATIENCE as usize * crate::order::ATTEMPTS as usize + 200;
        let mut gave_up = false;
        let mut saw_unreachable = false;
        for _ in 0..budget {
            terra.step(0.05);
            saw_unreachable |= terra.colony.orders.iter().any(|o| !o.reachable);
            if terra.colony.orders.is_empty() {
                gave_up = true;
                break;
            }
        }
        assert!(gave_up, "the queue jammed on an unreachable order");
        assert!(
            saw_unreachable,
            "and it should have been visibly unreachable while it lasted"
        );
        assert_eq!(terra.colony.orders.completed(), 0);
        assert!(terra.colony.orders.cancelled() >= 1);
    }
}
