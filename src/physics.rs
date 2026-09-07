//! The simulation step: gravity-driven movement, heat conduction, and phase
//! change, in that order, over a [`World`].
//!
//! Every rule here is written so that mass and energy conservation is a
//! *property of the construction*, not something checked afterwards and
//! corrected:
//!
//! - **Movement** relocates whole cell records with [`World::swap_cells`].
//!   A swap cannot change a sum, so no arrangement of falling sand or
//!   rising steam can gain or lose a gram or a joule.
//! - **Conduction** computes one transfer per adjacent *pair* and applies
//!   it with opposite signs to the two cells, clamped so it can never
//!   overshoot the pair's shared equilibrium. Whatever leaves one cell
//!   arrives in the other.
//! - **Phase change** rewrites a cell's material and temperature together,
//!   using an identity that holds `mass * (c·T + latent_offset + progress)`
//!   fixed across the switch.
//!
//! The tests at the bottom assert exactly that, not just that things look
//! plausible: after hundreds of steps of a scenario with lava boiling water
//! into steam that condenses and rains back as ice, the world's total mass
//! and total energy still match their starting values.
//!
//! ## Why melting is a slow accumulation, not a threshold flip
//!
//! The obvious way to melt ice — "if T > 273.15, become water" — is wrong
//! in a way that matters. Melting a gram of ice costs 334 J, which is the
//! sensible heat of raising that same gram by 160 K. A cell that flips as
//! soon as it crosses the threshold either creates that energy from nowhere
//! or, if made to pay for it honestly, must first be heated 160 K past
//! melting point, which is absurd. Worse, the flip is oscillatory: the new
//! water is instantly below freezing and turns straight back to ice.
//!
//! So a cell instead *accumulates* latent energy at the threshold. Its
//! temperature pins at the transition point and further heat goes into
//! [`Cell::progress`] until the full latent heat is paid, at which point
//! the material switches and any surplus becomes temperature again. That is
//! also what makes ice's melt genuinely slow while the water around it
//! warms — the behaviour a mushy zone actually has.

use crate::material::{Mobility, Transition};
use crate::math::{GridIndex, Scalar};
use crate::world::{Cell, World, NO_PENDING};
use std::collections::HashMap;

/// One fixed simulation step: movement, then conduction, then phase change.
///
/// Order matters only for feel, not for conservation — each stage conserves
/// on its own. Movement runs first so freshly fallen material conducts
/// against its new neighbours in the same step.
pub fn step(world: &mut World, dt: Scalar) {
    apply_gravity(world);
    equalise_liquid_levels(world);
    conduct_heat(world, dt);
    apply_phase_changes(world);
    world.step_count = world.step_count.wrapping_add(1);
}

