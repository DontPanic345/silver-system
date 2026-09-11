//! Gnomes: the game layer, and the only thing in this crate allowed to
//! break conservation — at a price, and on the record.
//!
//! `NORTH_STARS.md` #4 is the capstone this exists to serve. Its central
//! idea is not "simulate physics properly" on its own; it is that real
//! conservation *becomes the game mechanic*. A gnome can create or destroy
//! heat and mass, which no other part of this simulation can do, but every
//! such act spends **Gin** — a finite resource regenerated only by
//! foraging juniper — and is written into [`World`]'s
//! [`Ledger`](crate::world::Ledger). The world therefore stays closed in
//! the only sense that matters: nothing changes without something paying
//! for it, and the books always balance.
//!
//! Three of the dictated design's specifics are built here:
//!
//! - **Gin as a bounded mana resource.** Warming or chilling a cell costs
//!   Gin in proportion to the joules moved; conjuring or banishing matter
//!   costs it in proportion to the grams. A gnome out of Gin has no magic
//!   at all, which is what stops magic being a free pass around the
//!   physics.
//! - **The ethereal layer instead of death.** ONI's colonies are tuned to
//!   fail and dupes die; the note this is built from calls that out
//!   directly. A gnome in lethal trouble — drowning, roasting, freezing —
//!   spends Gin on cheap mitigations first, and only when those run out
//!   does it slip into the ethereal layer, from which another gnome can
//!   retrieve it. Nothing is ever lost permanently.
//! - **Ethereal pipes as an honest shortcut.** ONI's pipes move liquid
//!   uphill through a layer disconnected from physics and pretend that is
//!   physical. Here that shortcut exists — [`EtherealPipe`] moves matter
//!   from one node to another regardless of what lies between — but it is
//!   explicitly magical, it costs Gin, and, because it is implemented as a
//!   swap of two cells, it still cannot create or destroy anything.
//!
//! Gin also comes from drinking the real thing, brewed and distilled in
//! `src/still.rs`. Not built here: the knowledge economy, and buildings.
//! See `JOURNAL.md`.

use crate::material::{terrarium as t, Mobility, Phase};
use crate::math::{GridIndex, Scalar};
use crate::order::{Job, Load, Orders, Outcome};
use crate::path::{self, Routes, Through};
use crate::world::World;

/// The most Gin a gnome can hold.
pub const MAX_GIN: Scalar = 100.0;
/// Gin per joule moved by a warming or chilling spell. Tuned so a gnome
/// with a full flask can shift roughly the heat in a cell of water by 100 K
/// a few times over — magic that matters, but that runs out.
pub const GIN_PER_JOULE: Scalar = 1.0 / 4000.0;
/// Gin per gram conjured or banished. Matter is dearer than heat.
pub const GIN_PER_GRAM: Scalar = 8.0;
/// Grams a gnome picks off a bush at a time — the bite, not the worth.
///
/// What a mouthful of anything is *worth* is [`Material::nutrition`], a
/// column in the material table, because "what is food" is a property of
/// the stuff and not of this file. A berry is 0.05 g of juniper at 600 Gin
/// to the gram, so it is worth the 30 Gin it has been worth since night 1;
/// a cell of the still's own output is worth about twice that, which is the
/// whole reason to go to the trouble of building a still.
///
/// [`Material::nutrition`]: crate::material::Material::nutrition
pub const BERRY_MASS: Scalar = 0.05;
/// The smallest bite a gnome will stop for — a fifth of a berry. See
/// [`Colony::spare_berry`].
pub const MIN_BITE: Scalar = 0.2 * BERRY_MASS;
/// How much of a bush a gnome always leaves standing, as a fraction of a
/// full cell of it: never strip a bush bare, because a garden picked down
/// to nothing cannot grow back.
///
/// It was a quarter, and a quarter is above where this jar's garden
/// actually sits. The standing crop settles near 0.15 g a cell against a
/// full cell's 0.5, so almost every bush in it was under the floor, almost
/// nothing was ever ripe, and a colony walking through a garden of
/// seventeen bushes foraged twelve times in twenty thousand steps and ended
/// the run with empty bellies and no Gin. A tenth leaves a bush that is
/// visibly still a bush and still growing, and leaves the colony able to
/// live off it.
pub const GRAZE_FLOOR: Scalar = 0.1;
/// Gin a rescued gnome comes back with — a berry's worth, pressed on it by
/// whoever pulled it out.
pub const GIN_ON_RESCUE: Scalar = 30.0;

/// Below this, a gnome starts looking for juniper instead of working.
pub const GIN_HUNGRY: Scalar = 45.0;

/// Grams of food in the belly below which a gnome goes looking for a meal,
/// whatever its flask says.
///
/// Gin and food used to be the same appetite, and that quietly broke the
/// carbon cycle the moment there was one: a colony in a comfortable jar
/// spends almost no Gin, so its flasks stay full, so it never eats, so it
/// never breathes out any carbon, so the garden it lives in starves. A gnome
/// is hungry because it is alive, not because it is out of mana.
pub const BELLY_HUNGRY: Scalar = 0.02;
/// Grams of food a gnome will carry before it stops picking berries.
pub const BELLY_FULL: Scalar = 0.06;
/// Grams of a gnome's carried food that become a cutting when it plants one,
/// and the belly it will not plant below.
///
/// `NORTH_STARS.md` #4 names farming as one of the pillars, and this is the
/// smallest honest version of it: a gnome puts back into the world some of
/// what it is carrying, as a living seedling. It is not magic in the sense
/// that matters — the mass comes out of the gnome's belly, which the ledger
/// has been holding negative since it was picked, so planting a cutting
/// *returns* the books toward zero rather than moving them away from it.
///
/// A gnome plants only directly above a bush it is standing beside. That is
/// a reach limit, not a rule about hedges: it means the garden can be
/// trained one course taller than the gnomes' own heads and no further by
/// hand, which is both a plausible thing to be able to do and the thing that
/// stops a colony walling itself in behind its own allotment.
pub const CUTTING_G: Scalar = 0.05;
pub const BELLY_TO_PLANT: Scalar = 0.05;
/// Colony updates between cuttings.
///
/// A cadence, not a cost, and it is load-bearing. Without it a gnome picks a
/// berry off one bush and plants it on the next one the same second, for
/// ever: the mass is conserved and the colony looks busy, and all that is
/// really happening is a bush being moved one cell at a time. A garden is
/// planted at the pace a garden grows.
pub const PLANT_PERIOD: u64 = 600;

/// The temperature band a gnome is comfortable in, in kelvin.
pub const COMFORT_MIN: Scalar = 265.0;
pub const COMFORT_MAX: Scalar = 320.0;
/// Outside this wider band, staying put is lethal.
pub const LETHAL_MIN: Scalar = 250.0;
pub const LETHAL_MAX: Scalar = 340.0;

/// How close to lethal a cell may be before a gnome refuses to walk into
/// it. A couple of kelvin of hindsight: the cell a gnome steps into is not
/// the cell it measured, because the world has a step of physics in between.
pub const SHUN_MARGIN_K: Scalar = 5.0;

/// Whether a gnome would refuse to *walk into* this cell.
///
/// Only heat and cold, and deliberately: drowning, suffocating and being
/// buried are all survivable with a step sideways or a gasp, and a route
/// that treated every one of them as a wall would leave a gnome standing in
/// the one it is already in. Fire is the one a gnome cannot walk out of
/// afterwards.
pub fn dangerous(world: &World, at: GridIndex) -> bool {
    let t = world.temperature_at(at);
    t > LETHAL_MAX - SHUN_MARGIN_K || t < LETHAL_MIN + SHUN_MARGIN_K
}

/// Steps a gnome spends penned in and with nothing it can reach before it
/// will cut a crop to get out.
///
/// Cutting is a last resort and this is the whole of what makes it one.
/// A gnome that is walking, eating or working resets this to nothing, so
/// the only thing that reaches the far end of it is a gnome that has been
/// getting nowhere for a hundred seconds of simulated time.
///
/// The number is bounded from both sides and both bounds were measured. Too
/// short and it is not a last resort but a habit: at 120 steps, four gnomes
/// that had fallen into the garden's water bed cut a bush every thirty
/// steps between them and took 2.5 g of garden down to 0.06 g. Too long and
/// a colony walled off from its own garden by one bush that seeded itself
/// across the gap never gets out — which is the thing night 7 left behind
/// and this night exists to fix. At 2000 the hedge costs the colony one
/// bush and about two minutes, and a colony that keeps falling into a hole
/// somebody dug cuts at most a couple of cells before it is out.
pub const TRAPPED_STEPS: u32 = 2000;

/// How many courses of crop a gnome will lift at once to clear its way —
/// see [`Colony::heave`]. A hedge, not a tree.
pub const LIFT_MAX: i32 = 4;

/// Steps a gnome can hold its breath in something unbreathable.
pub const BREATH_STEPS: u32 = 40;

/// The partial pressure of breathable gas a gnome needs, as a fraction of
/// one atmosphere. Open air is 0.21 of an atmosphere of oxygen, and a human
/// is in trouble below about half of that; a gnome is hardier.
///
/// Before night 5 there was no such number, because "can I breathe here" was
/// a flag on the material a cell was *labelled* with. That answered the
/// wrong question as soon as gas cells became mixtures — and it made the
/// whole ONI complaint that `NORTH_STARS.md` #4 is built around
/// unreachable, since a room cannot run out of air if nothing in it is ever
/// consumed. Now a gnome suffocates in a sealed room because it has actually
/// breathed the oxygen out of it, and the bushes put it back.
pub const MIN_BREATHABLE: Scalar = 0.08;

/// Grams of biomass a gnome burns per step, and the gnome's body
/// temperature — the two numbers respiration is made of.
///
/// The rate is tuned, and the reason is a scale mismatch worth writing down.
/// A creature this size really does burn a few milligrams of sugar a second;
/// the jar it lives in holds about a gram of air, a quarter of which is
/// oxygen. At a realistic rate four gnomes would breathe a sealed terrarium
/// flat in half a minute of simulated time, which is not a colony sim, it is
/// an execution. The grid's cells are not metres and a gnome is not a cell,
/// so there is no consistent scale to appeal to here; this figure is chosen
/// so a colony spends roughly a fifth of its jar's oxygen over the few
/// minutes a demonstration runs for, which is enough that the bushes putting
/// it back is a fact you can read off a graph rather than a claim.
pub const RESPIRATION_G: Scalar = 3.0e-6;
pub const BODY_K: Scalar = 310.0;

