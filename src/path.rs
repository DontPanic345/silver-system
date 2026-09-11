//! Where a gnome can actually get to, and the first step of getting there.
//!
//! Until tonight a gnome steered by the *sign of the difference in column*
//! between it and whatever it wanted: nearest bush to the west, so walk
//! west, every step, for ever. That is fine in an empty room and wrong in a
//! world where things grow. Run the jar to eighty thousand steps and all
//! four gnomes end up in one cell with their bellies empty and their flasks
//! dry, pressed against a hedge that seeded itself across the walkway, one
//! cell below a clear route over the top of it that nothing ever tried —
//! because west was still west. Night 7 recorded that as "the cause is
//! pathing rather than physics", which it was.
//!
//! So: a real route, over the moves a gnome can really make.
//!
//! The one rule that matters here is that [`moves`] is the *same* move set
//! `Colony::update_embodied` executes — falling before walking, a climb of
//! one course only, and only into a cell a gnome would choose to stand in.
//! A route built from a more generous graph than the walker has is a lie
//! that shows up as a gnome jammed against a wall trying to walk through
//! it, which is the bug this module exists to remove rather than move.
//!
//! Two ways through a world ([`Through`]): what is open as it stands, and
//! what is open to a gnome willing to cut a crop out of its way. The second
//! is only ever consulted when the first finds nothing, so a gnome prunes
//! the hedge that has walled it in and never prunes one it could have
//! walked around.

use std::collections::VecDeque;

use crate::material::{Mobility, Phase};
use crate::math::GridIndex;
use crate::world::World;

/// What a route is allowed to go through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Through {
    /// Only cells a gnome can walk into as the world stands.
    #[default]
    Open,
    /// Those, plus crops — a gnome may cut one out of its way. See
    /// [`harvestable`].
    Crops,
}

/// A cell a gnome can end up in: not solid, and not something it would sink
/// through like a stone.
///
/// Moved here from `src/gnome.rs` unchanged, because a route and a walker
/// that disagree about this are a route and a walker that disagree about
/// everything.
///
/// "Mostly empty" has to mean granular, not simply under half full: a cell
/// of litter at a tenth of its density is a scattering of leaves with air
/// between them, and a cell of *juniper* at a tenth of its density is not a
/// gappy bush, it is a small one, because a plant cell's mass is its size.
pub fn passable(world: &World, at: GridIndex) -> bool {
    if !world.in_bounds(at) {
        return false;
    }
    let m = world.materials().get(world.material_at(at));
    if m.mobility != Mobility::Static && m.phase != Phase::Solid {
        return true;
    }
    m.mobility == Mobility::Granular && m.density > 0.0 && world.mass_at(at) < 0.5 * m.density
}

/// Whether there is enough liquid in this cell to wade in. A raindrop's
/// worth of water on the walkway is a puddle and a gnome walks through it;
/// half a cell of it is the pool.
pub fn wet(world: &World, at: GridIndex) -> bool {
    if !world.in_bounds(at) {
        return false;
    }
    let m = world.materials().get(world.material_at(at));
    m.phase == Phase::Liquid && m.density > 0.0 && world.mass_at(at) >= 0.5 * m.density
}

/// A cell a gnome will *choose* to walk into — [`passable`], minus the
/// pool, and minus anything standing directly over it.
///
/// The second clause is the one worth stating. A gnome falls before it
/// walks, so a cell with the pool underneath it is not a place to stand, it
/// is the last thing you see before you are swimming; without that lookahead
/// a route along the water's edge is a route into the water, and the gnome
/// that takes it spends Gin gasping its way back out. It can still *fall*
/// in, and a rising pool can still close over it — what it will no longer do
/// is walk in on purpose.
pub fn steppable(world: &World, at: GridIndex) -> bool {
    passable(world, at) && !wet(world, at) && !wet(world, GridIndex::new(at.i, at.j - 1))
}

