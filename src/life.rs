//! Life: the biology tier, run as one generic pass over living cells.
//!
//! `NORTH_STARS.md` #2 fixes the order this project works through —
//! *physics, then chemistry, then biology, then a game layer*. Nights 1–2
//! finished the physics tier's headline items, night 3 opened chemistry and
//! night 4 finished the gas half of physics. This is the first of the next
//! tier, and, like `src/chemistry.rs` before it, it is deliberately the
//! smallest thing that deserves the name: a table of [`Metabolism`] rows
//! saying what a living cell takes in and gives back, and one pass that
//! runs them.
//!
//! ## Why chemistry could not already do this
//!
//! `src/chemistry.rs` says it outright: a [`Reaction`](crate::material::Reaction)
//! has no stoichiometry, because each cell keeps its own mass and only
//! changes what it is called. That is enough for "wood becomes charcoal" and
//! not enough for anything alive, because life is *proportions*. A gram of
//! plant is 1.47 g of carbon dioxide plus 0.6 g of water minus 1.07 g of
//! oxygen, and if those ratios are approximated, the carbon in a sealed jar
//! is not conserved even while its total mass is — the jar would quietly
//! turn air into plant, which is precisely the class of lie `NORTH_STARS.md`
//! #4 exists to stamp out.
//!
//! ## Conservation, by construction, a third time
//!
//! - **Mass.** The table asserts `Σ intake == Σ output` at construction.
//!   At runtime, every gram removed from a donor cell is added to the host,
//!   and every gram removed from the host is added to an acceptor cell.
//!   There is no ledger entry anywhere in this file: nothing here is magic.
//! - **Energy.** Each parcel of mass carries its own enthalpy —
//!   `grams · (c·T + L)` at the temperature of the cell it leaves — and the
//!   *host* settles the balance, its temperature re-solved from what it now
//!   holds. So the heat of a living process is emergent in exactly the sense
//!   a reaction's is: it is the enthalpy the participants dropped, showing up
//!   as the plant's own temperature, and it cannot be anything else. That is
//!   why photosynthesis cools the bush that runs it and its reverse warms
//!   one.
//!
//! ## What a step is limited by
//!
//! Four things, each of them a reason a real plant stops growing:
//!
//! 1. **Light** — [`Metabolism::light_min`]/`light_max`, read off
//!    `src/light.rs`'s field. Shade is a real limit, and the day/night cycle
//!    is what makes photosynthesis and respiration alternate rather than
//!    cancel.
//! 2. **Temperature** — a band, outside which nothing happens.
//! 3. **Reagents** — turnover is capped by the scarcest one actually within
//!    reach. In a sealed jar this is the binding constraint almost always,
//!    and it is the good kind of constraint: a bush can only grow as fast as
//!    something in the jar breathes out carbon dioxide for it.
//! 4. **Room** — a host cannot grow past a full cell, and cannot shrink past
//!    nothing. A full plant *spreads* instead, into a neighbouring cell of
//!    air, by splitting its own mass in two — see [`spread`].
//!
//! [`Metabolism`]: crate::material::Metabolism

use crate::material::{MaterialId, Metabolism, Phase};
use crate::math::{GridIndex, Scalar};
use crate::world::{Cell, World, NO_MIX, NO_PENDING};

/// The most a single metabolic step may move a host cell's temperature, K.
///
/// The same clamp `physics::conduct_heat` puts on a pairwise transfer, and
/// for the same reason: a host cell's heat capacity is small (a cell of
/// juniper is 1 J/K), the enthalpy of a living process is large per gram,
/// and an unclamped step that happened to find plenty of reagent would swing
/// a bush by tens of kelvin in one tick and then swing it back. Capping the
/// *turnover* by what this allows is not a fudge on the energy — every joule
/// still moves with its gram — it is a statement that a plant cannot run
/// faster than it can shed the heat of running.
const MAX_HOST_SWING_K: Scalar = 0.5;

/// Fraction of a cell a plant must fill before it can seed a neighbour.
const SPREAD_AT: Scalar = 0.98;

/// Fraction of a cell below which what is left of it is a crumb, and may be
/// taken whole rather than halved — see [`takeable`].
const CRUMB: Scalar = 0.1;

