//! The glass pane: what a player asks for, and what it costs to ask.
//!
//! `NORTH_STARS.md` #2 lists the content order as physics, then chemistry,
//! then biology, then **a game/interaction layer**; #3 states the whole
//! thing as "a terrarium people can see on their screens *and interact
//! with*". Until now the pages were windows — the jar ran, and a human
//! watched it run. This module is the handle on the glass.
//!
//! A player does not touch the world. A player writes an [`Order`] on a
//! cell, and a gnome walks over and does it — which is the ONI-shaped
//! interaction (designate, and the colony obeys) and also the honest one,
//! because it keeps every actual change to the world inside the same rules
//! everything else obeys.
//!
//! Three jobs, and the split between them is the point:
//!
//! - [`Job::Dig`] takes a cell of solid out of the world and puts it in a
//!   gnome's hands as a [`Load`].
//! - [`Job::Build`] puts that load back down somewhere else.
//! - [`Job::Supply`] is those two as one instruction — *keep this cell
//!   stocked with that stuff* — with the colony finding the nearest source
//!   itself. It is the verb a colony needs to run a machine rather than
//!   only to dig a hole: a pot wants water and botanicals in it, a firebox
//!   wants fuel, and neither is a thing a player should have to hand-steer
//!   two orders at a time, for ever.
//! - [`Job::Temper`] warms or chills a cell. This one is **magic**, and it
//!   is priced in Gin at exactly the rate a gnome pays to save its own life
//!   (`gnome::GIN_PER_JOULE`), so a player who wants the pool warmed spends
//!   the colony's mana to get it.
//!
//! Any of them can be written as a **standing** order ([`Orders::repeat`]),
//! which is the same instruction with a cadence: when it is done it goes
//! back on the queue rather than off it. A still that has to be re-charged
//! every few hundred steps is then one mark on the glass instead of a
//! player tapping a cell all night.
//!
//! Digging and building are *not* magic and cost no Gin: they are a gnome
//! picking something up and putting it down. They still run through
//! [`World::conjure_mass`], for the same reason a gnome's belly does — what
//! is in a gnome's hands is outside the simulation while it is there, and
//! the ledger says how much. So the standing invariant is untouched, and it
//! has a new and rather pointed consequence:
//!
//! > **You cannot build what you have not dug.**
//!
//! There is no material palette and no resource counter. A wall built here
//! is a wall taken from there, the same grams at the same temperature. ONI
//! turns dug rock into an abstract number in a sidebar and lets you spend it
//! on things that weigh something else; `NORTH_STARS.md` #4 opens with that
//! class of complaint. This is the version where the mass is the resource.

use crate::material::{MaterialId, Mobility, Phase};
use crate::math::{GridIndex, Scalar};
use crate::world::World;

/// How close a gnome must be to work on a cell: its own cell or any of the
/// eight around it. A gnome digs at arm's length, like it plants.
pub const REACH: i32 = 1;

/// Kelvin a [`Job::Temper`] moves its cell per step. A spell that dumped the
/// whole difference in one step would cost a flask at once and look like a
/// switch being thrown; this is a gnome standing there working at it, and it
/// is bounded, so the Gin goes out at a rate a player can watch and stop.
pub const TEMPER_STEP_K: Scalar = 8.0;

/// How close to its target a [`Job::Temper`] counts as finished.
pub const TEMPER_TOLERANCE_K: Scalar = 1.5;

/// Steps a gnome will persist with one order before giving it up for
/// somebody else to try. Walking the length of the default jar takes about
/// forty steps, so this is generous by an order of magnitude: what it
/// actually catches is an order nobody can reach.
pub const PATIENCE: u32 = 400;

/// Attempts an order gets before the queue drops it. Three different gnomes
/// failing to reach the same cell is an answer, not an accident.
pub const ATTEMPTS: u8 = 3;