/// The stoichiometry of burning a gram of biomass, taken straight off the
/// table's own photosynthesis row so the two can never drift apart: a gnome
/// undoes exactly what a plant did.
fn respiration_ratios(world: &World) -> Option<(Scalar, Scalar, Scalar)> {
    let m = world
        .materials()
        .metabolisms()
        .iter()
        .find(|m| m.name == "photosynthesis")?;
    let grams = |rs: &[crate::material::Reagent], id| {
        rs.iter()
            .filter(|r| r.material == id)
            .map(|r| r.grams)
            .sum::<Scalar>()
    };
    Some((
        grams(&m.output, t::OXYGEN),
        grams(&m.intake, t::CO2),
        grams(&m.intake, t::WATER),
    ))
}
/// How close an embodied gnome must be to pull one back from the ethereal
/// layer, and how many steps of proximity it takes.
pub const RESCUE_RADIUS: i32 = 4;
pub const RESCUE_STEPS: u32 = 20;

/// Whether a gnome is in the world or in the ethereal layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Body {
    /// Present, physical, and killable-but-for-the-ethereal-layer.
    Embodied,
    /// Slipped out of the world to escape something lethal, waiting to be
    /// pulled back. `rescue_progress` counts steps spent with a living
    /// gnome nearby.
    Ethereal { rescue_progress: u32 },
}

/// What a gnome did on the step just simulated — the readable trace the
/// headless report prints, so behaviour can be checked by numbers instead
/// of by watching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    Idle,
    Walked,
    Fell,
    Foraged,
    /// Drank a cell of gin — the brewing half of the Gin economy.
    Drank,
    Warmed,
    Chilled,
    /// Spent Gin conjuring breathable air around itself.
    Breathed,
    /// Put a cutting from its own stores into the world — farming.
    Planted,
    /// Cut a crop that was in the way — the last resort when a hedge has
    /// shut the colony in and it cannot be lifted aside.
    Harvested,
    /// Heaved a crop up a course, out of the walking row: the colony's
    /// first answer to a hedge that has grown across the path, and the one
    /// that costs the garden nothing at all.
    Heaved,
    /// Took a cell of solid out of the world, on the player's orders.
    Dug,
    /// Put a carried cell back down, on the player's orders.
    Built,
    /// Spent Gin warming or chilling a cell, on the player's orders.
    Tempered,
    WentEthereal,
    Rescued,
    Waiting,
}

/// One gnome.
#[derive(Debug, Clone, Copy)]
pub struct Gnome {
    pub pos: GridIndex,
    /// Where this gnome slipped out of the world, and where a rescue
    /// returns it to. Meaningful only while ethereal.
    pub anchor: GridIndex,
    pub gin: Scalar,
    pub body: Body,
    /// Steps of held breath remaining; refills instantly in breathable air.
    pub breath: u32,
    pub last_act: Act,
    /// Which way this gnome is walking.
    pub facing: i8,
    /// Grams of biomass eaten and not yet breathed out again.
    ///
    /// A gnome's body is the one part of this world that is outside the
    /// simulation, and this is how much of the world is currently inside it.
    /// Everything that goes in is booked out of the [`Ledger`] and
    /// everything that comes back is booked in, so a gnome that has eaten a
    /// berry and finished digesting it has left the world's books exactly
    /// where it found them — which is what makes the jar's carbon cycle
    /// closed rather than merely tidy.
    ///
    /// [`Ledger`]: crate::world::Ledger
    pub belly: Scalar,
    /// What this gnome is carrying, if anything — see [`Load`].
    ///
    /// The hands are the belly's sibling: mass that has left the world and
    /// is booked out of the ledger until it is put down again. The
    /// difference is that a belly empties itself and hands do not, so a
    /// colony that digs and never builds shows up as a permanent, visible
    /// entry in the ledger rather than as matter quietly deleted.
    pub hands: Option<Load>,
    /// The order this gnome has claimed, by id — see [`Orders`].
    pub job: Option<u64>,
    /// Consecutive steps spent penned in with nowhere to go — see
    /// [`TRAPPED_STEPS`].
    pub stuck: u32,
}

impl Gnome {
    pub fn new(pos: GridIndex) -> Self {
        Gnome {
            pos,
            anchor: pos,
            gin: MAX_GIN,
            body: Body::Embodied,
            breath: BREATH_STEPS,
            last_act: Act::Idle,
            facing: 1,
            // Fed, rather than starving on arrival: a gnome that has to find
            // its first meal before it can breathe is a gnome that suffocates
            // on the way to breakfast, and one that arrives with less than a
            // cutting in it cannot start gardening for three simulated
            // minutes.
            belly: BELLY_FULL,
            hands: None,
            job: None,
            stuck: 0,
        }
    }

    /// Starts this gnome with a partly-empty flask, builder-style — the
    /// only way to get one that is actually motivated to go and find a
    /// drink.
    pub fn with_gin(mut self, gin: Scalar) -> Self {
        self.gin = gin.clamp(0.0, MAX_GIN);
        self
    }

    pub fn is_embodied(&self) -> bool {
        matches!(self.body, Body::Embodied)
    }

    /// Whether this gnome can afford `gin`.
    fn can_afford(&self, gin: Scalar) -> bool {
        self.gin >= gin
    }
}

/// A sanctioned magical shortcut for moving matter: whatever sits at
/// `input` is exchanged with whatever sits at `output`, however far apart
/// and whatever lies between.
///
/// This is the deliberate answer to ONI's liquid-in-pipes problem. That
/// game moves liquid uphill through a layer that isn't part of the physical
/// world while presenting it as plumbing; the note this is built from
/// objects to the pretence, not to the shortcut. So the shortcut is kept
/// and labelled: it is magic, it costs Gin per transfer, and — because it
/// is a swap rather than a delete-and-create — it cannot break
/// conservation even though it ignores geometry.
#[derive(Debug, Clone, Copy)]
pub struct EtherealPipe {
    pub input: GridIndex,
    pub output: GridIndex,
    /// Gin per cell moved.
    pub gin_per_transfer: Scalar,
    /// Steps between transfers: 1 moves a cell every step. A pipe is gnome
    /// infrastructure, and a gnome can build a trickle as easily as a
    /// torrent — a whole cell of water a step is twenty grams a second, a
    /// flood in a jar this size.
    pub period: u32,
}

impl EtherealPipe {
    pub fn new(input: GridIndex, output: GridIndex) -> Self {
        EtherealPipe {
            input,
            output,
            gin_per_transfer: 0.5,
            period: 1,
        }
    }

    /// Sets [`EtherealPipe::period`], builder-style.
    pub fn every(mut self, steps: u32) -> Self {
        self.period = steps.max(1);
        self
    }
}

/// The gnomes, their pipes, and the shared behaviour that drives them.
pub struct Colony {
    pub gnomes: Vec<Gnome>,
    pub pipes: Vec<EtherealPipe>,
    /// What the player has asked for — see [`crate::order`]. A colony with
    /// an empty queue is the colony every night before this one had: it
    /// forages, gardens and survives on its own. Everything in here is
    /// somebody outside the world having asked.
    pub orders: Orders,
    /// Updates run so far, for pipes that only fire every so often.
    tick: u64,
    /// The tick the colony last put a cutting in, or `None` if it never has
    /// — see [`PLANT_PERIOD`].
    last_plant: Option<u64>,
    /// Reusable route buffers — see [`crate::path`]. Held here so that
    /// flooding the world once or twice per gnome per step costs no
    /// allocation: `routes` is what is open, `crops` is what would be open
    /// to a gnome willing to cut its way out.
    routes: Routes,
    crops: Routes,
    /// Grams of food the colony has burned since it started — the other
    /// half of the jar's carbon books, beside [`crate::life::Tally`]. What
    /// a gnome breathes out is water and carbon dioxide in the same declared
    /// proportions a plant took them in at, so this one number is what makes
    /// a scenario-level water or carbon balance checkable at all.
    respired_g: f64,
}

impl Colony {
    pub fn new(gnomes: Vec<Gnome>) -> Self {
        Colony {
            gnomes,
            pipes: Vec::new(),
            orders: Orders::new(),
            tick: 0,
            last_plant: None,
            routes: Routes::default(),
            crops: Routes::default(),
            respired_g: 0.0,
        }
    }

    pub fn with_pipe(mut self, pipe: EtherealPipe) -> Self {
        self.pipes.push(pipe);
        self
    }

    /// How much Gin the colony holds in total — the resource the whole
    /// design hangs on, so worth reading directly.
    pub fn total_gin(&self) -> f64 {
        self.gnomes.iter().map(|g| g.gin).sum()
    }

    /// Grams of food this colony has breathed out — see
    /// [`Colony::respired_g`].
    pub fn respired_g(&self) -> f64 {
        self.respired_g
    }

    /// Steps of held breath left in the worst-off embodied gnome — full
    /// ([`BREATH_STEPS`]) when everybody is breathing freely, zero when
    /// somebody is out of air and about to pay for some. The colony's own
    /// summary of whether the jar is still liveable *where anyone actually
    /// stands*, as opposed to in its thinnest forgotten pocket.
    pub fn min_breath(&self) -> u32 {
        self.gnomes
            .iter()
            .filter(|g| g.is_embodied())
            .map(|g| g.breath)
            .min()
            .unwrap_or(0)
    }

    pub fn embodied_count(&self) -> usize {
        self.gnomes.iter().filter(|g| g.is_embodied()).count()
    }

    pub fn ethereal_count(&self) -> usize {
        self.gnomes.len() - self.embodied_count()
    }

    /// Writes an order on a cell — the whole of the player's reach into
    /// this world. Returns its id.
    pub fn order(&mut self, at: GridIndex, job: Job) -> u64 {
        self.orders.issue(at, job)
    }