/// A running tally of what life has done — a diary, not a ledger, in exactly
/// the sense [`crate::vapour::Tally`] is one: nothing here is a hole in the
/// books, it is just what moved.
///
/// It earns its place because it is the *only* way to state the jar's water
/// balance any more. Water used to be conserved species-by-species —
/// whatever left the pool came back as rain — and that made a very strong
/// test. It is not true now, and should not be: a plant takes water apart to
/// build itself and puts it back when it respires, so the free water in a
/// terrarium genuinely rises and falls with how much plant is standing in
/// it. What is still exactly true is that the difference is *this*, times
/// the declared proportion.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Tally {
    /// Grams of living matter built by metabolisms that grow their host.
    pub grown_g: f64,
    /// Grams of living matter spent by metabolisms that consume their host.
    pub respired_g: f64,
    /// Net grams of each mixture species life has put into the world, by
    /// slot: positive for produced, negative for consumed.
    ///
    /// This is the exact version of what `grown_g`/`respired_g` were being
    /// used as a proxy for. Those two work as a water balance only while
    /// every living process moves the same amount of water per gram of host,
    /// and the moment a bush could shed a dead leaf — mass leaving the host
    /// with no water in it at all — the proxy was wrong by whatever had been
    /// shed. This is not a proxy: it is the grams.
    pub moved: [f64; crate::material::MIX_SLOTS],
}

/// One pass of every living process in the world's table, followed by
/// [`spread`].
///
/// Runs after chemistry in `physics::step`, so a cell that has just become
/// something else this step is not also asked to live.
pub fn step(world: &mut World, dt: Scalar) {
    let metabolisms = world.materials().metabolisms().to_vec();
    if metabolisms.is_empty() {
        return;
    }
    let n = world.width() * world.height();
    // Every process gets its own look at every cell, including cells another
    // one has already worked on this step. A cell used to be marked busy the
    // moment anything ran in it, and that quietly made "leaf fall" dead
    // letter: a lit bush photosynthesises every step, so it would never have
    // reached the row that sheds. Nothing is lost by allowing both, because
    // each process re-reads the cell and re-caps against what is actually
    // there — the caps below are computed from the cell as it stands, not
    // from the cell as it stood at the top of the step.
    for m in &metabolisms {
        for p in 0..n {
            if world.material_cells()[p] != m.host {
                continue;
            }
            metabolise(world, p, m, dt);
        }
    }
    spread(world, &metabolisms);
}