/// What the player asked for at one cell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Job {
    /// Take the solid here away, into the hands of whoever does it.
    Dig,
    /// Put down here whatever the gnome doing it is carrying.
    Build,
    /// Fetch a cell of `material` from wherever the colony can reach one,
    /// and put it here.
    ///
    /// Two paces in one order, and the first of them has no fixed address:
    /// the gnome that takes it on looks for the nearest cell of the stuff it
    /// can actually walk to, lifts that, and carries it here. So a supply
    /// order is written on the *destination* — which is the thing a player
    /// knows and cares about — and where it comes from is the colony's
    /// problem. See [`crate::gnome::Colony::worksite`].
    Supply { material: MaterialId },
    /// Bring this cell to `target_k`, paid for in Gin. The magic one.
    Temper { target_k: Scalar },
}

impl Job {
    /// Whether a gnome carrying `load` can take this job on. Digging needs
    /// free hands, building needs full ones; magic needs neither; supplying
    /// needs either empty hands or the right thing already in them.
    pub fn suits(&self, load: Option<&Load>) -> bool {
        match self {
            Job::Dig => load.is_none(),
            Job::Build => load.is_some(),
            Job::Supply { material } => load.is_none_or(|l| l.material == *material),
            Job::Temper { .. } => true,
        }
    }

    /// The colour this job's marker paints as — data, like everything else
    /// the renderer reads.
    pub fn marker_colour(&self) -> (u8, u8, u8) {
        match self {
            Job::Dig => (240, 170, 60),
            Job::Build => (120, 200, 255),
            Job::Supply { .. } => (150, 235, 140),
            Job::Temper { .. } => (235, 100, 155),
        }
    }
}

/// One outstanding order.
#[derive(Debug, Clone, Copy)]
pub struct Order {
    pub id: u64,
    pub at: GridIndex,
    pub job: Job,
    /// Index into [`crate::gnome::Colony::gnomes`] of whoever took it on.
    pub claimed_by: Option<usize>,
    /// Steps the current claimant has held it for — see [`PATIENCE`].
    pub waited: u32,
    /// How many gnomes have given up on it — see [`ATTEMPTS`].
    pub attempts: u8,
    /// Steps to wait before this order is offered again once it is done, or
    /// `None` for the ordinary kind that is finished when it is finished.
    ///
    /// A standing order is not a second mechanism: it is the same order,
    /// re-armed. Everything else here — claiming, patience, reachability,
    /// rubbing it out — works on it unchanged.
    pub repeat: Option<u32>,
    /// The tick this order may next be claimed on. Only ever in the future
    /// for a standing order that has just been done.
    ready_at: u64,
    /// Whether anybody could get to it when the colony last looked.
    ///
    /// Since night 8 a gnome will not claim a job it has no route to, which
    /// is right — it stops three gnomes walking into the same wall — and
    /// which quietly removed the mechanism that used to retire an
    /// impossible order, because an order nobody claims is an order nobody
    /// can give up on. So the colony now says, once a step, which orders
    /// anybody could reach, and [`Orders::end_survey`] ages the rest out.
    /// It is also worth drawing: an order the colony cannot get to is the
    /// player's problem to solve, not the colony's.
    pub reachable: bool,
    /// Steps this order has spent with nobody able to reach it.
    unreached: u32,
}

/// What a gnome is carrying: real grams of a real material, at the
/// temperature it was dug at.
///
/// Not modelled: a carried load cooling toward the air around it. A gnome
/// hauling a rock out of the fire arrives with a hot rock however long it
/// walked. That is a lie of the same family as the belly's (digestion is
/// instantaneous too), and it is in the ledger either way.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Load {
    pub material: MaterialId,
    pub grams: Scalar,
    pub temperature: Scalar,
}

/// What came of one attempt at an order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Dug,
    Built,
    /// Picked up the makings of a supply order. The order is *not* done —
    /// the gnome is now carrying it to where it was asked for.
    Took,
    /// A supply order delivered.
    Stocked,
    /// Moved toward the target temperature but not there yet.
    Tempering,
    /// Reached the target: the order is done.
    Tempered,
    /// Not now — try again next step. The order stays.
    NotYet,
    /// This can never work: the queue drops the order.
    Impossible,
}