/// Gravity and buoyancy, as a single rule: a cell sinks past a neighbour
/// that is both lighter and able to get out of the way.
///
/// There is no separate "gases rise" rule, and deliberately so — steam
/// rising through water is the same event as water sinking through steam,
/// seen from the other cell. One rule, applied to the denser participant,
/// produces both.
pub fn apply_gravity(world: &mut World) {
    let (w, h) = (world.width() as i32, world.height() as i32);
    let mut moved = vec![false; (w * h) as usize];
    // Alternate the horizontal bias each step; a fixed preference makes
    // pools visibly creep in one direction over a long run.
    let rightward = world.step_count.is_multiple_of(2);
    let side = if rightward { 1 } else { -1 };

    // Two passes, and the order is load-bearing. Sinking is resolved for
    // the whole grid before anything slides sideways, because a single
    // combined pass lets a cell slide into the very hole that the cell
    // above it was about to fall through — and since a move marks both
    // cells as spent for the step, the fall never happens. Run together,
    // a shallow pool never fills its bottom row: it settles into a
    // permanently churning half-density sheet three rows deep. Run in
    // order, it fills from the bottom like water.
    for pass in [Pass::Sink, Pass::Spread] {
        for j in 0..h {
            // Bottom-up, so a cell that falls this step isn't re-examined
            // at its new position and allowed to fall twice.
            for i_raw in 0..w {
                let i = if rightward { i_raw } else { w - 1 - i_raw };
                let here = GridIndex::new(i, j);
                let p = world.linear_index(here);
                if moved[p] {
                    continue;
                }
                let material = world.material_of(p);
                if material.mobility == Mobility::Static {
                    continue;
                }
                // Only liquids spread sideways. Letting gases do it too is
                // tempting (they do diffuse, after all) but it makes every
                // open space churn permanently: an air cell and its
                // neighbour trade places forever because neither is
                // heavier, and the whole world shimmers. Gases still rise
                // and sink by density, which is the part that matters.
                let spreads = material.mobility == Mobility::Flowing
                    && material.phase == crate::material::Phase::Liquid;

                let target = match pass {
                    Pass::Sink => {
                        let down = [GridIndex::new(i, j - 1)];
                        let diagonals = [
                            GridIndex::new(i + side, j - 1),
                            GridIndex::new(i - side, j - 1),
                        ];
                        pick_target(world, p, &down, &moved)
                            .or_else(|| pick_target(world, p, &diagonals, &moved))
                    }
                    Pass::Spread if spreads => {
                        // Keep flowing the way this cell was already
                        // flowing — see `Cell::flow_dir`.
                        let preferred = match world.flow_dir(p) {
                            d if d != 0 => d as i32,
                            _ => side,
                        };
                        pick_target(
                            world,
                            p,
                            &[
                                GridIndex::new(i + preferred, j),
                                GridIndex::new(i - preferred, j),
                            ],
                            &moved,
                        )
                    }
                    Pass::Spread => None,
                };

                match (target, pass) {
                    (Some(q), Pass::Sink) => {
                        world.swap_cells(p, q);
                        moved[p] = true;
                        moved[q] = true;
                        world.set_flow_dir(q, 0);
                    }
                    (Some(q), Pass::Spread) => {
                        world.swap_cells(p, q);
                        moved[p] = true;
                        moved[q] = true;
                        // Remember which way it went, so next step it
                        // carries on rather than stepping straight back.
                        let dir = (q % world.width()) as i32 - i;
                        world.set_flow_dir(q, dir.signum() as i8);
                    }
                    (None, Pass::Spread) if spreads => {
                        // Blocked in the direction it was going: turn
                        // around, so a fluid that reaches a wall works its
                        // way back rather than pressing against it.
                        let d = world.flow_dir(p);
                        world.set_flow_dir(p, -d);
                    }
                    _ => {}
                }
            }
        }
    }
}

/// The two halves of [`apply_gravity`] — see the ordering note there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pass {
    Sink,
    Spread,
}

/// The first neighbour in `candidates` that this cell can sink into: in
/// bounds, not already moved this step, mobile enough to be displaced, and
/// genuinely lighter.
fn pick_target(world: &World, p: usize, candidates: &[GridIndex], moved: &[bool]) -> Option<usize> {
    let here_density = world.material_of(p).density;
    for &c in candidates {
        if !world.in_bounds(c) {
            continue;
        }
        let q = world.linear_index(c);
        if moved[q] {
            continue;
        }
        let there = world.material_of(q);
        if there.mobility == Mobility::Static {
            continue;
        }
        // A strict margin, not `<`: without it two materials of equal
        // density swap back and forth forever.
        if there.density < here_density * 0.999 {
            return Some(q);
        }
    }
    None
}