    /// Rubs out the order on a cell, and frees whoever was walking to it.
    pub fn cancel_order(&mut self, at: GridIndex) -> bool {
        let id = self.orders.at(at).map(|o| o.id);
        let gone = self.orders.cancel_at(at);
        if let Some(id) = id {
            for g in self.gnomes.iter_mut() {
                if g.job == Some(id) {
                    g.job = None;
                }
            }
        }
        gone
    }

    /// Grams the colony is holding in its hands — mass that has left the
    /// world and is waiting to be put back. The ledger holds exactly this
    /// much against it, which is what makes a half-finished wall honest
    /// rather than a leak.
    pub fn carried_g(&self) -> f64 {
        self.gnomes
            .iter()
            .filter_map(|g| g.hands.map(|l| l.grams))
            .sum()
    }

    /// One step of gnome behaviour, run after the physics step.
    pub fn update(&mut self, world: &mut World) {
        self.tick = self.tick.wrapping_add(1);
        self.run_pipes(world);
        // Taken out and put back so that a gnome's own routes can be
        // flooded while the colony it belongs to is borrowed mutably. The
        // buffers survive the round trip, which is the point of holding
        // them on the colony at all.
        let mut routes = std::mem::take(&mut self.routes);
        let mut crops = std::mem::take(&mut self.crops);
        self.orders.begin_survey();
        for idx in 0..self.gnomes.len() {
            match self.gnomes[idx].body {
                Body::Embodied => self.update_embodied(world, &mut routes, &mut crops, idx),
                Body::Ethereal { .. } => self.update_ethereal(idx),
            }
        }
        self.orders.end_survey();
        self.routes = routes;
        self.crops = crops;
    }

    /// Ethereal pipes move one cell of matter per step, paid for out of the
    /// nearest gnome's flask. No gnome with Gin to spare, no transfer —
    /// the pipes are gnome infrastructure, not free machinery.
    fn run_pipes(&mut self, world: &mut World) {
        for pipe_idx in 0..self.pipes.len() {
            let pipe = self.pipes[pipe_idx];
            if !self.tick.is_multiple_of(pipe.period as u64) {
                continue;
            }
            if !world.in_bounds(pipe.input) || !world.in_bounds(pipe.output) {
                continue;
            }
            let src = world.cell(pipe.input);
            let dst = world.cell(pipe.output);
            let src_mat = world.materials().get(src.material);
            let dst_mat = world.materials().get(dst.material);
            // Only carries fluids, and only when there is something lighter
            // at the far end to displace — otherwise the pipe is full.
            if src_mat.mobility == Mobility::Static || src_mat.phase == Phase::Gas {
                continue;
            }
            if dst_mat.mobility == Mobility::Static || dst_mat.density >= src_mat.density {
                continue;
            }
            let Some(payer) = self.nearest_payer(pipe.input, pipe.gin_per_transfer) else {
                continue;
            };
            self.gnomes[payer].gin -= pipe.gin_per_transfer;
            world.charge_gin(pipe.gin_per_transfer);
            let a = world.linear_index(pipe.input);
            let b = world.linear_index(pipe.output);
            world.swap_cells(a, b);
        }
    }

    /// The embodied gnome closest to `at` that can afford `gin`.
    fn nearest_payer(&self, at: GridIndex, gin: Scalar) -> Option<usize> {
        self.gnomes
            .iter()
            .enumerate()
            .filter(|(_, g)| g.is_embodied() && g.can_afford(gin))
            .min_by_key(|(_, g)| (g.pos.i - at.i).abs() + (g.pos.j - at.j).abs())
            .map(|(i, _)| i)
    }

    fn update_ethereal(&mut self, idx: usize) {
        let anchor = self.gnomes[idx].anchor;
        let helper_near = self.gnomes.iter().enumerate().any(|(other, g)| {
            other != idx
                && g.is_embodied()
                && (g.pos.i - anchor.i).abs() <= RESCUE_RADIUS
                && (g.pos.j - anchor.j).abs() <= RESCUE_RADIUS
        });
        let Body::Ethereal { rescue_progress } = self.gnomes[idx].body else {
            return;
        };
        if helper_near {
            let progress = rescue_progress + 1;
            if progress >= RESCUE_STEPS {
                let g = &mut self.gnomes[idx];
                g.body = Body::Embodied;
                g.breath = BREATH_STEPS;
                g.gin = (g.gin + GIN_ON_RESCUE).min(MAX_GIN);
                g.pos = g.anchor;
                g.last_act = Act::Rescued;
            } else {
                self.gnomes[idx].body = Body::Ethereal {
                    rescue_progress: progress,
                };
                self.gnomes[idx].last_act = Act::Waiting;
            }
        } else {
            self.gnomes[idx].last_act = Act::Waiting;
        }
    }

    fn update_embodied(
        &mut self,
        world: &mut World,
        routes: &mut Routes,
        crops: &mut Routes,
        idx: usize,
    ) {
        let pos = self.gnomes[idx].pos;
        if !world.in_bounds(pos) {
            return;
        }
        let cell = world.cell(pos);
        let here = *world.materials().get(cell.material);

        // --- Breathing ---
        //
        // A cell that is mostly empty is breathable whatever it is labelled.
        // Liquid cells can now be a raindrop's worth of water — a hundredth
        // of a cell — and a gnome that one landed on used to count as
        // submerged, run out of breath, and pay Gin to conjure away a drop
        // it could have stepped out of.
        let fill = if here.density > 0.0 {
            cell.mass / here.density
        } else {
            1.0
        };
        // ...and "mostly empty" is about how full the cell is, not about
        // which phase it is. It used to be liquids only, because until
        // something living could put a solid into a cell a grain at a time,
        // a solid cell was always a full one. Now a bush sheds dead leaves
        // and a mould grows across a floor, and a gnome standing in the first
        // few grains of either was being treated as buried alive: it gasped,
        // paid Gin for air it was already standing in, and the colony bled
        // its whole flask into a drift of leaves.
        let mostly_empty = here.phase != Phase::Gas && fill < 0.5;
        let air_here = cell.breathable_pressure(world.materials())
            >= MIN_BREATHABLE * world.materials().reference_pressure();
        if air_here || mostly_empty {
            self.gnomes[idx].breath = BREATH_STEPS;
        } else if self.gnomes[idx].breath > 0 {
            self.gnomes[idx].breath -= 1;
        }
        self.respire(world, idx, pos);

        // --- Lethal heat or cold: mitigate with Gin, or leave the world ---
        let too_hot = cell.temperature > LETHAL_MAX;
        let too_cold = cell.temperature < LETHAL_MIN;
        let drowning = self.gnomes[idx].breath == 0;

        if too_hot || too_cold {
            let target = if too_hot { COMFORT_MAX } else { COMFORT_MIN };
            let capacity = cell.capacity(world.materials());
            let joules = (target - cell.temperature) * capacity;
            let cost = (joules.abs() * GIN_PER_JOULE) as Scalar;
            if self.gnomes[idx].can_afford(cost) {
                self.spend(world, idx, cost);
                world.conjure_energy(pos, joules);
                self.gnomes[idx].last_act = if too_hot { Act::Chilled } else { Act::Warmed };
                return;
            }
            self.go_ethereal(idx);
            return;
        }

        if drowning {
            // Submerged in the drink is a different problem from submerged
            // in the pool: drinking the cell you are *standing in* both
            // refills the flask and clears the obstruction, and costs
            // nothing. Tried before the gasp so a gnome up to its neck in
            // gin does the obvious thing.
            //
            // Strictly its own cell, and not the wider reach a thirsty
            // gnome has: a gnome that drank from the puddle beside it every
            // time it ran out of breath emptied a working still as fast as
            // the still could fill it, flask long since full, purely
            // because it happened to be standing in the receiver.
            if self.drink(world, idx, pos, cell.temperature) {
                return;
            }
            // Then: step out of it. Magic is the *last* resort, not the
            // first, and a gnome with a breathable cell next to it has no
            // business paying for one. Buried by a drift of litter, by
            // spoil falling off a dig, or by a rising pool, the answer is
            // usually one pace sideways — and a colony that takes it keeps
            // the Gin it would otherwise have spent gasping.
            if let Some(out) = self.way_out(world, pos) {
                self.gnomes[idx].pos = out;
                self.gnomes[idx].last_act = Act::Walked;
                return;
            }
            // Still stuck. If what is holding it down is something that
            // grows, cut that: a bush costs nothing to cut and a gasp costs
            // Gin, and magic is meant to be the last resort rather than the
            // second.
            if let Some(crop) = self
                .reachable_cells(pos)
                .into_iter()
                .skip(1)
                .find(|&n| path::harvestable(world, n))
            {
                if self.heave(world, idx, crop)
                    || self.shove(world, idx, crop, pos)
                    || self.harvest(world, idx, crop)
                {
                    return;
                }
            }
            // A cheap Gin-powered gasp: replace the cell you are stuck in
            // with breathable air. Costs matter, so it is not free.
            // Priced by what it actually displaces, not by what it makes.
            // A gasp replaces the cell a gnome is stuck in, so drowning in a
            // pool banishes a whole gram of water through the ledger; when
            // that was charged as if it only conjured a milligram of gas, a
            // colony that fell in the water drained the pool it fell into,
            // fifty grams at a time, and never ran out of Gin doing it.
            let cost = (cell.mass + world.materials().get(t::OXYGEN).density) * GIN_PER_GRAM + 4.0;
            if self.gnomes[idx].can_afford(cost) {
                self.spend(world, idx, cost);
                // Oxygen, not "air": a bubble of the inert bulk of the
                // atmosphere would be no help at all now that breathing
                // reads what is actually in a cell.
                let gasp = world.materials().get(t::OXYGEN).density;
                world.conjure_mass(pos, t::OXYGEN, gasp, cell.temperature);
                self.gnomes[idx].breath = BREATH_STEPS;
                self.gnomes[idx].last_act = Act::Breathed;
                return;
            }
            self.go_ethereal(idx);
            return;
        }

        // --- Drink, or forage, when the flask or the belly runs low ---
        let hungry = self.gnomes[idx].gin < GIN_HUNGRY || self.gnomes[idx].belly < BELLY_HUNGRY;
        if hungry && self.drink_or_forage(world, idx, pos, cell.temperature) {
            return;
        }

        // Everything from here on is about somewhere else, so this is where
        // the gnome works out where it can get to. One flood per gnome per
        // step, reused by the order it takes on and by the walk it makes
        // toward it, so the two cannot disagree about what is reachable.
        routes.explore_avoiding(world, pos, Through::Open, |c| dangerous(world, c));
        // ...and reports what it can see of the queue, so that an order the
        // whole colony is walled off from ages out instead of sitting there
        // unclaimed for ever — see [`Orders::end_survey`].
        self.orders.survey(|at| Self::walk_steps(routes, at));

        // --- The player's orders, ahead of the colony's own gardening ---
        //
        // Behind survival and behind lunch, though. A colony that digs
        // while it suffocates is a colony tuned toward failure, which is
        // the thing `NORTH_STARS.md` #4 opens by objecting to.
        if self.work(world, routes, idx, pos) {
            return;
        }

        // --- Garden, when there is food to spare and a bush to train ---
        if self.plant(world, idx, pos) {
            return;
        }

        // --- Otherwise: fall, or go where you are needed ---
        let below = GridIndex::new(pos.i, pos.j - 1);
        if path::passable(world, below) {
            self.gnomes[idx].pos = below;
            self.gnomes[idx].last_act = Act::Fell;
            return;
        }

        self.travel(world, routes, crops, idx, pos);
    }