impl Outcome {
    /// Whether this outcome finishes the order.
    pub fn completes(&self) -> bool {
        matches!(
            self,
            Outcome::Dug | Outcome::Built | Outcome::Stocked | Outcome::Tempered
        )
    }
}

/// The result of [`perform`]: what happened, and the Gin it cost.
#[derive(Debug, Clone, Copy)]
pub struct Attempt {
    pub outcome: Outcome,
    pub gin: Scalar,
}

impl Attempt {
    fn free(outcome: Outcome) -> Self {
        Attempt { outcome, gin: 0.0 }
    }
}

/// Whether a cell is something a gnome could pick up: it has mass, and it
/// holds still. Read off the material table, so it is true of any solid or
/// granular material anyone ever adds, and of no liquid or gas.
pub fn diggable(world: &World, at: GridIndex) -> bool {
    if !world.in_bounds(at) {
        return false;
    }
    let cell = world.cell(at);
    if cell.mass <= 0.0 {
        return false;
    }
    let m = world.materials().get(cell.material);
    m.mobility == Mobility::Static || m.phase == Phase::Solid || m.phase == Phase::Granular
}

/// Whether a cell is something a gnome could carry away in its arms: it has
/// mass and it is not a gas.
///
/// Wider than [`diggable`] by exactly one phase — liquids. A shovel does not
/// pick up water and a bucket does, and the difference matters the moment a
/// colony has to *charge a pot* rather than only move a wall. It is still a
/// table lookup, so it is true of any liquid anyone ever adds.
pub fn liftable(world: &World, at: GridIndex) -> bool {
    if !world.in_bounds(at) {
        return false;
    }
    let cell = world.cell(at);
    cell.mass > 0.0 && !cell.is_gas(world.materials())
}

/// Takes the whole of the cell at `at` into `load`.
///
/// A whole cell, never a part of one, and for both of the reasons a gnome
/// would give: a partly-dug rock is not a thing, and a load that varies with
/// how full the cell happened to be makes every downstream figure — what a
/// wall weighs, what a pot was charged with — impossible to predict.
fn lift(world: &mut World, at: GridIndex, load: &mut Option<Load>) -> Attempt {
    let cell = world.cell(at);
    let (material, temperature) = (cell.material, cell.temperature);
    let taken = -world.conjure_mass(at, material, -cell.mass, temperature);
    if taken <= 0.0 {
        return Attempt::free(Outcome::Impossible);
    }
    *load = Some(Load {
        material,
        grams: taken,
        temperature,
    });
    Attempt::free(Outcome::Dug)
}

/// Puts `load` down in the cell at `at`, if there is room for it.
fn place(world: &mut World, at: GridIndex, load: &mut Option<Load>) -> Attempt {
    let Some(held) = *load else {
        return Attempt::free(Outcome::NotYet);
    };
    let cell = world.cell(at);
    // Only ever into gas. Building into a solid would overwrite it (and
    // book the loss), and building into the pool would delete water; a cell
    // of either may yet drain or be dug, so this is "not now", not "never" —
    // [`PATIENCE`] decides when never.
    if !cell.is_gas(world.materials()) {
        return Attempt::free(Outcome::NotYet);
    }
    // Push the air out of the way rather than overwriting it, so the
    // atmosphere the cell was holding survives being built on.
    let p = world.linear_index(at);
    if !crate::gas::displace(world, p) {
        return Attempt::free(Outcome::NotYet);
    }
    world.conjure_mass(at, held.material, held.grams, held.temperature);
    *load = None;
    Attempt::free(Outcome::Built)
}