/// Hydrostatic levelling: the rule that lets a connected body of liquid
/// climb.
///
/// Cell-by-cell gravity can only ever move something down or sideways, so
/// on its own it cannot reproduce the behaviour north star #3 asks for by
/// name — *"a U-shaped pipe brings water to a level across both arms"*.
/// Water poured into one arm runs to the bottom, and stops. Getting it up
/// the far arm needs pressure, and pressure is a property of a whole
/// connected body of fluid rather than of any one cell.
///
/// Rather than solve a pressure field (the rock the `stable-fluids`
/// experiment split on — see `JOURNAL.md`), this exploits the fact that for
/// a liquid at rest the pressure field's only consequence is the one thing
/// we want: every free surface of one connected body sits at one height.
/// So: find the connected bodies, take each one's highest occupied row, and
/// let any surface cell more than one row below that rise into the lighter
/// cell above it. The liquid climbs one row per step, and stops when the
/// surfaces agree.
///
/// Still a swap, so still conserving. The "more than one row" margin is
/// what keeps two arms that already agree to within a cell from trading a
/// cell back and forth forever.
pub fn equalise_liquid_levels(world: &mut World) {
    let (w, h) = (world.width(), world.height());
    let n = w * h;
    let mut parent: Vec<usize> = (0..n).collect();

    fn find(parent: &mut [usize], mut x: usize) -> usize {
        while parent[x] != x {
            parent[x] = parent[parent[x]];
            x = parent[x];
        }
        x
    }

    let is_liquid = |world: &World, p: usize| {
        let m = world.material_of(p);
        m.mobility == Mobility::Flowing && m.phase == crate::material::Phase::Liquid
    };
    let is_open = |world: &World, p: usize| world.material_of(p).mobility != Mobility::Static;

    // Connected components of *open* (non-static) cells — the cavity a
    // fluid body lives in. Not components of liquid: a pocket of trapped
    // air at the bottom of a U-bend would sever a body that is physically
    // one connected column of water, and then neither arm would know about
    // the other.
    for j in 0..h as i32 {
        for i in 0..w as i32 {
            let p = world.linear_index(GridIndex::new(i, j));
            if !is_open(world, p) {
                continue;
            }
            for n_idx in [GridIndex::new(i + 1, j), GridIndex::new(i, j + 1)] {
                if !world.in_bounds(n_idx) {
                    continue;
                }
                let q = world.linear_index(n_idx);
                if !is_open(world, q) {
                    continue;
                }
                let (ra, rb) = (find(&mut parent, p), find(&mut parent, q));
                if ra != rb {
                    parent[ra] = rb;
                }
            }
        }
    }

    // Per cavity, per column: the highest row that column holds liquid at.
    let mut column_top: HashMap<(usize, usize), i32> = HashMap::new();
    for p in 0..n {
        if !is_liquid(world, p) {
            continue;
        }
        let root = find(&mut parent, p);
        let (i, j) = (p % w, (p / w) as i32);
        let e = column_top.entry((root, i)).or_insert(j);
        *e = (*e).max(j);
    }

    // The tallest column of each cavity is the donor.
    let mut donor: HashMap<usize, (usize, i32)> = HashMap::new();
    for (&(root, i), &j) in &column_top {
        let e = donor.entry(root).or_insert((i, j));
        if j > e.1 {
            *e = (i, j);
        }
    }

    // Transfer, rather than lift. An earlier attempt raised a column by
    // shuffling its own cells up one row, which leaves a hole underneath
    // that gravity closes on the very next step: the two rules deadlock and
    // the water never actually crosses. Moving the donor's *surface* cell
    // straight onto the recipient's surface leaves both columns contiguous
    // and makes progress every step. It is one swap, so it conserves like
    // everything else here.
    let mut used_donor: HashMap<usize, bool> = HashMap::new();
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    for (&(root, i), &top_j) in &column_top {
        let (donor_col, donor_top) = donor[&root];
        if donor_col == i || top_j > donor_top - 2 {
            continue;
        }
        if *used_donor.get(&root).unwrap_or(&false) {
            continue;
        }
        let surface = GridIndex::new(i as i32, top_j + 1);
        if !world.in_bounds(surface) {
            continue;
        }
        let src = world.linear_index(GridIndex::new(donor_col as i32, donor_top));
        let dst = world.linear_index(surface);
        let there = world.material_of(dst);
        if there.mobility == Mobility::Static
            || there.density >= world.material_of(src).density * 0.999
        {
            continue;
        }
        if !laterally_confined(world, i as i32, top_j + 1) {
            continue;
        }
        used_donor.insert(root, true);
        pairs.push((src, dst));
    }
    // Sorted so the outcome doesn't depend on hash iteration order.
    pairs.sort_unstable();
    for (src, dst) in pairs {
        world.swap_cells(src, dst);
    }
}

/// Whether the cell at `(i, j)` is walled in on both sides — the proxy this
/// module uses for "inside a pipe or vessel arm, where water can be pushed
/// up".
///
/// Without it, hydrostatic levelling would happily lift water out of an
/// open lake into thin air just because some puddle on a ledge elsewhere in
/// the same cavity sat higher. The wall test confines the effect to the
/// place it is actually right: a confined column, which is exactly the
/// U-bend case. A wide, open vessel therefore levels by ordinary sideways
/// flow alone and not by this pass — a known limit, and the honest one to
/// have, since the alternative is a pressure solve.
fn laterally_confined(world: &World, i: i32, j: i32) -> bool {
    [-1i32, 1].into_iter().all(|d| {
        let n = GridIndex::new(i + d, j);
        !world.in_bounds(n) || world.material_of(world.linear_index(n)).mobility == Mobility::Static
    })
}