    /// Takes this gnome one step toward whatever it is currently for.
    ///
    /// The whole of tonight's change is here. What used to happen was:
    /// find the nearest larder by straight-line distance, take the *sign*
    /// of the difference in column, and try to walk that way — which is
    /// why four gnomes spent seventy thousand steps pressed against a hedge
    /// with the garden on the other side of it, never once trying the clear
    /// route over the top, because the garden was still west.
    ///
    /// Now: flood the world with routes over the moves a gnome can really
    /// make ([`crate::path`]), pick the nearest goal *it can actually get
    /// to*, and take the first step of the route. A goal nothing can reach
    /// is not chosen at all, so a gnome walled in by the hedge it planted
    /// goes and does the next thing on its list instead of starving in
    /// front of a wall.
    fn travel(
        &mut self,
        world: &mut World,
        routes: &mut Routes,
        crops: &mut Routes,
        idx: usize,
        pos: GridIndex,
    ) {
        if let Some(goal) = self.goal(world, routes, idx) {
            if let Some(next) = routes.next_step(goal) {
                self.gnomes[idx].stuck = 0;
                self.step_to(idx, next, pos);
                return;
            }
        }

        // Nothing open. A gnome that has had nowhere to go for a good while
        // may move a crop out of its way — lifting it if it can
        // ([`Colony::heave`]), shoving it along if it cannot, and cutting it
        // down only if neither works — and only ever to get *out*.
        //
        // Every clause there was learned the hard way. Letting any gnome
        // that could not reach food cut, at once, made a lawnmower rather
        // than a hedge-trimmer: every bush near a hungry colony has been
        // picked below a berry, so each gnome cut its way toward the next
        // bush that had one, and 2.5 g of garden became 1.9 g of leaf
        // litter inside fifteen hundred steps. And cutting *toward lunch*
        // from inside the hole a player dug into the garden chews through
        // the whole plot on the way.
        //
        // So: wait [`TRAPPED_STEPS`], then head for the nearest cell outside
        // the little world this gnome is currently confined to, moving
        // whatever is in the way as gently as it can. The counter resets the
        // moment anything works — walking to a job, reaching a bush, getting
        // out.
        //
        // The size of that little world is deliberately *not* a condition.
        // It was, and four gnomes spent forty thousand steps pacing a
        // five-cell pocket with the garden two cells away behind a bush,
        // because five was one more than the threshold. Being unable to get
        // anywhere you need to be for two minutes is what "walled in"
        // means; how big the pen is has nothing to do with it.
        self.gnomes[idx].stuck += 1;
        if self.gnomes[idx].stuck < TRAPPED_STEPS {
            self.pace(world, idx, pos);
            return;
        }
        crops.explore_avoiding(world, pos, Through::Crops, |c| dangerous(world, c));
        let out = crops.nearest(|c| routes.steps_to(c).is_none());
        if let Some(next) = out.and_then(|o| crops.next_step(o)) {
            if path::harvestable(world, next) && !path::steppable(world, next) {
                if self.heave(world, idx, next)
                    || self.shove(world, idx, next, pos)
                    || self.harvest(world, idx, next)
                {
                    self.gnomes[idx].stuck = 0;
                    return;
                }
            } else {
                self.step_to(idx, next, pos);
                return;
            }
        }

        self.pace(world, idx, pos);
    }

    /// Nowhere to be: pace. Keeps a colony with a full belly and a full
    /// flask from standing in one place, which is also what spreads them out
    /// enough to find each other's anchors.
    fn pace(&mut self, world: &World, idx: usize, pos: GridIndex) {
        let dir = self.gnomes[idx].facing;
        for way in [dir, -dir] {
            for to in [
                GridIndex::new(pos.i + way as i32, pos.j),
                GridIndex::new(pos.i + way as i32, pos.j + 1),
            ] {
                if path::steppable_from(world, to, pos) && !dangerous(world, to) {
                    self.gnomes[idx].facing = way;
                    self.step_to(idx, to, pos);
                    return;
                }
            }
        }
        // Boxed in on both sides: chimney out, the same move a route would
        // use. Without it a gnome with nowhere in particular to be sits in
        // the first hole it falls into for ever — which, since night 6, is
        // most often the hole it dug itself on the player's orders.
        let above = GridIndex::new(pos.i, pos.j + 1);
        if path::steppable_from(world, above, pos) && !dangerous(world, above) {
            self.step_to(idx, above, pos);
            return;
        }
        self.gnomes[idx].last_act = Act::Idle;
    }

    /// Moves this gnome into `next`, which the route says it can be in.
    fn step_to(&mut self, idx: usize, next: GridIndex, pos: GridIndex) {
        if next.i != pos.i {
            self.gnomes[idx].facing = (next.i - pos.i).signum() as i8;
        }
        self.gnomes[idx].pos = next;
        self.gnomes[idx].last_act = if next.j < pos.j {
            Act::Fell
        } else {
            Act::Walked
        };
    }

    /// Where this gnome is trying to get to, given what it can reach: the
    /// nearest cell it could *stand in* and do the thing it currently
    /// wants, or `None` if there is nothing to go to.
    ///
    /// The order is the colony's priorities, and it is the same order the
    /// old sign-of-the-difference steering used: a stranded comrade first,
    /// then the player's orders, then lunch, then gardening. What is new is
    /// that every one of them is filtered by whether a route exists, so an
    /// unreachable bush no longer outranks a reachable job.
    fn goal(&self, world: &World, routes: &Routes, idx: usize) -> Option<GridIndex> {
        let g = self.gnomes[idx];
        // A stranded comrade outranks everything else. Rescue is the whole
        // point of the ethereal layer: without someone walking over, an
        // ethereal gnome is just a slower death.
        if let Some(anchor) = self.nearest_anchor(idx) {
            if let Some(at) = routes.nearest(|c| {
                (c.i - anchor.i).abs() <= RESCUE_RADIUS && (c.j - anchor.j).abs() <= RESCUE_RADIUS
            }) {
                return Some(at);
            }
        }
        let hungry = g.gin < GIN_HUNGRY || g.belly < BELLY_HUNGRY;
        // Toward the job it has taken on — but a *hungry* gnome goes to the
        // larder first, because the alternative is a colony that works
        // itself into the ethereal layer on request.
        if !hungry {
            if let Some(job) = g.job.and_then(|id| self.orders.get(id)) {
                if let Some(at) = Self::workplace(routes, job.at) {
                    return Some(at);
                }
            }
        }
        if hungry {
            if let Some(at) =
                routes.nearest(|c| !self.crowded(idx, c) && self.can_feed_at(world, c))
            {
                return Some(at);
            }
        }
        // A gnome with food to spare is a gardener.
        if g.belly >= BELLY_TO_PLANT && self.may_plant() {
            if let Some(at) =
                routes.nearest(|c| !self.crowded(idx, c) && self.beside_crop(world, c))
            {
                return Some(at);
            }
        }
        None
    }

    /// Whether somebody else is already standing there.
    ///
    /// Four identical gnomes with identical appetites and one shared idea of
    /// the nearest bush walk in lockstep, arrive as a stack of four in one
    /// cell, and pick the same bush bare while the next one along is
    /// untouched. Taking the *nearest unoccupied* place to eat instead
    /// spreads them over the garden with no scheduler, no roles and no
    /// randomness — and it is also what anybody queuing for lunch does.
    fn crowded(&self, idx: usize, at: GridIndex) -> bool {
        self.gnomes
            .iter()
            .enumerate()
            .any(|(other, g)| other != idx && g.is_embodied() && g.pos == at)
    }

    /// The nearest cell this gnome can reach and work on `at` from — a
    /// place to stand within [`crate::order::REACH`] of the job.
    fn workplace(routes: &Routes, at: GridIndex) -> Option<GridIndex> {
        let reach = crate::order::REACH;
        (-reach..=reach)
            .flat_map(|di| (-reach..=reach).map(move |dj| GridIndex::new(at.i + di, at.j + dj)))
            .filter_map(|c| routes.steps_to(c).map(|d| (d, c.j, c.i)))
            .min()
            .map(|(_, j, i)| GridIndex::new(i, j))
    }

    /// Paces from here to somewhere `at` can be worked on, or `None` if
    /// there is no such place this gnome can get to.
    fn walk_steps(routes: &Routes, at: GridIndex) -> Option<u32> {
        Self::workplace(routes, at).and_then(|c| routes.steps_to(c))
    }

    /// Whether a gnome standing at `at` could eat: a cell of drink within
    /// reach, or a bush next to it. The reachability half of
    /// [`Colony::drink_or_forage`], and deliberately the same tests, so a
    /// gnome never walks to a larder it will then decline to eat from.
    fn can_feed_at(&self, world: &World, at: GridIndex) -> bool {
        self.drink_within_reach(world, at).is_some() || self.forage_beside(world, at).is_some()
    }