/// Does one step of work on `order` at `site`, from a gnome holding `load`
/// and able to afford `gin`.
///
/// `site` is the cell the gnome is actually working on this step, which is
/// `order.at` for every job except a [`Job::Supply`] whose carrier still has
/// to go and fetch the goods — see [`crate::gnome::Colony::worksite`].
///
/// Every change to the world here goes through the ledger, so a caller that
/// forgets to charge the Gin has still not broken conservation — it has
/// only given the magic away.
pub fn perform(
    world: &mut World,
    order: &Order,
    site: GridIndex,
    load: &mut Option<Load>,
    gin: Scalar,
) -> Attempt {
    let at = order.at;
    if !world.in_bounds(at) || !world.in_bounds(site) {
        return Attempt::free(Outcome::Impossible);
    }
    match order.job {
        Job::Dig => {
            if load.is_some() {
                return Attempt::free(Outcome::NotYet);
            }
            if !diggable(world, at) {
                return Attempt::free(Outcome::Impossible);
            }
            lift(world, at, load)
        }
        Job::Build => place(world, at, load),
        Job::Supply { material } => {
            if load.is_none() {
                // Fetching. The site is a source the colony found; check it
                // is still what it was when the route was laid, because a
                // pool drains and a bush gets eaten.
                if !liftable(world, site) || world.material_at(site) != material {
                    return Attempt::free(Outcome::NotYet);
                }
                let attempt = lift(world, site, load);
                return match attempt.outcome {
                    Outcome::Dug => Attempt::free(Outcome::Took),
                    other => Attempt::free(other),
                };
            }
            // Carrying. Put it where it was asked for.
            match place(world, at, load).outcome {
                Outcome::Built => Attempt::free(Outcome::Stocked),
                other => Attempt::free(other),
            }
        }
        Job::Temper { target_k } => {
            let cell = world.cell(at);
            let gap = target_k - cell.temperature;
            if gap.abs() <= TEMPER_TOLERANCE_K {
                return Attempt::free(Outcome::Tempered);
            }
            let capacity = cell.capacity(world.materials());
            if capacity <= 0.0 {
                // Nothing here to hold heat — an empty cell in a vacuum.
                return Attempt::free(Outcome::Impossible);
            }
            let step = gap.clamp(-TEMPER_STEP_K, TEMPER_STEP_K);
            let joules = step * capacity;
            let cost = joules.abs() * crate::gnome::GIN_PER_JOULE;
            if cost > gin {
                return Attempt::free(Outcome::NotYet);
            }
            world.conjure_energy(at, joules);
            Attempt {
                outcome: Outcome::Tempering,
                gin: cost,
            }
        }
    }
}

/// The player's outstanding orders, in the order they were written.
///
/// Held by the [`Colony`](crate::gnome::Colony), because it is the colony
/// that does them: there is no separate player entity in this world, only
/// gnomes with a list.
#[derive(Debug, Default, Clone)]
pub struct Orders {
    queue: Vec<Order>,
    next_id: u64,
    completed: u64,
    cancelled: u64,
    /// Gnomes that reported in during the current survey — see
    /// [`Orders::begin_survey`].
    surveyors: u32,
}

impl Orders {
    pub fn new() -> Self {
        Orders::default()
    }

    /// Writes an order on a cell, replacing any order already on it — a
    /// cell carries one instruction at a time, which is what makes clicking
    /// the same cell with a different tool mean "no, *this* instead".
    /// Returns the new order's id.
    pub fn issue(&mut self, at: GridIndex, job: Job) -> u64 {
        self.queue.retain(|o| o.at != at);
        self.next_id += 1;
        self.queue.push(Order {
            id: self.next_id,
            at,
            job,
            claimed_by: None,
            waited: 0,
            attempts: 0,
            repeat: None,
            ready_at: 0,
            reachable: true,
            unreached: 0,
        });
        self.next_id
    }

    /// Makes the order with this id a standing one, re-offered `every`
    /// steps after each time it is done. Silently does nothing if the order
    /// has already been finished or rubbed out.
    pub fn repeat(&mut self, id: u64, every: u32) {
        if let Some(o) = self.queue.iter_mut().find(|o| o.id == id) {
            o.repeat = Some(every);
        }
    }

    /// Rubs out the order on a cell, if there is one.
    pub fn cancel_at(&mut self, at: GridIndex) -> bool {
        let before = self.queue.len();
        self.queue.retain(|o| o.at != at);
        let gone = before != self.queue.len();
        if gone {
            self.cancelled += 1;
        }
        gone
    }