/// Runs one metabolism once in the cell at `p`, if its conditions are met
/// and its reagents can be found. Returns whether anything happened.
fn metabolise(world: &mut World, p: usize, m: &Metabolism, dt: Scalar) -> bool {
    let host = world.cell_at(p);
    if host.pending != NO_PENDING {
        return false;
    }
    let light = world.light_at(p);
    if !m.lit(light) {
        return false;
    }
    if host.temperature < m.min_k || host.temperature > m.max_k {
        return false;
    }

    // How much turnover this step wants, before anything says no — see
    // [`Metabolism::light_drive`].
    let mut units = m.rate * m.light_drive(light) * dt;
    if units <= 0.0 {
        return false;
    }

    // --- Cap 1: the host's own room to grow or shrink ---
    let density = world.materials().get(m.host).density;
    let gain = m.host_gain();
    if gain > 0.0 {
        units = units.min((density - host.mass).max(0.0) / gain);
    } else if gain < 0.0 {
        // Never all the way to nothing: a cell with no mass has no heat
        // capacity, so the energy it was holding would have nowhere to go
        // and would simply vanish. A plant that has spent itself down to a
        // twentieth of a cell has stopped either way.
        units = units.min(((host.mass - 0.05 * density) / -gain).max(0.0));
    }
    if units <= 0.0 {
        return false;
    }

    // --- Cap 2: the scarcest reagent actually within reach ---
    let neighbours: Vec<usize> = neighbours(world, p).collect();
    for r in &m.intake {
        if r.material == m.host {
            continue;
        }
        let available: Scalar = neighbours
            .iter()
            .map(|&q| takeable(world, q, r.material))
            .sum();
        if r.grams <= 0.0 {
            continue;
        }
        units = units.min(available / r.grams);
    }
    if units <= 1e-15 {
        return false;
    }

    // --- Cap 3: the heat the host would have to absorb or shed ---
    //
    // Computed against the enthalpy the table already derived, so it is the
    // same number the transfer below will actually produce.
    let table = world.materials();
    let per_unit: f64 = m
        .output
        .iter()
        .map(|r| r.grams * table.get(r.material).specific_energy(host.temperature))
        .sum::<Scalar>()
        - m.intake
            .iter()
            .map(|r| r.grams * table.get(r.material).specific_energy(host.temperature))
            .sum::<Scalar>();
    let host_capacity = host.capacity(table).max(1e-12);
    if per_unit != 0.0 {
        units = units.min(MAX_HOST_SWING_K * host_capacity / per_unit.abs());
    }
    if units <= 1e-15 {
        return false;
    }

    // --- Cap 4: somewhere to put a product that is not a gas ---
    //
    // A gas product always has somewhere to go: any neighbouring gas cell
    // takes it into its mixture. A solid one needs either a cell already made
    // of it with room left, or an empty cell it can push the air out of — and
    // if it has neither, the process does not run at all, which is why a bush
    // walled in by its own hedge stops shedding. Checked *before* anything is
    // taken, because a metabolism that has already moved mass and then finds
    // it cannot place a product has no honest way to put it back.
    for r in &m.output {
        if r.material == m.host || r.grams <= 0.0 {
            continue;
        }
        if world.materials().slot(r.material).is_some() {
            // A gas or a mist, and any neighbouring gas cell will take any
            // amount of it — but there has to *be* one. A cell walled in by
            // solids that ran anyway used to hand its carbon dioxide back to
            // its host as more host, which conserves grams and quietly
            // rewrites what the grams are made of: the jar's mass held and
            // its carbon did not.
            if !neighbours.iter().any(|&q| world.is_gas_at(q)) {
                return false;
            }
            continue;
        }
        let room: Scalar = neighbours
            .iter()
            .map(|&q| placeable(world, q, r.material))
            .sum();
        units = units.min(room / r.grams);
    }
    if units <= 1e-15 {
        return false;
    }

    // What this turnover moves, species by species. Not a proxy and not a
    // ledger — see [`Tally`].
    for (sign, reagents) in [(-1.0, &m.intake), (1.0, &m.output)] {
        for r in reagents {
            if let Some(s) = world.materials().slot(r.material) {
                world.life.moved[s] += sign * r.grams * units;
            }
        }
    }

    // --- Take ---
    //
    // Every gram leaves its donor at that donor's own temperature, carrying
    // exactly its own enthalpy, and the donor's temperature is unchanged by
    // losing it — the identity `World::conjure_mass` already relies on when
    // it banishes part of a cell.
    let mut gathered: f64 = 0.0;
    let mut host = world.cell_at(p);
    for r in &m.intake {
        let wanted = r.grams * units;
        if r.material == m.host {
            continue;
        }
        let mut left = wanted;
        for &q in &neighbours {
            if left <= 0.0 {
                break;
            }
            let got = take(world, q, r.material, left);
            gathered += got.1;
            left -= got.0;
        }
    }
    // The host's own share of the intake is simply mass it loses.
    let host_intake: Scalar = m
        .intake
        .iter()
        .filter(|r| r.material == m.host)
        .map(|r| r.grams * units)
        .sum();
    let host_output: Scalar = m
        .output
        .iter()
        .filter(|r| r.material == m.host)
        .map(|r| r.grams * units)
        .sum();

    // --- Give ---
    //
    // Every parcel leaves at the host's temperature *before* this step's
    // enthalpy change, and that same figure is what the host is debited.
    // Using the post-step temperature for one and not the other is a leak,
    // and it is not a small one: it was the whole of an energy residual of
    // 2e-8 relative in a few hundred steps of a greenhouse, found by the
    // conservation test and by nothing else.
    let out_t = host.temperature;
    let energy_before = host.energy(world.materials()) + gathered;
    let mut given: f64 = 0.0;
    let mut produced: Vec<(MaterialId, Scalar)> = Vec::new();
    for r in &m.output {
        if r.material == m.host {
            continue;
        }
        produced.push((r.material, r.grams * units));
    }
    // Gases and mists first, then anything solid — and the order is not
    // cosmetic, it is the difference between the jar's carbon balancing and
    // not. A gas joins a neighbouring gas cell and leaves it a gas cell; a
    // solid *consumes* one, by pushing its contents out and taking its place.
    // Run the other way round, a rot with one cell of air beside it put its
    // mould there first and then had nowhere to breathe out, and the carbon
    // dioxide it could not place was handed back to the host — as more host.
    // Mass held; six micrograms of dead leaf a step turned into six
    // micrograms of dead leaf that had been carbon dioxide a moment earlier.
    produced.sort_by_key(|&(material, _)| world.materials().slot(material).is_none());
    host.mass += host_output - host_intake;
    let net = host_output - host_intake;
    if net > 0.0 {
        world.life.grown_g += net;
    } else {
        world.life.respired_g -= net;
    }
    for (material, grams) in &produced {
        given += *grams * world.materials().get(*material).specific_energy(out_t);
    }
    // The host now holds what it started with, plus everything gathered,
    // minus everything it is about to hand out. Its temperature follows.
    host.solve_temperature(world.materials(), energy_before - given);
    world.set_cell_at(p, host);

    let host_t = out_t;
    for (material, grams) in produced {
        let mut left = grams;
        // Cells already made of the stuff first, and only then empty ones, so
        // what a process puts out builds a heap rather than a scatter of
        // specks — a shed leaf joins the litter beside it if there is any.
        for existing in [true, false] {
            for &q in &neighbours {
                if left <= 0.0 {
                    break;
                }
                let holds = !world.is_gas_at(q) && world.cell_at(q).material == material;
                if holds != existing {
                    continue;
                }
                left -= give(world, q, material, left, host_t);
            }
        }
        if left > 1e-12 {
            // Nowhere to put it — hand it back to the host rather than lose
            // it. Only reachable if every neighbour turned solid between the
            // cap above and here, which the caps make very unlikely; the
            // alternative is silently destroying mass, which is never the
            // right answer in this crate.
            let mut back = world.cell_at(p);
            let energy = back.energy(world.materials())
                + left * world.materials().get(material).specific_energy(host_t);
            if back.material == material {
                back.mass += left;
            } else if let (true, Some(s)) = (
                back.is_gas(world.materials()),
                world.materials().slot(material),
            ) {
                back.mix[s] += left;
            } else {
                back.mass += left;
            }
            back.solve_temperature(world.materials(), energy);
            world.set_cell_at(p, back);
        }
    }
    true
}