/// Whether the cell at `at` is a crop a gnome could cut down: something the
/// table grows, with a shed form to leave behind, and some of it there.
///
/// Every clause is a table lookup. Stone is not harvestable because nothing
/// grows it, which is why a wall still needs a dig order and a shovel.
///
/// Deliberately not gated on the crop being *mature*. The first draft was —
/// a gnome should prune a hedge, not trample a seedling — and it starved a
/// colony walled in behind bushes that had all been picked down below the
/// threshold, which is exactly the population of bushes a hungry colony is
/// surrounded by. What keeps a garden from being mown down is not this
/// test; it is that a crop route is only ever asked for when no open route
/// exists at all.
pub fn harvestable(world: &World, at: GridIndex) -> bool {
    if !world.in_bounds(at) {
        return false;
    }
    let id = world.material_at(at);
    world.materials().grows(id)
        && world.materials().shed_form(id).is_some()
        && world.mass_at(at) > 0.0
}

/// Whether a route of this kind may enter `at`.
fn open(world: &World, at: GridIndex, through: Through) -> bool {
    steppable(world, at) || (through == Through::Crops && harvestable(world, at))
}

/// The cells a gnome standing at `from` can be in one step later, in the
/// order the walker itself tries them.
///
/// Gravity first, and exclusively: a gnome over a hole falls into it and
/// gets no say, so a route that walks it sideways off a ledge would be
/// wrong about where it ends up. Then each way along the row, and a climb
/// of one course only where the way along is blocked.
pub fn moves(world: &World, from: GridIndex, through: Through, out: &mut Vec<GridIndex>) {
    out.clear();
    let below = GridIndex::new(from.i, from.j - 1);
    if passable(world, below) {
        out.push(below);
        return;
    }
    for dir in [-1, 1] {
        let ahead = GridIndex::new(from.i + dir, from.j);
        let up = GridIndex::new(from.i + dir, from.j + 1);
        // Steppable first in both cases, so that a relaxed route climbs
        // over a hedge it could climb over and only cuts one it cannot.
        if steppable(world, ahead) {
            out.push(ahead);
        } else if steppable(world, up) {
            out.push(up);
        } else if open(world, ahead, through) {
            out.push(ahead);
        } else if open(world, up, through) {
            out.push(up);
        }
    }
    // Chimneying: with both ways along blocked, a gnome can get straight up
    // one course.
    //
    // Without this a gnome can step *down* into a hole anywhere and can only
    // climb back up diagonally, so a pit one cell wide is a trap it cannot
    // leave by any route — which is how a colony ended up living in the
    // garden's water bed, cutting the bushes over its head to get out. It is
    // not free wall-climbing: in open ground there is always a way along, so
    // this move never appears there.
    if out.is_empty() {
        let above = GridIndex::new(from.i, from.j + 1);
        if steppable(world, above) {
            out.push(above);
        }
    }
}

/// Not reached by the search.
const UNREACHED: u32 = u32::MAX;

/// A breadth-first flow field over [`moves`], from wherever a gnome is
/// standing.
///
/// Recomputed every step rather than committed to: the world moves under a
/// gnome's feet — pools rise, spoil falls, bushes seed — so a plan made
/// once and followed for fifty steps is a plan made about a world that has
/// gone. This is cheap enough to redo (a jar is under two thousand cells)
/// and the buffers are reused, so it costs no allocation per step.
#[derive(Debug, Default, Clone)]
pub struct Routes {
    width: usize,
    height: usize,
    start: usize,
    through: Through,
    dist: Vec<u32>,
    prev: Vec<u32>,
    queue: VecDeque<u32>,
    scratch: Vec<GridIndex>,
}

impl Routes {
    /// Floods the world with distances from `start`, in steps.
    pub fn explore(&mut self, world: &World, start: GridIndex, through: Through) {
        self.explore_avoiding(world, start, through, |_| false)
    }