    pub fn clear(&mut self) {
        self.cancelled += self.queue.len() as u64;
        self.queue.clear();
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Orders finished, and orders given up on or rubbed out — the two
    /// numbers the report prints, so a headless run can say whether the
    /// colony is actually working.
    pub fn completed(&self) -> u64 {
        self.completed
    }

    pub fn cancelled(&self) -> u64 {
        self.cancelled
    }

    pub fn iter(&self) -> impl Iterator<Item = &Order> {
        self.queue.iter()
    }

    pub fn at(&self, at: GridIndex) -> Option<&Order> {
        self.queue.iter().find(|o| o.at == at)
    }

    pub(crate) fn get(&self, id: u64) -> Option<Order> {
        self.queue.iter().find(|o| o.id == id).copied()
    }

    /// Starts a round of "who can get where" — call once per colony
    /// update, before any gnome surveys.
    pub(crate) fn begin_survey(&mut self) {
        for o in self.queue.iter_mut() {
            o.reachable = false;
        }
        self.surveyors = 0;
    }

    /// One gnome's contribution to that round: every order it can reach is
    /// marked reachable.
    pub(crate) fn survey(&mut self, steps: impl Fn(GridIndex) -> Option<u32>) {
        self.surveyors += 1;
        for o in self.queue.iter_mut() {
            if !o.reachable && steps(o.at).is_some() {
                o.reachable = true;
            }
        }
    }

    /// Closes the round, and retires anything the whole colony has been
    /// unable to reach for [`PATIENCE`] × [`ATTEMPTS`] steps.
    ///
    /// Nothing is retired in a round nobody surveyed — a colony entirely in
    /// the ethereal layer has not decided an order is impossible, it has
    /// merely stopped looking.
    pub(crate) fn end_survey(&mut self) {
        if self.surveyors == 0 {
            return;
        }
        let limit = PATIENCE * ATTEMPTS as u32;
        let doomed: Vec<u64> = self
            .queue
            .iter_mut()
            .filter_map(|o| {
                if o.reachable {
                    o.unreached = 0;
                    return None;
                }
                o.unreached += 1;
                (o.unreached >= limit).then_some(o.id)
            })
            .collect();
        for id in doomed {
            self.cancel(id);
        }
    }

    /// The order `worker` could usefully take on soonest, claimed for it.
    ///
    /// "Soonest" is `steps` — how many paces away a job is by a route the
    /// gnome could actually walk, `None` for one it cannot get to at all.
    /// It used to be straight-line distance, which is how a colony ends up
    /// with every gnome claiming the job on the far side of a wall and
    /// nobody claiming the one behind them.
    pub(crate) fn claim_for(
        &mut self,
        worker: usize,
        load: Option<&Load>,
        now: u64,
        steps: impl Fn(GridIndex) -> Option<u32>,
    ) -> Option<u64> {
        let pick = self
            .queue
            .iter()
            .filter(|o| o.claimed_by.is_none() && o.ready_at <= now && o.job.suits(load))
            .filter_map(|o| steps(o.at).map(|d| (d, o.id)))
            .min()
            .map(|(_, id)| id)?;
        if let Some(o) = self.queue.iter_mut().find(|o| o.id == pick) {
            o.claimed_by = Some(worker);
            o.waited = 0;
        }
        Some(pick)
    }

    /// Counts a step of waiting against the claimant. Returns true if it
    /// has run out of patience.
    pub(crate) fn tick_claim(&mut self, id: u64) -> bool {
        match self.queue.iter_mut().find(|o| o.id == id) {
            Some(o) => {
                o.waited += 1;
                o.waited >= PATIENCE
            }
            None => false,
        }
    }

    /// Hands an order back. After [`ATTEMPTS`] of these it is dropped.
    pub(crate) fn release(&mut self, id: u64) {
        let Some(o) = self.queue.iter_mut().find(|o| o.id == id) else {
            return;
        };
        o.claimed_by = None;
        o.waited = 0;
        o.attempts += 1;
        if o.attempts >= ATTEMPTS {
            self.cancel(id);
        }
    }

    /// Marks an order done. A standing one goes back on the queue with its
    /// cadence to wait out; an ordinary one leaves.
    pub(crate) fn complete(&mut self, id: u64, now: u64) {
        let Some(o) = self.queue.iter_mut().find(|o| o.id == id) else {
            return;
        };
        self.completed += 1;
        match o.repeat {
            Some(every) => {
                o.claimed_by = None;
                o.waited = 0;
                o.attempts = 0;
                o.unreached = 0;
                o.ready_at = now + every as u64;
            }
            None => self.queue.retain(|o| o.id != id),
        }
    }

    pub(crate) fn cancel(&mut self, id: u64) {
        if self.queue.iter().any(|o| o.id == id) {
            self.queue.retain(|o| o.id != id);
            self.cancelled += 1;
        }
    }

    /// Drops any claim held by `worker` — used when a gnome slips into the
    /// ethereal layer part-way through a job.
    pub(crate) fn release_all_of(&mut self, worker: usize) {
        let ids: Vec<u64> = self
            .queue
            .iter()
            .filter(|o| o.claimed_by == Some(worker))
            .map(|o| o.id)
            .collect();
        for id in ids {
            self.release(id);
        }
    }
}

/// Whether `pos` is close enough to `at` to work on it.
pub fn within_reach(pos: GridIndex, at: GridIndex) -> bool {
    (pos.i - at.i).abs() <= REACH && (pos.j - at.j).abs() <= REACH
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::{terrarium as t, MaterialTable};

    fn cavern() -> World {
        let mut w = World::new_open(16, 10, MaterialTable::terrarium(), 293.0);
        for i in 0..16 {
            w.fill(GridIndex::new(i, 0), t::STONE, 293.0);
            w.fill(GridIndex::new(i, 1), t::SAND, 293.0);
        }
        w.rebaseline();
        w
    }

    /// Scenario: digging a cell and putting it down elsewhere moves real
    /// grams and leaves the world's books exactly where it found them.
    #[test]
    fn a_dug_cell_is_carried_and_put_back_down_gram_for_gram() {
        let mut w = cavern();
        let start_mass = w.total_mass();
        let mut load = None;

        let dig = Order {
            id: 1,
            at: GridIndex::new(4, 1),
            job: Job::Dig,
            claimed_by: None,
            waited: 0,
            attempts: 0,
            repeat: None,
            ready_at: 0,
            reachable: true,
            unreached: 0,
        };
        let a = perform(&mut w, &dig, dig.at, &mut load, 0.0);
        assert_eq!(a.outcome, Outcome::Dug);
        assert_eq!(a.gin, 0.0, "digging is labour, not magic");
        let held = load.expect("the gnome should be holding the sand");
        assert_eq!(held.material, t::SAND);
        assert!(held.grams > 0.0);

        // While it is in hand, the world is exactly that much lighter, and
        // the ledger says so.
        assert!((w.total_mass() - (start_mass - held.grams)).abs() < 1e-9);
        assert!((w.ledger().mass_conjured + held.grams).abs() < 1e-9);

        let build = Order {
            id: 2,
            at: GridIndex::new(9, 4),
            job: Job::Build,
            claimed_by: None,
            waited: 0,
            attempts: 0,
            repeat: None,
            ready_at: 0,
            reachable: true,
            unreached: 0,
        };
        let b = perform(&mut w, &build, build.at, &mut load, 0.0);
        assert_eq!(b.outcome, Outcome::Built);
        assert!(load.is_none(), "hands are empty again");
        assert_eq!(w.material_at(GridIndex::new(9, 4)), t::SAND);

        // The round trip is a no-op on the books: mass back, ledger back,
        // residuals untouched.
        assert!(
            (w.total_mass() - start_mass).abs() < 1e-9,
            "mass {} vs {}",
            w.total_mass(),
            start_mass
        );
        assert!(w.ledger().mass_conjured.abs() < 1e-9);
        assert!(w.conservation_residuals().mass.abs() < 1e-6);
        assert!(w.conservation_residuals().energy.abs() < 1e-3);
    }

    /// Scenario: you cannot dig the pool, and you cannot dig the air.
    #[test]
    fn only_solids_can_be_dug() {
        let mut w = cavern();
        w.fill(GridIndex::new(4, 2), t::WATER, 293.0);
        w.rebaseline();
        let mut load = None;
        for (at, what) in [
            (GridIndex::new(4, 2), "water"),
            (GridIndex::new(4, 6), "air"),
        ] {
            let order = Order {
                id: 1,
                at,
                job: Job::Dig,
                claimed_by: None,
                waited: 0,
                attempts: 0,
                repeat: None,
                ready_at: 0,
                reachable: true,
                unreached: 0,
            };
            assert_eq!(
                perform(&mut w, &order, order.at, &mut load, 0.0).outcome,
                Outcome::Impossible,
                "{what} should not be diggable"
            );
            assert!(load.is_none());
        }
    }

    /// Scenario: tempering is magic, so it costs Gin in proportion to the
    /// joules it moves — and it moves them into the ledger, not out of
    /// nowhere.
    #[test]
    fn tempering_costs_gin_and_is_booked() {
        let mut w = cavern();
        let at = GridIndex::new(4, 1);
        let order = Order {
            id: 1,
            at,
            job: Job::Temper { target_k: 350.0 },
            claimed_by: None,
            waited: 0,
            attempts: 0,
            repeat: None,
            ready_at: 0,
            reachable: true,
            unreached: 0,
        };
        let mut load = None;
        let before = w.cell(at).temperature;
        let a = perform(&mut w, &order, order.at, &mut load, 100.0);
        assert_eq!(a.outcome, Outcome::Tempering);
        assert!(a.gin > 0.0, "warming a cell should cost Gin");
        let after = w.cell(at).temperature;
        assert!(after > before, "{after} should be warmer than {before}");
        assert!(
            (after - before - TEMPER_STEP_K).abs() < 1e-6,
            "one step should move {TEMPER_STEP_K} K, moved {}",
            after - before
        );
        assert!(w.ledger().energy_conjured > 0.0);
        assert!(w.conservation_residuals().energy.abs() < 1e-3);
    }

    /// Scenario: a gnome with no Gin cannot cast, and the order waits for
    /// one who can rather than being thrown away.
    #[test]
    fn a_spell_nobody_can_pay_for_waits() {
        let mut w = cavern();
        let order = Order {
            id: 1,
            at: GridIndex::new(4, 1),
            job: Job::Temper { target_k: 350.0 },
            claimed_by: None,
            waited: 0,
            attempts: 0,
            repeat: None,
            ready_at: 0,
            reachable: true,
            unreached: 0,
        };
        let mut load = None;
        assert_eq!(
            perform(&mut w, &order, order.at, &mut load, 0.0).outcome,
            Outcome::NotYet
        );
        assert!(w.ledger().energy_conjured.abs() < 1e-12);
    }

    /// Scenario: one cell, one instruction — writing a second order on a
    /// cell replaces the first rather than queueing behind it.
    #[test]
    fn a_cell_carries_one_order_at_a_time() {
        let mut orders = Orders::new();
        let at = GridIndex::new(3, 3);
        orders.issue(at, Job::Dig);
        orders.issue(at, Job::Build);
        assert_eq!(orders.len(), 1);
        assert_eq!(orders.at(at).map(|o| o.job), Some(Job::Build));
    }

    /// Scenario: an order nobody can reach is dropped rather than jamming
    /// the queue for ever.
    #[test]
    fn an_unreachable_order_is_given_up_on() {
        let mut orders = Orders::new();
        let at = GridIndex::new(3, 3);
        let id = orders.issue(at, Job::Dig);
        for _ in 0..ATTEMPTS {
            assert!(orders.claim_for(0, None, 0, |_| Some(4)).is_some());
            orders.release(id);
        }
        assert!(orders.is_empty(), "three refusals should retire it");
        assert_eq!(orders.cancelled(), 1);
    }
}