/// How many grams of `material` could be taken out of the cell at `q`.
///
/// A gas species comes out of a gas cell's mixture; a liquid comes out of a
/// cell of that liquid, or out of the mist suspended in a gas cell — which
/// is how a plant in humid air can drink without standing in a puddle. A
/// donor is never emptied completely: a cell stripped to nothing has no
/// temperature worth speaking of and the rules downstream of it all assume
/// there is something there.
fn takeable(world: &World, q: usize, material: MaterialId) -> Scalar {
    // Never out of a cell part-way through a phase change: its energy is
    // `mass·(c·T + L + progress)`, and a parcel taken out of it at plain
    // `c·T + L` leaves the progress behind on less mass than paid for it.
    if !world.is_gas_at(q) && world.cell_at(q).pending != NO_PENDING {
        return 0.0;
    }
    let held = world.grams_of_at(q, material);
    if held <= 0.0 {
        return 0.0;
    }
    // Half of it, so nothing is stripped bare in one go — except a crumb,
    // which may go entirely. Without that exception a heap of litter being
    // eaten by mould halves for ever and never disappears, and what is left
    // behind is a hundredth of a cell of dead matter that still blocks the
    // way and still shades what is under it. A cell emptied this way costs
    // nothing to empty: everything it was holding leaves with the mass, at
    // the temperature it was already at.
    if !world.is_gas_at(q) && world.cell_at(q).material == material {
        let density = world.materials().get(material).density;
        if held <= CRUMB * density {
            return held;
        }
    }
    (held * 0.5).max(0.0)
}

/// How many grams of `material` — something that is not a gas — the cell at
/// `q` could take: the room left in a cell already made of it, or a whole
/// cell's worth of an empty one that has somewhere to push its air.
fn placeable(world: &World, q: usize, material: MaterialId) -> Scalar {
    let density = world.materials().get(material).density;
    if density <= 0.0 {
        return 0.0;
    }
    let cell = world.cell_at(q);
    if !world.is_gas_at(q) {
        if cell.material == material && cell.pending == NO_PENDING {
            return (density - cell.mass).max(0.0);
        }
        return 0.0;
    }
    if crate::gas::displaceable(world, q) {
        density
    } else {
        0.0
    }
}

/// Takes up to `grams` of `material` out of the cell at `q`, returning what
/// was taken and the joules that came with it.
fn take(world: &mut World, q: usize, material: MaterialId, grams: Scalar) -> (Scalar, f64) {
    let taken = takeable(world, q, material).min(grams);
    if taken <= 0.0 {
        return (0.0, 0.0);
    }
    let mut cell = world.cell_at(q);
    let joules = taken
        * world
            .materials()
            .get(material)
            .specific_energy(cell.temperature);
    if let (true, Some(s)) = (
        cell.is_gas(world.materials()),
        world.materials().slot(material),
    ) {
        cell.mix[s] -= taken;
        cell.refresh(world.materials());
    } else {
        cell.mass -= taken;
        if cell.mass <= 0.0 {
            // Eaten out of existence. A cell with no mass holds no energy
            // either, so there is nothing to place and nothing to book: it
            // becomes an empty gas cell and the room's own air flows into it,
            // exactly as `World::conjure_mass` already does when a gnome eats
            // the last of something.
            cell.material = world.materials().vacuum_label();
            cell.mass = 0.0;
            cell.progress = 0.0;
            cell.pending = NO_PENDING;
            cell.flow_dir = 0;
            cell.mix = NO_MIX;
        }
    }
    world.set_cell_at(q, cell);
    (taken, joules)
}