/// Heat conduction between orthogonally adjacent cells.
///
/// One transfer per pair, applied with opposite signs, so the sum of
/// `mass * c * T` over the pair is unchanged to the last bit the arithmetic
/// allows. The transfer is clamped to the energy that would bring the pair
/// exactly to its shared equilibrium temperature, which keeps the scheme
/// unconditionally stable: no timestep, however large, can make heat
/// overshoot and oscillate.
pub fn conduct_heat(world: &mut World, dt: Scalar) {
    let (w, h) = (world.width() as i32, world.height() as i32);
    for j in 0..h {
        for i in 0..w {
            let here = GridIndex::new(i, j);
            let p = world.linear_index(here);
            // Only the +i and +j neighbours, so each pair is visited once.
            for n in [GridIndex::new(i + 1, j), GridIndex::new(i, j + 1)] {
                if !world.in_bounds(n) {
                    continue;
                }
                let q = world.linear_index(n);
                exchange_heat(world, p, q, dt);
            }
        }
    }
}

fn exchange_heat(world: &mut World, p: usize, q: usize, dt: Scalar) {
    let (ma, mb) = (world.material_of(p), world.material_of(q));
    let (ka, kb) = (ma.conductivity, mb.conductivity);
    if ka <= 0.0 || kb <= 0.0 {
        return;
    }
    // Series conductance: heat crossing the boundary passes through both
    // materials, so the poorer conductor governs — a harmonic mean, not an
    // arithmetic one.
    let k = 2.0 * ka * kb / (ka + kb);

    let cell_a = world.cell_at(p);
    let cell_b = world.cell_at(q);
    let cap_a = cell_a.mass * ma.heat_capacity;
    let cap_b = cell_b.mass * mb.heat_capacity;
    if cap_a <= 0.0 || cap_b <= 0.0 {
        return;
    }

    let (ta, tb) = (cell_a.temperature, cell_b.temperature);
    let mut q_joules = k * (tb - ta) * dt;
    // Clamp to the equilibrium transfer, in whichever direction heat is
    // actually flowing.
    let equilibrium = (cap_a * ta + cap_b * tb) / (cap_a + cap_b);
    let max_transfer = cap_a * (equilibrium - ta);
    if (q_joules > 0.0 && q_joules > max_transfer) || (q_joules < 0.0 && q_joules < max_transfer) {
        q_joules = max_transfer;
    }

    let mut a = cell_a;
    let mut b = cell_b;
    a.temperature = ta + q_joules / cap_a;
    b.temperature = tb - q_joules / cap_b;
    world.set_cell_at(p, a);
    world.set_cell_at(q, b);
}

/// Phase changes: melting, freezing, boiling, condensing, and rock melt,
/// all through the same data-driven accumulation described in the module
/// doc comment.
pub fn apply_phase_changes(world: &mut World) {
    let transitions: Vec<Transition> = world.transitions().to_vec();
    let latents: Vec<Scalar> = transitions.iter().map(|t| world.latent_of(t)).collect();

    for p in 0..world.width() * world.height() {
        let mut cell = world.cell_at(p);

        // Pick up where this cell left off, or find a transition it has
        // newly crossed the threshold of.
        if cell.pending == NO_PENDING || transitions[cell.pending as usize].from != cell.material {
            cell.pending = NO_PENDING;
            cell.progress = 0.0;
            for (idx, tr) in transitions.iter().enumerate() {
                if tr.from != cell.material {
                    continue;
                }
                let crossed = if World::is_heating(tr) {
                    cell.temperature > tr.threshold_k
                } else {
                    cell.temperature < tr.threshold_k
                };
                if crossed {
                    cell.pending = idx as u8;
                    break;
                }
            }
        }
        if cell.pending == NO_PENDING {
            continue;
        }

        let tr = transitions[cell.pending as usize];
        let latent = latents[cell.pending as usize];
        let c_from = world.materials().get(tr.from).heat_capacity;
        if c_from <= 0.0 || latent == 0.0 {
            cell.pending = NO_PENDING;
            world.set_cell_at(p, cell);
            continue;
        }

        // Move the cell's departure from the threshold into latent
        // progress. Positive while melting/boiling, negative while
        // freezing/condensing; `latent` carries the matching sign.
        let excess = c_from * (cell.temperature - tr.threshold_k);
        cell.temperature = tr.threshold_k;
        cell.progress += excess;

        let complete = if latent > 0.0 {
            cell.progress >= latent
        } else {
            cell.progress <= latent
        };
        let abandoned = if latent > 0.0 {
            cell.progress <= 0.0
        } else {
            cell.progress >= 0.0
        };

        if complete {
            // The switch. Energy before is `c_from·Θ + L_from + progress`;
            // setting the new temperature to `Θ + (progress - latent)/c_to`
            // makes energy after identical, because `latent` is by
            // definition `(c_to·Θ + L_to) - (c_from·Θ + L_from)`.
            let c_to = world.materials().get(tr.to).heat_capacity;
            let surplus = cell.progress - latent;
            cell.material = tr.to;
            cell.temperature = tr.threshold_k + if c_to > 0.0 { surplus / c_to } else { 0.0 };
            cell.progress = 0.0;
            cell.pending = NO_PENDING;
        } else if abandoned {
            // Heat went back the other way before the change completed —
            // give the accumulated latent energy back as temperature.
            cell.temperature = tr.threshold_k + cell.progress / c_from;
            cell.progress = 0.0;
            cell.pending = NO_PENDING;
        }

        world.set_cell_at(p, cell);
    }
}