    /// Whether there is a crop within planting reach of `at`.
    fn beside_crop(&self, world: &World, at: GridIndex) -> bool {
        (-2..=2).any(|di| {
            (-1..=1).any(|dj| {
                let n = GridIndex::new(at.i + di, at.j + dj);
                world.in_bounds(n) && world.materials().grows(world.material_at(n))
            })
        })
    }

    /// Lifts the crop at `at` — and everything growing on top of it — up one
    /// course, so the way along opens underneath.
    ///
    /// This is the *first* thing a gnome shut in by the garden tries, and it
    /// is the one that costs nothing: the cells are swapped, so every bush
    /// keeps every gram and every joule it had and the ledger has nothing to
    /// say about it. A hedge that has grown across the walkway becomes a
    /// canopy over it, which is exactly where night 5's planting rule puts a
    /// cutting by hand and for the same reason — "planted overhead it is a
    /// canopy, and the gnomes keep their path".
    ///
    /// It lifts the whole column because a bush in a garden usually has
    /// another bush on top of it: looking only one cell up, this worked once
    /// in twenty thousand steps and the colony cut five bushes down instead,
    /// which in a five-cell garden is the garden.
    ///
    /// Cutting one down ([`Colony::harvest`]) is the fallback for a crop
    /// with something that does not grow sitting on it.
    fn heave(&mut self, world: &mut World, idx: usize, at: GridIndex) -> bool {
        // Find the gas cell the column can be pushed into, through crops
        // only: a bush under a rock does not lift.
        let mut top = at;
        loop {
            let next = GridIndex::new(top.i, top.j + 1);
            if !world.in_bounds(next) || next.j - at.j > LIFT_MAX {
                return false;
            }
            if world.cell(next).is_gas(world.materials()) {
                top = next;
                break;
            }
            if !path::harvestable(world, next) {
                return false;
            }
            top = next;
        }
        let mut j = top.j;
        while j > at.j {
            let (a, b) = (
                world.linear_index(GridIndex::new(at.i, j)),
                world.linear_index(GridIndex::new(at.i, j - 1)),
            );
            world.swap_cells(a, b);
            j -= 1;
        }
        self.gnomes[idx].last_act = Act::Heaved;
        true
    }

    /// Shoves the crop at `at` one cell further along, the way the gnome
    /// walking into it is going — the other way to clear a hedge that has
    /// something solid sitting on top of it, and the same swap, so it costs
    /// the garden nothing either.
    fn shove(&mut self, world: &mut World, idx: usize, at: GridIndex, from: GridIndex) -> bool {
        let on = GridIndex::new(at.i + (at.i - from.i), at.j + (at.j - from.j));
        if !world.in_bounds(on) || !world.cell(on).is_gas(world.materials()) {
            return false;
        }
        let (a, b) = (world.linear_index(at), world.linear_index(on));
        world.swap_cells(a, b);
        self.gnomes[idx].last_act = Act::Heaved;
        true
    }

    /// Cuts the crop at `at` down, leaving what the table says a cut
    /// one leaves — see [`MaterialTable::shed_form`].
    ///
    /// Conserving by construction, and in the strict sense rather than the
    /// approximate one: the cell keeps every gram it had, and its
    /// temperature afterwards is *solved* from the energy it had before, so
    /// whatever the two materials' enthalpies differ by comes out as the
    /// cut cell being a little warmer or cooler and cannot come out as
    /// energy. Nothing goes on the ledger because nothing left the world.
    ///
    /// What it gives the gnome is a berry, on the way past — pruning is
    /// work, and this is the wage.
    ///
    /// [`MaterialTable::shed_form`]: crate::material::MaterialTable::shed_form
    fn harvest(&mut self, world: &mut World, idx: usize, at: GridIndex) -> bool {
        let cell = world.cell(at);
        let Some(cut) = world.materials().shed_form(cell.material) else {
            return false;
        };
        let temperature = cell.temperature;
        self.forage_from(world, idx, at, temperature);
        let cell = world.cell(at);
        if cell.mass <= 0.0 {
            return false;
        }
        let energy = cell.energy(world.materials());
        let mut left = crate::world::Cell {
            material: cut,
            ..cell
        };
        left.solve_temperature(world.materials(), energy);
        world.set_cell(at, left);
        self.gnomes[idx].last_act = Act::Harvested;
        true
    }

    /// Burns a little of what this gnome has eaten, against the oxygen in
    /// the cell it is standing in, and breathes out carbon dioxide and water
    /// vapour — the other half of the jar's carbon cycle, and the reason a
    /// sealed room full of gnomes eventually runs out of air.
    ///
    /// The proportions are read off the material table's own photosynthesis
    /// row (see [`respiration_ratios`]), so a gnome undoes precisely what a
    /// bush did, gram for gram, and the carbon in a jar goes round rather
    /// than accumulating anywhere. Everything moves through
    /// [`World::conjure_mass`]: what the gnome eats leaves the world's
    /// books and what it exhales comes back, so a gnome part-way through
    /// digesting a berry shows up as a small non-zero ledger entry and a
    /// gnome that has finished shows up as none.
    ///
    /// Exhaled at [`BODY_K`] rather than at ambient, which is the whole of
    /// how a gnome warms the room it is in.
    fn respire(&mut self, world: &mut World, idx: usize, pos: GridIndex) {
        let belly = self.gnomes[idx].belly;
        if belly <= 0.0 {
            return;
        }
        let Some((o2_per_g, co2_per_g, h2o_per_g)) = respiration_ratios(world) else {
            return;
        };
        let cell = world.cell(pos);
        if !cell.is_gas(world.materials()) {
            return;
        }
        let oxygen = cell.grams_of(world.materials(), t::OXYGEN);
        // Half the oxygen in the cell at most, so a gnome cannot strip its
        // own cell bare in one step and suffocate itself instantly.
        let burn = belly
            .min(RESPIRATION_G)
            .min(0.5 * oxygen / o2_per_g.max(1e-12));
        if burn <= 0.0 {
            return;
        }
        world.conjure_mass(pos, t::OXYGEN, -burn * o2_per_g, cell.temperature);
        world.conjure_mass(pos, t::CO2, burn * co2_per_g, BODY_K);
        world.conjure_mass(pos, t::STEAM, burn * h2o_per_g, BODY_K);
        self.gnomes[idx].belly -= burn;
        self.respired_g += burn;
    }

    /// Takes on, walks toward, and carries out the player's orders.
    ///
    /// Returns whether this gnome's step was spent working — walking toward
    /// a job is *not* working in that sense, because the walk itself is the
    /// ordinary movement code with a destination; this returns false and
    /// lets [`Colony::walk_direction`] steer.
    ///
    /// [`Colony::walk_direction`]: Colony::walk_direction
    fn work(&mut self, world: &mut World, routes: &Routes, idx: usize, pos: GridIndex) -> bool {
        // Claim something, if this gnome is free and there is anything it
        // could usefully do with the hands it has — and that it can get to.
        if self.gnomes[idx].job.is_none() {
            let load = self.gnomes[idx].hands;
            self.gnomes[idx].job = self
                .orders
                .claim_for(idx, load.as_ref(), |at| Self::walk_steps(routes, at));
        }
        let Some(id) = self.gnomes[idx].job else {
            return false;
        };
        let Some(order) = self.orders.get(id) else {
            // Somebody rubbed it out while we were walking over.
            self.gnomes[idx].job = None;
            return false;
        };
        // Hands that no longer suit the job — it was dug out from under us,
        // or we picked something up on the way. Hand it back.
        if !order.job.suits(self.gnomes[idx].hands.as_ref()) {
            self.orders.release(id);
            self.gnomes[idx].job = None;
            return false;
        }
        if !crate::order::within_reach(pos, order.at) {
            if self.orders.tick_claim(id) {
                self.orders.release(id);
                self.gnomes[idx].job = None;
            }
            return false;
        }
        // Never build on top of somebody. A gnome inside a cell of sand
        // cannot breathe, and would pay Gin to gasp its way out of a wall
        // the player's own colony put there — a colony tuned toward failure
        // by way of its own diligence. It waits for the cell to clear.
        if order.job == Job::Build
            && self
                .gnomes
                .iter()
                .any(|g| g.is_embodied() && g.pos == order.at)
        {
            if self.orders.tick_claim(id) {
                self.orders.release(id);
                self.gnomes[idx].job = None;
            }
            return false;
        }
        let mut hands = self.gnomes[idx].hands;
        let attempt = crate::order::perform(world, &order, &mut hands, self.gnomes[idx].gin);
        self.gnomes[idx].hands = hands;
        if attempt.gin > 0.0 {
            self.spend(world, idx, attempt.gin);
        }
        match attempt.outcome {
            Outcome::Dug => {
                self.orders.complete(id);
                self.gnomes[idx].job = None;
                self.gnomes[idx].last_act = Act::Dug;
                true
            }
            Outcome::Built => {
                self.orders.complete(id);
                self.gnomes[idx].job = None;
                self.gnomes[idx].last_act = Act::Built;
                true
            }
            Outcome::Tempered => {
                self.orders.complete(id);
                self.gnomes[idx].job = None;
                self.gnomes[idx].last_act = Act::Tempered;
                true
            }
            Outcome::Tempering => {
                self.gnomes[idx].last_act = Act::Tempered;
                true
            }
            Outcome::Impossible => {
                self.orders.cancel(id);
                self.gnomes[idx].job = None;
                false
            }
            Outcome::NotYet => {
                if self.orders.tick_claim(id) {
                    self.orders.release(id);
                    self.gnomes[idx].job = None;
                }
                false
            }
        }
    }

    /// Whether enough updates have passed since the last cutting went in.
    fn may_plant(&self) -> bool {
        self.last_plant
            .is_none_or(|last| self.tick >= last + PLANT_PERIOD)
    }