/// Puts up to `grams` of `material` into the cell at `q`, at `temperature`,
/// and returns how much it accepted.
///
/// Three ways that can happen: a gas (or a mist) joins a gas cell's mixture;
/// anything joins a cell already made of it that has room; and something
/// solid may be put into an *empty* cell, pushing the air in the way into a
/// neighbour rather than overwriting it. That third case is what lets a
/// living process put out something that is not a gas at all — a bush shedding
/// a dead leaf, a rot growing mould beside it — which nothing in this file
/// could express before.
fn give(
    world: &mut World,
    q: usize,
    material: MaterialId,
    grams: Scalar,
    temperature: Scalar,
) -> Scalar {
    let is_gas_cell = world.is_gas_at(q);
    let density = world.materials().get(material).density;
    let mixes = world.materials().slot(material).is_some();
    if is_gas_cell && !mixes {
        let room = grams.min(density);
        if room <= 0.0 || !crate::gas::displace(world, q) {
            return 0.0;
        }
        world.set_cell_at(
            q,
            Cell {
                material,
                mass: room,
                temperature,
                progress: 0.0,
                pending: NO_PENDING,
                flow_dir: 0,
                mix: NO_MIX,
            },
        );
        return room;
    }
    let mut cell = world.cell_at(q);
    let table = world.materials();
    let accepted = if is_gas_cell {
        grams
    } else if cell.material == material && cell.pending == NO_PENDING {
        grams.min((density - cell.mass).max(0.0))
    } else {
        0.0
    };
    if accepted <= 0.0 {
        return 0.0;
    }
    let energy = cell.energy(table) + accepted * table.get(material).specific_energy(temperature);
    if let (true, Some(s)) = (cell.is_gas(table), table.slot(material)) {
        cell.mix[s] += accepted;
        cell.refresh(table);
    } else {
        cell.mass += accepted;
    }
    cell.solve_temperature(table, energy);
    world.set_cell_at(q, cell);
    accepted
}

/// A full plant cell seeds a neighbouring cell of air by splitting its own
/// mass in two.
///
/// This is growth *in space* rather than in mass, and it is the only way a
/// bush ever becomes more than one cell. It creates nothing: half the host's
/// mass and half its energy move into the new cell, and the air that was
/// there is pushed into a neighbouring gas cell (`gas::move_species`, which
/// is the same primitive rain already uses to get out of the way).
///
/// A seed only goes where a plant could actually hold on — a cell with
/// something solid or granular under it, or another plant — so a bush
/// creeps along the ground and up a wall rather than growing into thin air.
pub fn spread(world: &mut World, metabolisms: &[Metabolism]) {
    let growers: Vec<&Metabolism> = metabolisms.iter().filter(|m| m.host_gain() > 0.0).collect();
    if growers.is_empty() {
        return;
    }
    let n = world.width() * world.height();
    for p in 0..n {
        let host = world.cell_at(p);
        let density = world.materials().get(host.material).density;
        if host.mass < SPREAD_AT * density || host.pending != NO_PENDING {
            continue;
        }
        // Seeding is growth in space, so it takes the same conditions
        // growing does: a process on this host that is actually running in
        // this cell's light. It used to be "any light at all", which was
        // right for the only thing that grew at the time and wrong the
        // moment something grew in the dark — a mould in a compost heap
        // would have had to wait for sunrise to spread.
        if !growers
            .iter()
            .any(|m| m.host == host.material && m.lit(world.light_at(p)))
        {
            continue;
        }
        let Some(q) = seed_target(world, p) else {
            continue;
        };
        // Get the air out of the way. If there is nowhere for it to go, the
        // seed waits.
        if !crate::gas::displace(world, q) {
            continue;
        }

        let mut parent = world.cell_at(p);
        let half = parent.mass * 0.5;
        parent.mass -= half;
        let seed = Cell {
            material: parent.material,
            mass: half,
            temperature: parent.temperature,
            progress: 0.0,
            pending: NO_PENDING,
            flow_dir: 0,
            mix: NO_MIX,
        };
        world.set_cell_at(p, parent);
        world.set_cell_at(q, seed);
    }
}

/// Where a full plant at `p` may put a seed: an adjacent gas cell that has
/// something to stand on.
fn seed_target(world: &World, p: usize) -> Option<usize> {
    let here = GridIndex::new((p % world.width()) as i32, (p / world.width()) as i32);
    for d in [
        GridIndex::new(0, 1),
        GridIndex::new(1, 0),
        GridIndex::new(-1, 0),
    ] {
        let n = GridIndex::new(here.i + d.i, here.j + d.j);
        if !world.in_bounds(n) {
            continue;
        }
        let q = world.linear_index(n);
        if !world.is_gas_at(q) {
            continue;
        }
        let under = GridIndex::new(n.i, n.j - 1);
        if !world.in_bounds(under) {
            continue;
        }
        let u = world.linear_index(under);
        let below = world.materials().get(world.material_cells()[u]);
        let supported = below.phase == Phase::Solid || below.phase == Phase::Granular;
        if supported && q != p {
            return Some(q);
        }
    }
    None
}