/// Convenience for tests and scenarios: run `n` steps of `dt`.
pub fn run(world: &mut World, n: u32, dt: Scalar) {
    for _ in 0..n {
        step(world, dt);
    }
}

/// A cell's total energy, exposed for tests that want to reason about one
/// cell rather than the whole world.
pub fn cell_energy(world: &World, index: GridIndex) -> f64 {
    Cell::energy(&world.cell(index), world.materials())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::{terrarium as t, MaterialTable};
    use crate::world::World;

    fn air_world(w: usize, h: usize) -> World {
        World::new(w, h, MaterialTable::terrarium(), t::AIR, 290.0)
    }

    // --- Movement ---

    #[test]
    fn sand_falls_to_the_floor_and_stays_there() {
        let mut w = air_world(3, 8);
        w.fill(GridIndex::new(1, 7), t::SAND, 290.0);
        w.rebaseline();
        run(&mut w, 20, 0.05);
        assert_eq!(w.material_at(GridIndex::new(1, 0)), t::SAND);
        assert_eq!(w.count_of(t::SAND), 1);
    }

    #[test]
    fn sand_piles_into_a_heap_instead_of_a_tower() {
        let mut w = air_world(9, 12);
        for k in 0..8 {
            w.fill(GridIndex::new(4, 3 + k), t::SAND, 290.0);
        }
        w.rebaseline();
        run(&mut w, 120, 0.05);
        // A column of sand dropped on a floor must spread: the pile can't
        // still be one cell wide.
        let bottom_width = (0..9)
            .filter(|&i| w.material_at(GridIndex::new(i, 0)) == t::SAND)
            .count();
        assert!(
            bottom_width >= 3,
            "sand should spread into a heap, bottom row width was {bottom_width}"
        );
        assert_eq!(w.count_of(t::SAND), 8, "no grain may be lost");
    }

    #[test]
    fn water_finds_its_level_across_a_u_shaped_pipe() {
        // North star #3's own example, as a test: two arms joined at the
        // bottom, water poured into one arm only, must end up level in both.
        let mut w = air_world(7, 10);
        for j in 0..10 {
            w.fill(GridIndex::new(3, j), t::STONE, 290.0);
        }
        // Open the base of the divider so the two arms connect.
        w.fill(GridIndex::new(3, 0), t::AIR, 290.0);
        for j in 0..10 {
            w.fill(GridIndex::new(0, j), t::STONE, 290.0);
            w.fill(GridIndex::new(6, j), t::STONE, 290.0);
        }
        for j in 0..10 {
            w.fill(GridIndex::new(1, j), t::STONE, 290.0);
            w.fill(GridIndex::new(5, j), t::STONE, 290.0);
        }
        // Left arm: cells (2, 0..) — right arm: cells (4, 0..).
        for j in 0..8 {
            w.fill(GridIndex::new(2, j), t::AIR, 290.0);
            w.fill(GridIndex::new(4, j), t::AIR, 290.0);
        }
        // Pour seven cells of water into the left arm only: three per arm
        // plus the junction cell that joins them, so a genuinely level
        // solution exists.
        for j in 1..8 {
            w.fill(GridIndex::new(2, j), t::WATER, 290.0);
        }
        w.rebaseline();
        run(&mut w, 400, 0.05);

        let left = (0..10)
            .filter(|&j| w.material_at(GridIndex::new(2, j)) == t::WATER)
            .count();
        let right = (0..10)
            .filter(|&j| w.material_at(GridIndex::new(4, j)) == t::WATER)
            .count();
        // (3, 0) is the junction joining the two arms, in neither of them.
        let junction = usize::from(w.material_at(GridIndex::new(3, 0)) == t::WATER);
        assert_eq!(
            left + right + junction,
            7,
            "no water may be lost in the pipe"
        );
        assert_eq!(
            junction, 1,
            "the junction joining the arms should stay full"
        );
        assert!(
            left.abs_diff(right) <= 1,
            "water should find its level across both arms, got left={left} right={right}"
        );
    }

    #[test]
    fn a_resting_pool_stays_flat() {
        let mut w = air_world(8, 6);
        for i in 0..8 {
            for j in 0..2 {
                w.fill(GridIndex::new(i, j), t::WATER, 290.0);
            }
        }
        w.rebaseline();
        run(&mut w, 100, 0.05);
        for i in 0..8 {
            for j in 0..2 {
                assert_eq!(
                    w.material_at(GridIndex::new(i, j)),
                    t::WATER,
                    "cell ({i},{j}) should still be water in a resting pool"
                );
            }
        }
    }

    #[test]
    fn steam_rises_through_water_without_a_rule_that_says_so() {
        let mut w = air_world(3, 8);
        for j in 0..6 {
            for i in 0..3 {
                w.fill(GridIndex::new(i, j), t::WATER, 300.0);
            }
        }
        w.fill(GridIndex::new(1, 0), t::STEAM, 380.0);
        w.rebaseline();
        apply_gravity(&mut w);
        assert_ne!(
            w.material_at(GridIndex::new(1, 0)),
            t::STEAM,
            "a steam bubble at the bottom of a water column must rise"
        );
    }

    // --- Conduction ---

    #[test]
    fn heat_flows_from_hot_to_cold_and_stops_at_equilibrium() {
        let mut w = air_world(2, 1);
        w.fill(GridIndex::new(0, 0), t::STONE, 400.0);
        w.fill(GridIndex::new(1, 0), t::STONE, 300.0);
        w.rebaseline();
        let start = w.total_energy();
        for _ in 0..500 {
            conduct_heat(&mut w, 0.05);
        }
        let a = w.cell(GridIndex::new(0, 0)).temperature;
        let b = w.cell(GridIndex::new(1, 0)).temperature;
        assert!((a - b).abs() < 0.5, "should equalise, got {a} and {b}");
        assert!((a - 350.0).abs() < 0.5, "equal masses meet in the middle");
        assert!((w.total_energy() - start).abs() < 1e-3, "energy conserved");
    }

    #[test]
    fn conduction_never_overshoots_even_at_an_absurd_timestep() {
        let mut w = air_world(2, 1);
        w.fill(GridIndex::new(0, 0), t::STONE, 400.0);
        w.fill(GridIndex::new(1, 0), t::STONE, 300.0);
        w.rebaseline();
        conduct_heat(&mut w, 1_000_000.0);
        let a = w.cell(GridIndex::new(0, 0)).temperature;
        let b = w.cell(GridIndex::new(1, 0)).temperature;
        assert!(
            (a - 350.0).abs() < 0.01 && (b - 350.0).abs() < 0.01,
            "an enormous dt should land exactly on equilibrium, got {a} and {b}"
        );
    }

    // --- Phase change ---

    #[test]
    fn ice_takes_its_full_latent_heat_to_melt() {
        let mut w = air_world(1, 1);
        w.fill(GridIndex::new(0, 0), t::ICE, 273.0);
        w.rebaseline();
        let idx = GridIndex::new(0, 0);
        let mass = w.cell(idx).mass as f64;

        // Feed in slightly less than the latent heat: still ice.
        w.conjure_energy(idx, mass * 333.0 + mass * 2.093 * 0.2);
        apply_phase_changes(&mut w);
        assert_eq!(w.material_at(idx), t::ICE, "should not melt early");
        assert!(
            (w.cell(idx).temperature - 273.15).abs() < 0.01,
            "a melting cell pins at the melting point"
        );

        // Then the last couple of joules: now it melts.
        w.conjure_energy(idx, mass * 5.0);
        apply_phase_changes(&mut w);
        assert_eq!(w.material_at(idx), t::WATER, "should melt once paid for");
        assert!(w.conservation_residuals().energy.abs() < 1e-3);
    }

    #[test]
    fn melting_and_refreezing_returns_to_the_starting_state() {
        let mut w = air_world(1, 1);
        w.fill(GridIndex::new(0, 0), t::ICE, 270.0);
        w.rebaseline();
        let idx = GridIndex::new(0, 0);
        let mass = w.cell(idx).mass as f64;
        let start_energy = w.total_energy();

        w.conjure_energy(idx, mass * 400.0);
        apply_phase_changes(&mut w);
        assert_eq!(w.material_at(idx), t::WATER);
        w.conjure_energy(idx, -(mass * 400.0));
        for _ in 0..3 {
            apply_phase_changes(&mut w);
        }
        assert_eq!(w.material_at(idx), t::ICE, "should freeze back");
        assert!(
            (w.total_energy() - start_energy).abs() < 1e-2,
            "a round trip through melting must be energy-neutral"
        );
        assert!(
            (w.cell(idx).temperature - 270.0).abs() < 0.1,
            "and land back at the starting temperature, got {}",
            w.cell(idx).temperature
        );
    }

    #[test]
    fn a_water_cell_does_not_oscillate_between_phases_at_the_threshold() {
        let mut w = air_world(1, 1);
        w.fill(GridIndex::new(0, 0), t::WATER, 273.15);
        w.rebaseline();
        let mut flips = 0;
        let mut last = w.material_at(GridIndex::new(0, 0));
        for _ in 0..200 {
            apply_phase_changes(&mut w);
            let now = w.material_at(GridIndex::new(0, 0));
            if now != last {
                flips += 1;
                last = now;
            }
        }
        assert!(flips <= 1, "phase should settle, saw {flips} flips");
    }

    // --- The standing invariant ---

    #[test]
    fn a_violent_closed_world_conserves_mass_and_energy_over_a_long_run() {
        // Lava at the bottom, ice above it, water and sand in between:
        // melting, boiling, condensing, falling and piling all at once.
        let mut w = air_world(16, 24);
        for i in 0..16 {
            for j in 0..2 {
                w.fill(GridIndex::new(i, j), t::LAVA, 1800.0);
            }
        }
        for i in 0..16 {
            for j in 2..8 {
                w.fill(GridIndex::new(i, j), t::WATER, 300.0);
            }
        }
        for i in 4..12 {
            for j in 14..18 {
                w.fill(GridIndex::new(i, j), t::ICE, 250.0);
            }
        }
        for i in 0..16 {
            w.fill(GridIndex::new(i, 20), t::SAND, 290.0);
        }
        w.rebaseline();

        let mass0 = w.total_mass();
        let energy0 = w.total_energy();
        run(&mut w, 600, 0.05);

        let r = w.conservation_residuals();
        assert!(
            r.mass_relative.abs() < 1e-6,
            "mass drifted by {} ({:e} relative) from {mass0}",
            r.mass,
            r.mass_relative
        );
        assert!(
            r.energy_relative.abs() < 1e-6,
            "energy drifted by {} ({:e} relative) from {energy0}",
            r.energy,
            r.energy_relative
        );
        // And it must actually have done something interesting.
        assert!(
            w.count_of(t::STEAM) > 0 || w.count_of(t::WATER) > 24,
            "the scenario should have produced steam or melted the ice"
        );
    }

    #[test]
    fn nothing_leaks_through_the_world_boundary() {
        let mut w = air_world(6, 6);
        for i in 0..6 {
            for j in 0..3 {
                w.fill(GridIndex::new(i, j), t::WATER, 350.0);
            }
        }
        w.rebaseline();
        let cells = 36;
        run(&mut w, 200, 0.05);
        let total: usize = t::ALL.iter().map(|&m| w.count_of(m)).sum();
        assert_eq!(total, cells, "every cell must still hold some material");
        assert_eq!(w.count_of(t::WATER), 18, "water may not escape the box");
    }
}