    /// The same, refusing to route through any cell `avoid` names.
    ///
    /// What a gnome avoids is a gnome's business, not the graph's — the
    /// caller passes "anything hot enough to kill me", and the route comes
    /// back as though the fire were a wall. A gnome standing *in* one is
    /// still routed out of it, because the start cell is always its own
    /// first node: this stops a gnome walking into trouble, not out of it.
    pub fn explore_avoiding(
        &mut self,
        world: &World,
        start: GridIndex,
        through: Through,
        avoid: impl Fn(GridIndex) -> bool,
    ) {
        let n = world.width() * world.height();
        if self.dist.len() != n {
            self.dist = vec![UNREACHED; n];
            self.prev = vec![0; n];
        } else {
            self.dist.fill(UNREACHED);
        }
        self.width = world.width();
        self.height = world.height();
        self.through = through;
        self.queue.clear();
        if !world.in_bounds(start) {
            self.start = usize::MAX;
            return;
        }
        let from = world.linear_index(start);
        self.start = from;
        self.dist[from] = 0;
        self.prev[from] = from as u32;
        self.queue.push_back(from as u32);
        let mut scratch = std::mem::take(&mut self.scratch);
        while let Some(p) = self.queue.pop_front() {
            let here = self.at(p as usize);
            let step = self.dist[p as usize] + 1;
            moves(world, here, through, &mut scratch);
            for &n in scratch.iter() {
                let q = world.linear_index(n);
                if self.dist[q] != UNREACHED || avoid(n) {
                    continue;
                }
                self.dist[q] = step;
                self.prev[q] = p;
                self.queue.push_back(q as u32);
            }
        }
        self.scratch = scratch;
    }

    /// Where this field was flooded from.
    pub fn start(&self) -> GridIndex {
        self.at(self.start)
    }

    /// What kind of route this field holds.
    pub fn through(&self) -> Through {
        self.through
    }

    fn at(&self, p: usize) -> GridIndex {
        GridIndex::new(
            (p % self.width.max(1)) as i32,
            (p / self.width.max(1)) as i32,
        )
    }

    fn inside(&self, at: GridIndex) -> Option<usize> {
        (at.i >= 0 && at.j >= 0 && (at.i as usize) < self.width && (at.j as usize) < self.height)
            .then(|| at.j as usize * self.width + at.i as usize)
    }

    /// How many cells this gnome can get to at all, itself included — the
    /// size of the world as far as it is concerned.
    ///
    /// What it is for is telling "walled in" from "hungry". A gnome with
    /// the run of the jar and nothing to eat is not trapped and must not
    /// start cutting its way through the garden; a gnome that can reach
    /// eight cells is in a pen, whatever else is true.
    pub fn reach(&self) -> usize {
        self.dist.iter().filter(|&&d| d != UNREACHED).count()
    }

    /// Steps from the start to `at`, or `None` if a gnome cannot get there.
    pub fn steps_to(&self, at: GridIndex) -> Option<u32> {
        let p = self.inside(at)?;
        (self.dist[p] != UNREACHED).then_some(self.dist[p])
    }

    /// The first cell to move into on the way to `at` — `None` if `at` is
    /// unreachable or is where the gnome already stands.
    pub fn next_step(&self, at: GridIndex) -> Option<GridIndex> {
        let mut p = self.inside(at)?;
        if self.dist[p] == UNREACHED || p == self.start {
            return None;
        }
        // Walk the parent chain back to the cell whose parent is the start.
        let mut guard = self.dist.len() + 1;
        while self.prev[p] as usize != self.start {
            p = self.prev[p] as usize;
            guard -= 1;
            if guard == 0 {
                return None;
            }
        }
        Some(self.at(p))
    }