    /// Puts a cutting in, directly above a bush the gnome is standing beside
    /// — see [`CUTTING_G`]. Returns whether anything was planted.
    fn plant(&mut self, world: &mut World, idx: usize, pos: GridIndex) -> bool {
        if self.gnomes[idx].belly < BELLY_TO_PLANT || !self.may_plant() {
            return false;
        }
        // Any bush within a couple of paces — a gnome plants with a tool at
        // arm's length, not by pressing its nose against the plant.
        let mut bushes: Vec<GridIndex> = Vec::new();
        for di in -2..=2 {
            for dj in -1..=1 {
                let at = GridIndex::new(pos.i + di, pos.j + dj);
                if world.in_bounds(at) && world.material_at(at) == t::JUNIPER {
                    bushes.push(at);
                }
            }
        }
        bushes.sort_by_key(|b| (b.i - pos.i).abs() + (b.j - pos.j).abs());
        if bushes.is_empty() {
            return false;
        }
        // Somewhere in the two courses above that bush, within arm's reach:
        // strictly higher than the gnome's own feet, at most two above them,
        // empty, and with something for a cutting to root in underneath.
        //
        // Never on the gnome's own row, and that is the whole of what keeps
        // a colony from walling itself in: juniper is a static solid, a gnome
        // will not walk into one and can only climb a single course, so a
        // hedge planted along the walkway is a fence. Planted overhead it is
        // a canopy, and the gnomes keep their path.
        let mut chosen = None;
        'search: for (bush, dj) in bushes.iter().flat_map(|&b| (1..=2).map(move |d| (b, d))) {
            let j = bush.j + dj;
            if j <= pos.j || j > pos.j + 2 {
                continue;
            }
            for di in [0, -1, 1] {
                let at = GridIndex::new(bush.i + di, j);
                let under = GridIndex::new(at.i, at.j - 1);
                if !world.in_bounds(at) || !world.in_bounds(under) {
                    continue;
                }
                if !world.cell(at).is_gas(world.materials()) {
                    continue;
                }
                let below = world.materials().get(world.material_at(under));
                let rooted = below.mobility == Mobility::Static
                    || below.phase == Phase::Solid
                    || below.phase == Phase::Granular;
                if rooted {
                    chosen = Some(at);
                    break 'search;
                }
            }
        }
        let Some(above) = chosen else {
            return false;
        };
        let cell = world.cell(above);
        // Push the air (and whatever it is carrying) into a neighbour rather
        // than letting the cutting overwrite it — see `gas::displace`.
        let p = world.linear_index(above);
        if !crate::gas::displace(world, p) {
            return false;
        }
        world.conjure_mass(above, t::JUNIPER, CUTTING_G, cell.temperature);
        self.last_plant = Some(self.tick);
        let g = &mut self.gnomes[idx];
        g.belly -= CUTTING_G;
        g.last_act = Act::Planted;
        true
    }

    /// Refills the flask from whatever is within reach: a cell of gin
    /// first, a juniper bush second. Returns whether anything was drunk or
    /// eaten.
    ///
    /// Both take matter out of the world and both are booked through
    /// [`World::conjure_mass`], for the same reason: a gnome's stomach is
    /// outside the simulation, so what goes into it has left, and the
    /// ledger says by how much. Nothing here is free and nothing here is
    /// unaccounted.
    fn drink_or_forage(
        &mut self,
        world: &mut World,
        idx: usize,
        pos: GridIndex,
        temperature: Scalar,
    ) -> bool {
        // A drink can be in the cell the gnome is standing in — it pools on
        // the floor, which is where a gnome stands — so its own cell is part
        // of the search, unlike a bush, which it has to stand next to.
        if let Some(cup) = self.drink_within_reach(world, pos) {
            if self.drink(world, idx, cup, temperature) {
                return true;
            }
        }
        match self.forage_beside(world, pos) {
            Some(bush) => self.forage_from(world, idx, bush, temperature),
            None => false,
        }
    }

    /// A cell of drink — a liquid the table calls food — in this gnome's
    /// own cell or one of its four neighbours.
    fn drink_within_reach(&self, world: &World, pos: GridIndex) -> Option<GridIndex> {
        self.reachable_cells(pos)
            .into_iter()
            .find(|&n| {
                let m = world.materials().get(world.material_at(n));
                world.in_bounds(n) && m.is_food() && m.phase == Phase::Liquid
            })
            .filter(|&n| world.cell(n).mass > 0.0)
    }

    /// A bush beside this gnome with a berry to spare. Never its own cell:
    /// a solid food is something you stand next to, not in.
    fn forage_beside(&self, world: &World, pos: GridIndex) -> Option<GridIndex> {
        self.reachable_cells(pos)
            .into_iter()
            .skip(1)
            .find(|&n| world.in_bounds(n) && self.spare_berry(world, n) > 0.0)
    }

    /// The gnome's own cell and its four neighbours, own cell first.
    fn reachable_cells(&self, pos: GridIndex) -> [GridIndex; 5] {
        [
            pos,
            GridIndex::new(pos.i + 1, pos.j),
            GridIndex::new(pos.i - 1, pos.j),
            GridIndex::new(pos.i, pos.j + 1),
            GridIndex::new(pos.i, pos.j - 1),
        ]
    }

    /// Grams of food a solid cell can spare — zero for anything that is not
    /// food, and for a bush already picked down to a quarter of itself.
    ///
    /// Never strip a bush bare: a berry is what a bush can spare, and a
    /// garden picked down to nothing cannot grow back.
    fn spare_berry(&self, world: &World, at: GridIndex) -> Scalar {
        let m = world.materials().get(world.material_at(at));
        if !m.is_food() || m.phase == Phase::Liquid || m.phase == Phase::Gas {
            return 0.0;
        }
        let spare = (world.cell(at).mass - GRAZE_FLOOR * m.density).clamp(0.0, BERRY_MASS);
        // Below a bite worth taking there is nothing here to eat *yet*, and
        // saying so is what stops a gnome grazing. A colony in a
        // picked-over garden used to stand at the barest bush in it taking
        // a microgram a step for ever — its belly flat, its flask empty,
        // technically eating — because a crumb counted as a meal and a meal
        // ended the step. A gnome that finds nothing ripe here goes and
        // looks over there.
        if spare < MIN_BITE {
            return 0.0;
        }
        spare
    }

    /// Picks a berry off the cell at `bush`.
    fn forage_from(
        &mut self,
        world: &mut World,
        idx: usize,
        bush: GridIndex,
        temperature: Scalar,
    ) -> bool {
        if self.gnomes[idx].belly >= BELLY_FULL {
            return false;
        }
        let food = world.material_at(bush);
        let worth = world.materials().get(food).nutrition;
        let spare = self.spare_berry(world, bush);
        if spare <= 0.0 {
            return false;
        }
        let picked = -world.conjure_mass(bush, food, -spare, temperature);
        if picked <= 0.0 {
            return false;
        }
        let g = &mut self.gnomes[idx];
        g.gin = (g.gin + worth * picked).min(MAX_GIN);
        g.belly += picked;
        g.last_act = Act::Foraged;
        true
    }

    /// Drinks the cell of gin at `cup`. The matter leaves the world, so it
    /// goes on the ledger like every other thing a gnome consumes.
    fn drink(
        &mut self,
        world: &mut World,
        idx: usize,
        cup: GridIndex,
        temperature: Scalar,
    ) -> bool {
        let held = world.cell(cup).mass;
        let drink = world.material_at(cup);
        let worth = world.materials().get(drink).nutrition;
        if held <= 0.0 || worth <= 0.0 {
            return false;
        }
        world.conjure_mass(cup, drink, -held, temperature);
        let g = &mut self.gnomes[idx];
        g.gin = (g.gin + held * worth).min(MAX_GIN);
        // Ethanol is biomass too: it goes into the same belly and comes
        // back out of the same lungs. Treated at juniper's proportions,
        // which is the one liberty — a gram of spirit oxidises to rather
        // more CO₂ than a gram of sugar does.
        g.belly += held;
        g.last_act = Act::Drank;
        true
    }

    /// The anchor of the closest gnome currently in the ethereal layer.
    fn nearest_anchor(&self, idx: usize) -> Option<GridIndex> {
        let from = self.gnomes[idx].pos;
        self.gnomes
            .iter()
            .enumerate()
            .filter(|(other, g)| *other != idx && !g.is_embodied())
            .min_by_key(|(_, g)| (g.anchor.i - from.i).abs() + (g.anchor.j - from.j).abs())
            .map(|(_, g)| g.anchor)
    }

    fn go_ethereal(&mut self, idx: usize) {
        // Whatever it was carrying goes with it — the load stays booked out
        // of the ledger, and comes back into the world when the gnome is
        // rescued and puts it down. Its job, though, is somebody else's now.
        self.orders.release_all_of(idx);
        let g = &mut self.gnomes[idx];
        g.job = None;
        g.anchor = g.pos;
        g.body = Body::Ethereal { rescue_progress: 0 };
        g.last_act = Act::WentEthereal;
    }

    fn spend(&mut self, world: &mut World, idx: usize, gin: Scalar) {
        self.gnomes[idx].gin -= gin;
        world.charge_gin(gin);
    }

    /// A cell beside `pos` a gnome could breathe in, for when the one it is
    /// standing in has filled up around it. Upward first — whatever buried
    /// it probably came from below or is still arriving — then across, then
    /// down.
    fn way_out(&self, world: &World, pos: GridIndex) -> Option<GridIndex> {
        let threshold = MIN_BREATHABLE * world.materials().reference_pressure();
        [
            GridIndex::new(pos.i, pos.j + 1),
            GridIndex::new(pos.i - 1, pos.j),
            GridIndex::new(pos.i + 1, pos.j),
            GridIndex::new(pos.i, pos.j - 1),
        ]
        .into_iter()
        .find(|&n| {
            world.in_bounds(n)
                && path::steppable_from(world, n, pos)
                && world.cell(n).breathable_pressure(world.materials()) >= threshold
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::MaterialTable;
    use crate::physics;

    fn cavern() -> World {
        let mut w = World::new_open(20, 12, MaterialTable::terrarium(), 293.0);
        for i in 0..20 {
            w.fill(GridIndex::new(i, 0), t::STONE, 293.0);
        }
        w.rebaseline();
        w
    }

    /// Scenario: the larder is to the west, and the only way to it is east,
    /// up a step and back along a shelf. The old steering could not do this
    /// in principle — it took the sign of the difference in column and
    /// walked that way — and it is the shape of the jam that froze a colony
    /// for seventy thousand steps.
    #[test]
    fn a_gnome_walks_away_from_the_larder_to_get_to_it() {
        let mut w = cavern();
        // A shelf over the corridor, and a wall under its western end.
        for i in 6..=10 {
            w.fill(GridIndex::new(i, 2), t::STONE, 293.0);
        }
        w.fill(GridIndex::new(6, 1), t::STONE, 293.0);
        // The one step up onto it, at the far end of the corridor.
        w.fill(GridIndex::new(11, 1), t::STONE, 293.0);
        w.fill(GridIndex::new(3, 1), t::JUNIPER, 293.0);
        w.rebaseline();

        let mut gnome = Gnome::new(GridIndex::new(7, 1));
        gnome.belly = 0.0;
        let mut colony = Colony::new(vec![gnome]);
        let mut fed = false;
        let mut went_east = false;
        for _ in 0..120 {
            colony.update(&mut w);
            went_east |= colony.gnomes[0].pos.i > 7;
            if colony.gnomes[0].last_act == Act::Foraged {
                fed = true;
                break;
            }
        }
        assert!(went_east, "it never tried the way round");
        assert!(
            fed,
            "it never reached the bush: ended at {:?}",
            colony.gnomes[0].pos
        );
    }

    /// Scenario: a gnome shut in by a garden that has been picked bare. No
    /// open route to anything it needs and nothing left to eat where it is,
    /// so it lifts the hedge over its head and walks out — and the garden
    /// is all still there afterwards, because lifting is a swap.
    #[test]
    fn a_walled_in_gnome_lifts_the_hedge_rather_than_cutting_it() {
        let mut w = pen();
        let crop_before = w.mass_of(t::JUNIPER);
        let mass_before = w.total_mass();
        let energy_before = w.total_energy();

        let mut gnome = Gnome::new(GridIndex::new(6, 1));
        gnome.belly = 0.0;
        let mut colony = Colony::new(vec![gnome]);
        let mut lifted = false;
        let mut fed = false;
        // Long enough for it to give up on getting out any other way — see
        // `TRAPPED_STEPS`. A gnome does not start rearranging the garden the
        // moment it is inconvenienced.
        for _ in 0..(TRAPPED_STEPS as usize + 400) {
            colony.update(&mut w);
            lifted |= colony.gnomes[0].last_act == Act::Heaved;
            fed |= colony.gnomes[0].last_act == Act::Foraged;
        }
        assert!(lifted, "it never lifted the hedge");
        let out = colony.gnomes[0].pos;
        assert!(
            !(4..=8).contains(&out.i),
            "it never got out of the pen: ended at {out:?} (fed: {fed})"
        );
        assert!(
            w.mass_of(t::JUNIPER) > 0.9 * crop_before,
            "lifting a hedge should not cost the garden: {crop_before} -> {} g",
            w.mass_of(t::JUNIPER)
        );
        let r = w.conservation_residuals();
        assert!(r.mass_relative.abs() < 1e-12, "mass residual {r:?}");
        assert!(r.energy_relative.abs() < 1e-12, "energy residual {r:?}");
        assert!(
            (w.total_mass() - mass_before - w.ledger().mass_conjured).abs() < 1e-9,
            "mass moved further than the ledger says"
        );
        let _ = energy_before;
    }

    /// ...and when there is a rock on top of the hedge and no room to shove
    /// it along, the same gnome cuts it down instead, leaving what the table
    /// says a shed bush leaves.
    #[test]
    fn a_hedge_that_cannot_be_lifted_is_cut_down() {
        let mut w = pen();
        // Cap both hedges with rock, and back them with it, so there is
        // nowhere to lift the bush to and nowhere to shove it along to.
        for i in [4, 8] {
            for j in 4..=6 {
                w.fill(GridIndex::new(i, j), t::STONE, 293.0);
            }
        }
        for i in [3, 9] {
            for j in 1..=6 {
                w.fill(GridIndex::new(i, j), t::STONE, 293.0);
            }
        }
        w.rebaseline();
        let mass_before = w.total_mass();

        let mut gnome = Gnome::new(GridIndex::new(6, 1));
        gnome.belly = 0.0;
        let mut colony = Colony::new(vec![gnome]);
        let mut cut = false;
        for _ in 0..(TRAPPED_STEPS as usize + 400) {
            colony.update(&mut w);
            cut |= colony.gnomes[0].last_act == Act::Harvested;
        }
        assert!(cut, "it never cut its way out");
        assert!(
            w.mass_of(t::LITTER) > 0.0,
            "a cut bush should leave what the table says leaf fall leaves"
        );
        // Cutting moves nothing in or out of the world, so both books stay
        // exact and the only ledger entry is the berry it ate on the way.
        let r = w.conservation_residuals();
        assert!(r.mass_relative.abs() < 1e-12, "mass residual {r:?}");
        assert!(r.energy_relative.abs() < 1e-12, "energy residual {r:?}");
        assert!(
            (w.total_mass() - mass_before - w.ledger().mass_conjured).abs() < 1e-9,
            "mass moved further than the ledger says"
        );
    }

    /// Scenario: the player designates a cell at the far end of a room,
    /// with a step in the middle of it. Nobody is standing anywhere near it,
    /// and it still gets done — the order is claimed by how far away it is
    /// *by route*, and the gnome walks the route.
    #[test]
    fn an_order_across_the_room_is_claimed_and_walked_to() {
        let mut w = cavern();
        w.fill(GridIndex::new(10, 1), t::STONE, 293.0);
        w.rebaseline();
        let far = GridIndex::new(17, 1);
        w.fill(far, t::SAND, 293.0);
        w.rebaseline();

        let mut colony = Colony::new(vec![Gnome::new(GridIndex::new(3, 1))]);
        colony.order(far, Job::Dig);
        let mut done = false;
        for _ in 0..200 {
            colony.update(&mut w);
            if colony.orders.completed() == 1 {
                done = true;
                break;
            }
        }
        assert!(
            done,
            "the order was never carried out; the gnome got to {:?}",
            colony.gnomes[0].pos
        );
        assert!(colony.carried_g() > 0.0, "it should be holding the spoil");
    }

    /// Scenario: an order behind a wall nobody can climb is not claimed, is
    /// visibly unreachable, and is eventually given up on rather than
    /// jamming the queue.
    #[test]
    fn an_order_nobody_can_route_to_is_marked_and_retired() {
        let mut w = cavern();
        for j in 1..=6 {
            w.fill(GridIndex::new(10, j), t::STONE, 293.0);
        }
        let far = GridIndex::new(15, 1);
        w.fill(far, t::SAND, 293.0);
        w.rebaseline();

        let mut colony = Colony::new(vec![Gnome::new(GridIndex::new(3, 1))]);
        colony.order(far, Job::Dig);
        colony.update(&mut w);
        assert!(
            colony.orders.iter().all(|o| !o.reachable),
            "an order behind a wall should read as unreachable"
        );
        assert!(
            colony.gnomes[0].job.is_none(),
            "and nobody should have claimed it"
        );
        let budget = crate::order::PATIENCE as usize * crate::order::ATTEMPTS as usize + 10;
        for _ in 0..budget {
            colony.update(&mut w);
        }
        assert!(colony.orders.is_empty(), "it should have been retired");
        assert_eq!(colony.orders.cancelled(), 1);
    }

    /// Scenario: a gnome will walk to a puddle but not into the pool.
    #[test]
    fn a_gnome_does_not_route_along_the_water() {
        let mut w = cavern();
        for i in 8..14 {
            w.fill(GridIndex::new(i, 1), t::WATER, 293.0);
        }
        w.fill(GridIndex::new(16, 1), t::JUNIPER, 293.0);
        w.rebaseline();
        let mut gnome = Gnome::new(GridIndex::new(5, 1));
        gnome.belly = 0.0;
        let mut colony = Colony::new(vec![gnome]);
        for _ in 0..80 {
            colony.update(&mut w);
            let here = colony.gnomes[0].pos;
            assert!(
                !(8..14).contains(&here.i) || here.j != 1,
                "a gnome walked into the pool at {here:?}"
            );
        }
    }

    /// A cavern with a gnome-sized pen of bushes in the middle of it,
    /// already picked down below a berry: in the way, and no use as lunch.
    /// There is a bush worth walking to outside it.
    fn pen() -> World {
        let mut w = cavern();
        for i in [4, 8] {
            for j in 1..=3 {
                let at = GridIndex::new(i, j);
                w.fill(at, t::JUNIPER, 293.0);
                let mut bare = w.cell(at);
                bare.mass = 0.04;
                w.set_cell(at, bare);
            }
        }
        w.fill(GridIndex::new(12, 1), t::JUNIPER, 293.0);
        w.rebaseline();
        w
    }

    #[test]
    fn a_gnome_walks_along_the_floor_instead_of_sinking_into_it() {
        let mut w = cavern();
        let mut colony = Colony::new(vec![Gnome::new(GridIndex::new(5, 6))]);
        for _ in 0..40 {
            colony.update(&mut w);
        }
        assert_eq!(
            colony.gnomes[0].pos.j, 1,
            "a gnome should fall to the floor and stand on it"
        );
    }

    #[test]
    fn magic_costs_gin_and_the_ledger_records_exactly_what_it_added() {
        let mut w = cavern();
        // A pocket of lava-hot air the gnome must chill to survive.
        w.fill(GridIndex::new(5, 1), t::AIR, 500.0);
        w.rebaseline();
        let mut colony = Colony::new(vec![Gnome::new(GridIndex::new(5, 1))]);
        let energy_before = w.total_energy();

        colony.update(&mut w);

        assert_eq!(colony.gnomes[0].last_act, Act::Chilled);
        assert!(colony.gnomes[0].gin < MAX_GIN, "chilling should cost Gin");
        assert!(w.ledger().gin_spent > 0.0);
        // Energy really did leave the world — and the ledger accounts for
        // every joule of it.
        assert!(w.total_energy() < energy_before);
        assert!(
            w.conservation_residuals().energy.abs() < 1e-3,
            "residual {}",
            w.conservation_residuals().energy
        );
    }

    #[test]
    fn a_gnome_out_of_gin_slips_into_the_ethereal_layer_rather_than_dying() {
        let mut w = cavern();
        w.fill(GridIndex::new(5, 1), t::AIR, 900.0);
        w.rebaseline();
        let mut gnome = Gnome::new(GridIndex::new(5, 1));
        gnome.gin = 0.0;
        let mut colony = Colony::new(vec![gnome]);

        colony.update(&mut w);

        assert!(!colony.gnomes[0].is_embodied(), "should have gone ethereal");
        assert_eq!(colony.gnomes[0].last_act, Act::WentEthereal);
        assert_eq!(colony.gnomes.len(), 1, "nobody dies; nobody is removed");
    }

    #[test]
    fn another_gnome_nearby_pulls_an_ethereal_gnome_back() {
        let mut w = cavern();
        w.fill(GridIndex::new(5, 1), t::AIR, 900.0);
        w.rebaseline();
        let mut stuck = Gnome::new(GridIndex::new(5, 1));
        stuck.gin = 0.0;
        let rescuer = Gnome::new(GridIndex::new(7, 1));
        let mut colony = Colony::new(vec![stuck, rescuer]);

        colony.update(&mut w);
        assert!(!colony.gnomes[0].is_embodied());

        // Let the danger pass, then let the rescuer do its work.
        w.fill(GridIndex::new(5, 1), t::AIR, 293.0);
        for _ in 0..RESCUE_STEPS + 5 {
            colony.update(&mut w);
        }
        assert!(
            colony.gnomes[0].is_embodied(),
            "a nearby gnome should have pulled it back"
        );
    }

    #[test]
    fn drowning_costs_gin_first_and_the_ethereal_layer_only_as_a_last_resort() {
        // A sealed flooded pocket, so the gnome cannot simply walk out of
        // the water — which, given the chance, it does.
        let mut w = cavern();
        for i in 3..8 {
            for j in 1..6 {
                w.fill(GridIndex::new(i, j), t::STONE, 293.0);
            }
        }
        for i in 4..7 {
            for j in 1..5 {
                w.fill(GridIndex::new(i, j), t::WATER, 293.0);
            }
        }
        w.rebaseline();
        let mut colony = Colony::new(vec![Gnome::new(GridIndex::new(5, 2))]);
        for _ in 0..BREATH_STEPS + 4 {
            colony.update(&mut w);
            physics::step(&mut w, 0.05);
        }
        assert!(
            colony.gnomes[0].gin < MAX_GIN || !colony.gnomes[0].is_embodied(),
            "a drowning gnome must have either spent Gin or gone ethereal"
        );
        assert!(
            w.conservation_residuals().mass.abs() < 1e-4,
            "even a gasp for air is booked: residual {}",
            w.conservation_residuals().mass
        );
    }

    #[test]
    fn foraging_juniper_restores_gin_and_removes_the_matter_it_ate() {
        let mut w = cavern();
        w.fill(GridIndex::new(6, 1), t::JUNIPER, 293.0);
        w.rebaseline();
        let mut gnome = Gnome::new(GridIndex::new(5, 1));
        gnome.gin = 10.0;
        let mut colony = Colony::new(vec![gnome]);
        let mass_before = w.total_mass();

        colony.update(&mut w);

        assert_eq!(colony.gnomes[0].last_act, Act::Foraged);
        assert!(colony.gnomes[0].gin > 10.0, "berries should restore Gin");
        assert!(w.total_mass() < mass_before, "the berry left the world");
        assert!(w.conservation_residuals().mass.abs() < 1e-4);
    }

    /// Farming, in its smallest honest form: a gnome standing by a bush with
    /// food to spare puts a cutting in above it, and the mass comes out of
    /// its own belly rather than out of nowhere — so the ledger moves *back*
    /// toward zero, because the belly has been holding it negative since the
    /// berry was picked.
    #[test]
    fn a_gnome_with_food_to_spare_plants_a_cutting_from_its_own_belly() {
        let mut w = cavern();
        w.fill(GridIndex::new(6, 1), t::JUNIPER, 293.0);
        w.rebaseline();
        let mut gnome = Gnome::new(GridIndex::new(5, 1));
        gnome.belly = BELLY_FULL;
        let mut colony = Colony::new(vec![gnome]);
        // The cadence starts the colony able to plant on its first update.
        colony.update(&mut w);

        assert_eq!(colony.gnomes[0].last_act, Act::Planted);
        assert_eq!(
            w.material_at(GridIndex::new(6, 2)),
            t::JUNIPER,
            "the cutting should be in the course above the bush:\n{}",
            crate::report::ascii_map(&w)
        );
        assert!(
            // Within one step's breathing, which happens on the same update.
            (colony.gnomes[0].belly - (BELLY_FULL - CUTTING_G)).abs() < 2.0 * RESPIRATION_G,
            "the cutting came from somewhere other than the belly: {}",
            colony.gnomes[0].belly
        );
        // Planting is matter entering the world, so it is booked — positive,
        // which is the ledger coming back toward zero after a meal.
        assert!(w.ledger().mass_conjured > 0.04);
        assert!(w.conservation_residuals().mass.abs() < 1e-9);
    }

    /// ...and never on its own row, whatever is standing there — a hedge
    /// across the walkway is a fence, and a gnome cannot climb a fence.
    #[test]
    fn a_gnome_never_plants_a_cutting_across_its_own_path() {
        let mut w = cavern();
        for i in 4..9 {
            w.fill(GridIndex::new(i, 1), t::JUNIPER, 293.0);
        }
        w.rebaseline();
        let mut gnome = Gnome::new(GridIndex::new(3, 1));
        gnome.belly = BELLY_FULL;
        let mut colony = Colony::new(vec![gnome]);
        for _ in 0..PLANT_PERIOD * 3 {
            colony.update(&mut w);
        }
        for i in 0..20 {
            if (4..9).contains(&i) {
                continue;
            }
            assert_ne!(
                w.material_at(GridIndex::new(i, 1)),
                t::JUNIPER,
                "a cutting went in on the walkway at {i}:\n{}",
                crate::report::ascii_map(&w)
            );
        }
    }

    #[test]
    fn an_ethereal_pipe_moves_water_uphill_without_creating_any() {
        // The ONI complaint, made concrete: water in a sealed basin, an
        // outlet high above it with no physical path between, and the water
        // gets there anyway — magically, at a price, and without a gram
        // appearing from nowhere.
        let mut w = World::new_open(9, 12, MaterialTable::terrarium(), 293.0);
        for i in 0..9 {
            w.fill(GridIndex::new(i, 0), t::STONE, 293.0);
            w.fill(GridIndex::new(i, 4), t::STONE, 293.0);
        }
        for j in 0..12 {
            w.fill(GridIndex::new(0, j), t::STONE, 293.0);
            w.fill(GridIndex::new(8, j), t::STONE, 293.0);
        }
        for i in 1..8 {
            for j in 1..4 {
                w.fill(GridIndex::new(i, j), t::WATER, 293.0);
            }
        }
        w.rebaseline();
        let mass_before = w.total_mass();

        let pipe = EtherealPipe::new(GridIndex::new(4, 3), GridIndex::new(4, 6));
        let mut gnome = Gnome::new(GridIndex::new(2, 6));
        gnome.gin = MAX_GIN;
        let mut colony = Colony::new(vec![gnome]).with_pipe(pipe);

        let above_before = (5..12)
            .filter(|&j| w.material_at(GridIndex::new(4, j)) == t::WATER)
            .count();
        for _ in 0..30 {
            colony.update(&mut w);
            physics::step(&mut w, 0.05);
        }
        let above_after: usize = (1..8)
            .flat_map(|i| (5..12).map(move |j| GridIndex::new(i, j)))
            .filter(|&idx| w.material_at(idx) == t::WATER)
            .count();

        assert_eq!(above_before, 0);
        assert!(
            above_after > 0,
            "the pipe should have lifted water over the ceiling"
        );
        assert!(w.ledger().gin_spent > 0.0, "the pipe should cost Gin");
        assert!(
            (w.total_mass() - mass_before).abs() < 1e-4,
            "a pipe transfer is a swap: no mass may appear or vanish"
        );
        assert!(w.conservation_residuals().mass.abs() < 1e-4);
    }

    #[test]
    fn a_colony_survives_a_long_run_and_the_books_still_balance() {
        let mut w = World::new_open(32, 20, MaterialTable::terrarium(), 293.0);
        for i in 0..32 {
            w.fill(GridIndex::new(i, 0), t::STONE, 293.0);
            w.fill(GridIndex::new(i, 1), t::STONE, 293.0);
        }
        for i in 2..30 {
            w.fill(GridIndex::new(i, 2), t::SAND, 293.0);
        }
        for i in [5, 11, 19, 25] {
            w.fill(GridIndex::new(i, 3), t::JUNIPER, 293.0);
        }
        // A hot spring at one end and a block of ice at the other, so the
        // world is doing something while the gnomes live in it.
        for i in 1..4 {
            w.fill(GridIndex::new(i, 1), t::LAVA, 1700.0);
        }
        for i in 26..30 {
            for j in 8..11 {
                w.fill(GridIndex::new(i, j), t::ICE, 240.0);
            }
        }
        w.rebaseline();

        let mut colony = Colony::new(
            (0..6)
                .map(|k| Gnome::new(GridIndex::new(6 + k * 3, 8)))
                .collect(),
        );

        for _ in 0..800 {
            physics::step(&mut w, 0.05);
            colony.update(&mut w);
        }

        let r = w.conservation_residuals();
        assert!(
            r.mass_relative.abs() < 1e-6,
            "mass residual {} ({:e} relative) after a full colony run",
            r.mass,
            r.mass_relative
        );
        assert!(
            r.energy_relative.abs() < 1e-6,
            "energy residual {} ({:e} relative) after a full colony run",
            r.energy,
            r.energy_relative
        );
        assert_eq!(colony.gnomes.len(), 6, "no gnome is ever destroyed");
        assert!(
            colony.embodied_count() >= 1,
            "the colony should not be wiped out"
        );
        assert!(
            w.ledger().gin_spent >= 0.0,
            "the Gin ledger should be coherent"
        );
    }
}