/// The four orthogonal neighbours of flat position `p`.
fn neighbours(world: &World, p: usize) -> impl Iterator<Item = usize> + '_ {
    let w = world.width() as i32;
    let here = GridIndex::new((p as i32) % w, (p as i32) / w);
    [
        GridIndex::new(here.i - 1, here.j),
        GridIndex::new(here.i + 1, here.j),
        GridIndex::new(here.i, here.j - 1),
        GridIndex::new(here.i, here.j + 1),
    ]
    .into_iter()
    .filter(move |&n| world.in_bounds(n))
    .map(move |n| world.linear_index(n))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::{terrarium as t, MaterialTable};

    /// A lit box with a floor, a bush on it, and as much CO₂ in the air as
    /// asked for.
    fn greenhouse(co2_g: Scalar) -> World {
        let mut w = World::new_open(7, 7, MaterialTable::terrarium(), 295.0);
        for i in 0..7 {
            w.fill(GridIndex::new(i, 0), t::STONE, 295.0);
            w.fill(GridIndex::new(i, 1), t::WATER, 295.0);
        }
        w.fill(GridIndex::new(3, 2), t::JUNIPER, 295.0);
        // Charge the air with CO2, evenly.
        for j in 2..7 {
            for i in 0..7 {
                let p = w.linear_index(GridIndex::new(i, j));
                if !w.is_gas_at(p) {
                    continue;
                }
                let mut cell = w.cell_at(p);
                let s = w.materials().slot(t::CO2).unwrap();
                cell.mix[s] += co2_g;
                w.set_cell_at(p, cell);
            }
        }
        w.sky = 1.0;
        crate::light::illuminate(&mut w);
        w.rebaseline();
        w
    }

    fn photosynthesis(w: &World) -> Metabolism {
        w.materials()
            .metabolisms()
            .iter()
            .find(|m| m.name == "photosynthesis")
            .expect("the table should carry photosynthesis")
            .clone()
    }

    /// A box with a floor, one cell of dead matter on it, and air.
    fn compost(litter_g: Scalar) -> World {
        let mut w = World::new_open(7, 7, MaterialTable::terrarium(), 295.0);
        for i in 0..7 {
            w.fill(GridIndex::new(i, 0), t::STONE, 295.0);
        }
        let p = w.linear_index(GridIndex::new(3, 1));
        let mut cell = w.cell_at(p);
        cell.material = t::LITTER;
        cell.mass = litter_g;
        cell.temperature = 295.0;
        cell.mix = NO_MIX;
        w.set_cell_at(p, cell);
        w.sky = 1.0;
        crate::light::illuminate(&mut w);
        w.rebaseline();
        w
    }

    #[test]
    fn dead_matter_rots_into_mould_and_gas() {
        let mut w = compost(0.05);
        let before = w.mass_of(t::LITTER);
        for _ in 0..400 {
            crate::light::illuminate(&mut w);
            step(&mut w, 0.05);
        }
        let after = w.mass_of(t::LITTER);
        assert!(
            after < before - 1e-9,
            "the litter did not rot: {before} -> {after}\n{}",
            crate::report::ascii_map(&w)
        );
        assert!(
            w.mass_of(t::FUNGUS) > 0.0,
            "nothing grew on it:\n{}",
            crate::report::ascii_map(&w)
        );
        assert!(w.mass_of(t::CO2) > 0.0, "and nothing came back as gas");
    }

    /// Rot moves carbon between three materials with three different carbon
    /// fractions, so "mass is conserved" says nothing about whether the
    /// carbon is. This is the check that it is: dead matter, mould and
    /// carbon dioxide, added up, are a constant.
    #[test]
    fn rot_conserves_carbon_as_well_as_mass_and_energy() {
        let mut w = compost(0.05);
        let carbon = |w: &World| {
            0.4 * (w.mass_of(t::LITTER) + w.mass_of(t::FUNGUS) + w.mass_of(t::JUNIPER))
                + (12.0 / 44.0) * w.mass_of(t::CO2)
        };
        let (c0, m0) = (carbon(&w), w.total_mass());
        for _ in 0..600 {
            crate::physics::step(&mut w, 0.05);
        }
        assert!(w.mass_of(t::CO2) > 1e-6, "nothing rotted");
        assert!(
            (carbon(&w) - c0).abs() < 1e-9 * c0,
            "carbon went {c0} -> {} g",
            carbon(&w)
        );
        assert!((w.total_mass() - m0).abs() < 1e-9 * m0);
        let r = w.conservation_residuals();
        assert!(r.energy_relative.abs() < 1e-9, "{:e}", r.energy_relative);
    }

    #[test]
    fn a_lit_bush_with_carbon_dioxide_and_water_grows() {
        let mut w = greenhouse(0.02);
        let before = w.cell(GridIndex::new(3, 2)).mass;
        for _ in 0..200 {
            crate::light::illuminate(&mut w);
            step(&mut w, 0.05);
        }
        let after = w.mass_of(t::JUNIPER);
        assert!(
            after > before + 1e-6,
            "the bush did not grow: {before} -> {after}"
        );
    }

    #[test]
    fn growth_takes_the_carbon_out_of_the_air_and_puts_oxygen_back() {
        let mut w = greenhouse(0.02);
        let co2_before = w.mass_of(t::CO2);
        let o2_before = w.mass_of(t::OXYGEN);
        for _ in 0..200 {
            crate::light::illuminate(&mut w);
            step(&mut w, 0.05);
        }
        // What photosynthesis built, not what is standing: a bush sheds as
        // well as grows, and the shed part is no longer juniper.
        let grown = w.life_tally().grown_g;
        let co2_used = co2_before - w.mass_of(t::CO2);
        let o2_made = w.mass_of(t::OXYGEN) - o2_before;
        assert!(grown > 0.0);
        // The declared proportions, to the last part in a thousand.
        assert!(
            (co2_used / grown - 264.0 / 180.0).abs() < 1e-3,
            "{co2_used} g of CO2 per {grown} g of plant"
        );
        assert!(
            (o2_made / grown - 192.0 / 180.0).abs() < 1e-3,
            "{o2_made} g of O2 per {grown} g of plant"
        );
    }

    #[test]
    fn growing_conserves_mass_and_energy_exactly() {
        let mut w = greenhouse(0.02);
        let m0 = w.total_mass();
        for _ in 0..400 {
            crate::light::illuminate(&mut w);
            step(&mut w, 0.05);
        }
        assert!(
            w.mass_of(t::JUNIPER) > 0.5,
            "nothing grew, so nothing to check"
        );
        let r = w.conservation_residuals();
        assert!(
            (w.total_mass() - m0).abs() < 1e-9 * m0,
            "mass {m0} -> {}",
            w.total_mass()
        );
        assert!(
            r.energy_relative.abs() < 1e-12,
            "energy residual {:e}",
            r.energy_relative
        );
        assert_eq!(
            w.ledger().mass_conjured,
            0.0,
            "life is not magic; it takes no ledger entry"
        );
    }

    /// Photosynthesis banks energy, so the cell doing it must cool. This is
    /// the check that the declared heat actually reaches the runtime rather
    /// than decorating a table nothing reads.
    #[test]
    fn photosynthesis_cools_the_plant_that_runs_it() {
        let mut w = greenhouse(0.05);
        let before = w.cell(GridIndex::new(3, 2)).temperature;
        for _ in 0..20 {
            crate::light::illuminate(&mut w);
            step(&mut w, 0.05);
        }
        let after = w.cell(GridIndex::new(3, 2)).temperature;
        assert!(
            after < before - 0.01,
            "banking energy should cool the bush: {before} -> {after}"
        );
    }

    /// ...and running it backwards in the dark gives exactly that energy
    /// back. The two rows share one declared figure, so this is the round
    /// trip closing, not two numbers agreeing by luck.
    #[test]
    fn a_plants_night_is_its_day_run_backwards() {
        let m = photosynthesis(&greenhouse(0.0));
        let table = MaterialTable::terrarium();
        let enthalpy = |rs: &[crate::material::Reagent]| -> Scalar {
            rs.iter()
                .map(|r| r.grams * table.get(r.material).specific_energy(291.0))
                .sum()
        };
        let day = enthalpy(&m.output) - enthalpy(&m.intake);
        let night = table
            .metabolisms()
            .iter()
            .find(|x| x.name == "plant respiration")
            .map(|x| enthalpy(&x.output) - enthalpy(&x.intake))
            .expect("the table should carry plant respiration");
        assert!(
            (day + night).abs() < 1e-6 * day.abs().max(1.0),
            "day banks {day} J/g and night releases {night} J/g — they must cancel"
        );
        assert!(
            day > 1000.0,
            "photosynthesis should bank real energy, got {day} J/g"
        );
    }

    #[test]
    fn a_bush_in_the_dark_does_not_grow() {
        let mut w = greenhouse(0.02);
        w.sky = 0.0;
        crate::light::illuminate(&mut w);
        let before = w.mass_of(t::JUNIPER);
        for _ in 0..50 {
            step(&mut w, 0.05);
        }
        assert!(
            w.mass_of(t::JUNIPER) <= before,
            "a bush grew with the lights off"
        );
    }

    /// In the dark a plant runs its own respiration instead: it loses mass,
    /// and the carbon it loses turns up as CO₂.
    #[test]
    fn a_bush_in_the_dark_respires_its_own_mass_away() {
        let mut w = greenhouse(0.0);
        w.sky = 0.0;
        crate::light::illuminate(&mut w);
        let before = w.mass_of(t::JUNIPER);
        let co2_before = w.mass_of(t::CO2);
        for _ in 0..300 {
            step(&mut w, 0.05);
        }
        assert!(
            w.mass_of(t::JUNIPER) < before - 1e-9,
            "a bush at night should be spending itself: {before} -> {}",
            w.mass_of(t::JUNIPER)
        );
        assert!(
            w.mass_of(t::CO2) > co2_before,
            "and breathing out carbon dioxide"
        );
        let r = w.conservation_residuals();
        assert!(r.energy_relative.abs() < 1e-9, "{:e}", r.energy_relative);
    }

    #[test]
    fn a_bush_with_no_carbon_dioxide_at_all_cannot_grow() {
        let mut w = greenhouse(0.0);
        let before = w.mass_of(t::JUNIPER);
        for _ in 0..200 {
            crate::light::illuminate(&mut w);
            step(&mut w, 0.05);
        }
        // It may shed — a bush does that whatever the air is like — but it
        // may not put on any.
        assert!(
            w.mass_of(t::JUNIPER) <= before + 1e-12,
            "a bush grew out of nothing: {before} -> {}",
            w.mass_of(t::JUNIPER)
        );
        assert_eq!(
            w.life_tally().grown_g,
            0.0,
            "nothing should have been built"
        );
    }

    /// A full bush seeds the cell beside it, and the pair together mass
    /// exactly what the one did — spreading is a split, not a birth.
    #[test]
    fn a_full_bush_seeds_its_neighbour_without_creating_any_plant() {
        let mut w = World::new_open(7, 7, MaterialTable::terrarium(), 295.0);
        for i in 0..7 {
            w.fill(GridIndex::new(i, 0), t::STONE, 295.0);
        }
        w.fill(GridIndex::new(3, 1), t::JUNIPER, 295.0);
        w.sky = 1.0;
        crate::light::illuminate(&mut w);
        w.rebaseline();
        let before = w.mass_of(t::JUNIPER);
        let (m0, e0) = (w.total_mass(), w.total_energy());
        let metabolisms = w.materials().metabolisms().to_vec();
        spread(&mut w, &metabolisms);
        assert!(
            w.count_of(t::JUNIPER) == 2,
            "a full bush should have seeded:\n{}",
            crate::report::ascii_map(&w)
        );
        assert!(
            (w.mass_of(t::JUNIPER) - before).abs() < 1e-12,
            "spreading made plant out of nothing: {before} -> {}",
            w.mass_of(t::JUNIPER)
        );
        assert!((w.total_mass() - m0).abs() < 1e-9);
        assert!((w.total_energy() - e0).abs() < 1e-6 * e0.abs());
    }

    /// A plant grows on its own stem or along the ground, never sideways
    /// into open air — which is the difference between a bush and a slick
    /// spreading across the sky.
    #[test]
    fn a_bush_grows_on_its_own_stem_rather_than_sideways_into_the_void() {
        let mut w = World::new_open(7, 9, MaterialTable::terrarium(), 295.0);
        for i in 0..7 {
            w.fill(GridIndex::new(i, 0), t::STONE, 295.0);
        }
        // A bush floating two rows up, nothing beneath the cells beside it.
        w.fill(GridIndex::new(3, 5), t::JUNIPER, 295.0);
        w.sky = 1.0;
        crate::light::illuminate(&mut w);
        w.rebaseline();
        let metabolisms = w.materials().metabolisms().to_vec();
        spread(&mut w, &metabolisms);
        assert_eq!(
            w.material_at(GridIndex::new(4, 5)),
            t::AIR,
            "it went sideways"
        );
        assert_eq!(
            w.material_at(GridIndex::new(2, 5)),
            t::AIR,
            "it went sideways"
        );
        assert_eq!(
            w.material_at(GridIndex::new(3, 6)),
            t::JUNIPER,
            "a stem should carry a bush upward:\n{}",
            crate::report::ascii_map(&w)
        );
    }
}