    /// The closest cell this gnome can reach that `wanted` accepts.
    ///
    /// Deterministic: ties go to the first in storage order, which is what
    /// keeps a run reproducible without an RNG.
    pub fn nearest(&self, wanted: impl Fn(GridIndex) -> bool) -> Option<GridIndex> {
        let mut best: Option<(u32, GridIndex)> = None;
        for (p, &d) in self.dist.iter().enumerate() {
            if d == UNREACHED {
                continue;
            }
            if best.is_some_and(|(bd, _)| d >= bd) {
                continue;
            }
            let at = self.at(p);
            if wanted(at) {
                best = Some((d, at));
            }
        }
        best.map(|(_, at)| at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::{terrarium as t, MaterialTable};
    use crate::world::World;

    /// A room with a floor, air above it, and nothing else.
    fn room(w: usize, h: usize) -> World {
        let mut world = World::new_open(w, h, MaterialTable::terrarium(), 290.0);
        for i in 0..w as i32 {
            world.fill(GridIndex::new(i, 0), t::STONE, 290.0);
        }
        world
    }

    #[test]
    fn a_gnome_walks_the_length_of_a_room() {
        let world = room(12, 6);
        let mut routes = Routes::default();
        routes.explore(&world, GridIndex::new(1, 1), Through::Open);
        assert_eq!(routes.steps_to(GridIndex::new(10, 1)), Some(9));
        assert_eq!(
            routes.next_step(GridIndex::new(10, 1)),
            Some(GridIndex::new(2, 1))
        );
    }

    #[test]
    fn a_route_climbs_one_course_and_not_two() {
        let mut world = room(12, 6);
        // A single step up in the floor: passable.
        world.fill(GridIndex::new(6, 1), t::STONE, 290.0);
        let mut routes = Routes::default();
        routes.explore(&world, GridIndex::new(1, 1), Through::Open);
        assert!(routes.steps_to(GridIndex::new(7, 2)).is_some());

        // Two courses: a wall, and nothing on the far side is reachable.
        world.fill(GridIndex::new(6, 2), t::STONE, 290.0);
        routes.explore(&world, GridIndex::new(1, 1), Through::Open);
        assert_eq!(routes.steps_to(GridIndex::new(10, 1)), None);
    }

    #[test]
    fn a_hedge_is_a_wall_until_the_gnome_is_willing_to_cut_it() {
        let mut world = room(12, 6);
        for j in 1..=3 {
            world.fill(GridIndex::new(6, j), t::JUNIPER, 290.0);
        }
        let far = GridIndex::new(10, 1);
        let mut routes = Routes::default();
        routes.explore(&world, GridIndex::new(1, 1), Through::Open);
        assert_eq!(routes.steps_to(far), None, "a full hedge blocks the way");

        routes.explore(&world, GridIndex::new(1, 1), Through::Crops);
        assert!(
            routes.steps_to(far).is_some(),
            "a gnome that will cut a crop can get past a hedge"
        );
        assert!(harvestable(&world, GridIndex::new(6, 1)));
        assert!(
            !harvestable(&world, GridIndex::new(5, 0)),
            "stone is not a crop"
        );
    }

    #[test]
    fn a_route_falls_before_it_walks() {
        let world = room(12, 6);
        let mut routes = Routes::default();
        // Standing in mid-air: the only move is down.
        routes.explore(&world, GridIndex::new(5, 4), Through::Open);
        assert_eq!(routes.steps_to(GridIndex::new(6, 4)), None);
        assert_eq!(
            routes.next_step(GridIndex::new(5, 1)),
            Some(GridIndex::new(5, 3))
        );
    }

    #[test]
    fn the_pool_is_not_a_road() {
        let mut world = room(12, 6);
        for i in 4..8 {
            world.fill(GridIndex::new(i, 1), t::WATER, 290.0);
        }
        let mut routes = Routes::default();
        routes.explore(&world, GridIndex::new(1, 1), Through::Open);
        assert_eq!(
            routes.steps_to(GridIndex::new(10, 1)),
            None,
            "a gnome does not wade across a pool on purpose"
        );
    }
}
