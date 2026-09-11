//! Vapour: evaporation below boiling, condensation into mist, and rain —
//! the water cycle done by vapour pressure instead of by a kettle.
//!
//! Up to now the only way water became vapour here was to boil: a cell had
//! to reach 373.15 K and pay its full latent heat, and then *all* of it
//! turned to steam. So the terrarium's water cycle needed a 950 K vent and
//! ran its whole jar at around 400 K, and "humid air" did not exist — air
//! either held no water at all or was a cell of pure steam. A pond at room
//! temperature never evaporated, and nothing ever fogged.
//!
//! What was missing is the one curve every real water cycle runs on: the
//! most vapour air can hold at a given temperature. Once gas cells can hold
//! mixtures (`src/gas.rs`) the curve has somewhere to live, and the
//! material table already knew everything needed to draw it — the boiling
//! point and the latent heat are on the transition, the vapour's gas
//! constant on the gas. See [`MaterialTable::saturation_pressure`].
//!
//! Three rules, each conserving by construction:
//!
//! - [`exchange_at_surfaces`] — a liquid surface gives vapour to a touching
//!   gas cell until that cell's partial pressure reaches saturation, and
//!   vapour past saturation gives itself up onto whatever surface it
//!   touches. The latent heat is paid into or out of the pair, so a pond
//!   cools as it evaporates and a cold wall warms as dew forms on it.
//! - [`condense`] — vapour in excess of saturation in open air becomes
//!   **mist**, liquid
//!   droplets carried inside the gas cell's mixture, and gives up its latent
//!   heat to the cell; mist in air that has warmed past saturation
//!   evaporates back. A cloud is a region of cells holding mist.
//! - [`precipitate`] — mist touching its own liquid joins it, and a cell
//!   holding enough mist rains out as a droplet: its gas is pushed into a
//!   neighbour and what is left is a small cell of liquid, which the
//!   ordinary gravity rule drops and `physics::coalesce_loose` pools.
//!
//! Boiling survives only where it belongs. A liquid cell *touching gas*
//! never boils by threshold any more — it evaporates, here, and that is the
//! only place the pressure it is evaporating against is known (see
//! `physics::apply_phase_changes` for the second-law violation that
//! motivated it). A liquid cell with no gas to evaporate into still
//! accumulates latent heat past its boiling point and turns wholesale to
//! vapour: a bubble, which rises and bursts, and which these rules leave
//! alone until it has (see [`expanding`]).
//!
//! [`MaterialTable::saturation_pressure`]: crate::material::MaterialTable::saturation_pressure

use crate::material::{Mobility, Phase, Volatile};
use crate::math::{GridIndex, Scalar};
use crate::world::{Cell, World, NO_MIX, NO_PENDING};

/// Fraction of a gas cell's shortfall from saturation that a touching
/// liquid surface makes up per second. Tuned, like conductivity: real
/// evaporation is limited by diffusion through a thin boundary layer this
/// grid does not resolve. What matters for the water cycle is only that it
/// is fast next to the rate humid air is carried away, which it is.
const EVAPORATION_PER_SECOND: Scalar = 2.0;

/// Fraction of the way to saturation a gas cell's vapour and mist close per
/// second. Fast, because it is not really the limit: condensing releases
/// latent heat into a gas cell whose heat capacity is tiny, which is what
/// stops it (see [`equilibrium_gap`]). The true rate of condensation is how
/// fast the cold lid can conduct that heat away — which is exactly how real
/// dew forms.
const CONDENSATION_PER_SECOND: Scalar = 10.0;

/// Grams of mist a gas cell holds before it rains out as a droplet — one
/// percent of a full cell of water.
///
/// Mist is never lost below this; it only stays mist, drifting with the air
/// and falling slowly because a misty cell is genuinely heavier. The
/// threshold only decides when droplets are big enough to be drawn and
/// pooled as liquid.
pub const DROPLET_G: Scalar = 0.01;

/// Grams of mist a gas cell touching a surface — anything that is not gas —
/// gathers before it condenses onto it as a drop. A tenth of a free-air
/// droplet: see [`precipitate`].
pub const DEW_G: Scalar = 0.001;

/// Above this many atmospheres a small pocket of gas is a bubble whose
/// vapour is left alone — see [`expanding`].
const TRANSIENT_ATMOSPHERES: Scalar = 3.0;

/// At most this many of the twelve cells within two steps of a gas cell may
/// be gas for it to count as a bubble — see [`expanding`].
const BUBBLE_NEIGHBOURS: usize = 4;

/// How far, in cells, an over-pressured gas cell looks for the lower
/// pressure that would make it a burst still spreading out — see
/// [`expanding`].
const BURST_REACH: i32 = 3;

/// How much above the lowest pressure within [`BURST_REACH`] a gas cell must
/// be to count as a burst still spreading out. A settled vessel is uniform
/// to within a few per cent; the cloud a boiling bubble leaves behind as it
/// breaks the surface is not, and twice was too lenient to catch its edges.
const BURST_RATIO: Scalar = 1.2;

/// A running tally of what the water cycle has done — a diary, not a
/// ledger: nothing here is a hole in the books, it is just what moved.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Tally {
    /// Grams evaporated from liquid surfaces into the air.
    pub evaporated_g: f64,
    /// Net grams condensed out of the air as mist.
    pub condensed_g: f64,
    /// Grams of mist that fell out of the air as liquid — into a pool, or
    /// as a droplet.
    pub rained_g: f64,
}

/// The vapour rules, in the order `physics::step` runs them.
pub fn step(world: &mut World, dt: Scalar) {
    exchange_at_surfaces(world, dt);
    condense(world, dt);
    precipitate(world);
}

fn is_flowing_gas(world: &World, p: usize) -> bool {
    world.is_gas_at(p) && world.material_of(p).mobility != Mobility::Static
}

fn neighbours(world: &World, p: usize) -> impl Iterator<Item = usize> + '_ {
    let w = world.width() as i32;
    let (i, j) = ((p as i32) % w, (p as i32) / w);
    // Downward first, then sideways, then up — the order a drop looks for
    // somewhere to land.
    [(0, -1), (-1, 0), (1, 0), (0, 1)]
        .into_iter()
        .map(move |(di, dj)| GridIndex::new(i + di, j + dj))
        .filter(|&n| world.in_bounds(n))
        .map(|n| world.linear_index(n))
}

/// Grams of `volatile`'s vapour one cell of gas can hold at `temperature`
/// — the saturation pressure turned into a mass, since a cell's volume is
/// one.
fn saturation_grams(world: &World, volatile: &Volatile, temperature: Scalar) -> Scalar {
    let r_v = world.materials().get(volatile.vapour).gas_constant;
    if r_v <= 0.0 || temperature <= 0.0 {
        return 0.0;
    }
    world.materials().saturation_pressure(volatile, temperature) / (r_v * temperature)
}

/// How far a body's vapour is from saturation, in grams, *after* paying for
/// the move itself — the amount that, evaporated (positive) or condensed
/// (negative), leaves it exactly saturated at the temperature that
/// evaporating or condensing it produces.
///
/// The naive gap, `saturation_grams(T) − vapour`, is the wrong thing to
/// relax toward, and not by a little. Condensing a gram of spirit releases
/// 846 J, and a gas cell's heat capacity is a few thousandths of a joule
/// per kelvin, so condensing even a fraction of the naive excess heats the
/// cell so far that it is suddenly well *under* saturation; the next step
/// evaporates the mist back, cools it past the curve the other way, and the
/// swing grows. The first still built on vapour pressure had its head space
/// oscillating above 410 K beside a 368 K hob, with energy conserved to
/// fourteen digits the whole time. This is the same lesson
/// `physics::conduct_heat` learned long ago — clamp to the equilibrium,
/// never step past it — with the equilibrium moved by the step's own heat.
///
/// Linearised about the current state: the curve's slope is
/// `d(ln x_sat)/dT = L/(R·T²) − 1/T`, the step's own temperature change is
/// `h_fg / C` per gram, and the equilibrium is the naive gap divided by
/// `1 + slope·h_fg/C`. Linear is close enough, because what is applied is a
/// fraction of this and it is re-measured every step.
///
/// `volume` is how many cells the gas will occupy once it has expanded to
/// the pressure around it — see [`expanded_volume`]. Saturation is a limit
/// per unit volume, so the most vapour a parcel can hold scales with it.
fn equilibrium_gap(
    world: &World,
    volatile: &Volatile,
    vapour: Scalar,
    temperature: Scalar,
    capacity: f64,
    volume: Scalar,
) -> Scalar {
    let table = world.materials();
    let x_sat = saturation_grams(world, volatile, temperature) * volume;
    let gap = x_sat - vapour;
    if capacity <= 0.0 || temperature <= 0.0 {
        return gap;
    }
    let r_v = table.get(volatile.vapour).gas_constant;
    let slope = x_sat
        * (volatile.latent_heat / (r_v * temperature * temperature) - 1.0 / temperature).max(0.0);
    // Specific enthalpy of vaporisation at this temperature, from the
    // table's own offsets — the number the energy solve will actually use.
    let h_fg = (table.get(volatile.vapour).specific_energy(temperature)
        - table.get(volatile.liquid).specific_energy(temperature))
    .max(0.0);
    gap / (1.0 + slope * h_fg / capacity as Scalar)
}

/// How many cells' worth of room the gas in cell `p` will fill once it has
/// expanded to the pressure around it: its own pressure over the lowest of
/// its gas neighbours', and never less than one. Saturation is judged
/// against the volume a parcel is about to have, not the one cell the grid
/// has squeezed it into for the moment.
fn expanded_volume(world: &World, p: usize) -> Scalar {
    let own = crate::gas::pressure_at(world, p);
    if own <= 0.0 {
        return 1.0;
    }
    let ambient = neighbours(world, p)
        .filter(|&q| world.is_gas_at(q))
        .map(|q| crate::gas::pressure_at(world, q))
        .fold(Scalar::INFINITY, Scalar::min);
    if !ambient.is_finite() || ambient <= 0.0 {
        return 1.0;
    }
    (own / ambient).max(1.0)
}

/// Whether the gas cell at `p` is still expanding from something that just
/// boiled into it — above [`TRANSIENT_ATMOSPHERES`] — and so not yet in a
/// state whose saturation means anything.
///
/// A liquid that boils in this grid becomes one cell holding a gram of
/// vapour at several hundred atmospheres, because one cell is all the room
/// it has until flow lets it out; a real bubble is at the pressure around
/// it and hundreds of times the size. Asked "is this saturated?" as it
/// stands, the squeezed cell is, enormously, and the vapour rules used to
/// act on that: bubbles rising through a still's pot collapsed on the way
/// up, onto the water around them or into drops inside themselves, and the
/// gin they made floated up and pooled in the pot it had just boiled out
/// of. Judging against the volume the parcel is about to have fixed a lone
/// bubble but not two side by side — mashing makes wash in pairs, so they
/// boil in pairs — each of which saw the other as its surroundings.
///
/// So gas well over one atmosphere is left alone while it is visibly still
/// on its way out: either a *bubble* — a small pocket with hardly any other
/// gas around it — or a *burst*, well above the lowest pressure a few cells
/// away. What it is not is simply "high pressure": the first version of
/// this rule went by pressure alone, and the terrarium, whose vent boils
/// enough of its pool to fill the sealed jar with tens of atmospheres of
/// steam, stopped raining entirely, because every cell of a genuine
/// pressure vessel looked like a bubble. A vessel is uniformly high; a
/// bubble or a burst is high against its surroundings.
fn expanding(world: &World, p: usize) -> bool {
    let own = crate::gas::pressure_at(world, p);
    if own <= TRANSIENT_ATMOSPHERES * world.materials().reference_pressure() {
        return false;
    }
    let (w, h) = (world.width() as i32, world.height() as i32);
    let (i, j) = ((p as i32) % w, (p as i32) / w);
    let (mut gas_near, mut lowest) = (0usize, Scalar::INFINITY);
    for dj in -BURST_REACH..=BURST_REACH {
        for di in -BURST_REACH..=BURST_REACH {
            let d = di.abs() + dj.abs();
            if d == 0 || d > BURST_REACH {
                continue;
            }
            let (x, y) = (i + di, j + dj);
            if x < 0 || y < 0 || x >= w || y >= h {
                continue;
            }
            let q = (y * w + x) as usize;
            if !world.is_gas_at(q) {
                continue;
            }
            if d <= 2 {
                gas_near += 1;
            }
            lowest = lowest.min(crate::gas::pressure_at(world, q));
        }
    }
    gas_near <= BUBBLE_NEIGHBOURS || own > BURST_RATIO * lowest
}

/// Evaporation and condensation at every surface: each pair of a gas cell
/// and a liquid or solid touching it trades vapour toward saturation at the
/// pair's shared temperature.
///
/// - **Evaporation.** A liquid gives vapour to the gas until the gas is
///   saturated. The vapour carries more enthalpy than the liquid it came
///   from, so the pair comes out cooler — evaporative cooling, emergent
///   rather than applied.
/// - **Condensation.** Vapour above saturation at the surface's temperature
///   gives itself up *onto the surface*: into it, if the surface is the
///   very liquid the vapour condenses into and has room; otherwise as dew,
///   mist in the gas cell against the surface, which [`precipitate`] turns
///   into drops. The latent heat released goes into the pair — which, next
///   to a cold wall with hundreds of times the gas cell's heat capacity,
///   means into the wall.
///
/// That last point is the whole reason this is pairwise. Condensing only
/// *inside* a gas cell dumps the latent heat into the one thing in the
/// world least able to hold it, which then has to hand it on to the wall
/// through gas, which conducts poorly; the first still built on vapour
/// pressure had a condenser that saturated at 320 K and backed the whole
/// still up to two atmospheres. A real condenser works because vapour
/// condenses on the cold surface and the surface takes the heat.
///
/// Each exchange is treated like a reaction in `src/chemistry.rs`: the
/// pair's energy is totalled before, the mass moves, and the pair's shared
/// temperature afterwards is solved from that same total. Cells with a phase
/// change in flight are left alone.
pub fn exchange_at_surfaces(world: &mut World, dt: Scalar) {
    let rate = (EVAPORATION_PER_SECOND * dt).clamp(0.0, 1.0);
    let table = world.materials();
    let vapour_slots: Vec<usize> = (0..table.slots().len())
        .filter(|&s| table.slot_props().is_gas[s])
        .filter(|&s| {
            let id = table.slots()[s];
            table.condensation_of(id).is_some() || table.evaporation_of_any_into(id)
        })
        .collect();
    if vapour_slots.is_empty() {
        return;
    }
    let n = world.width() * world.height();
    for p in 0..n {
        if world.is_gas_at(p) || world.cell_at(p).pending != NO_PENDING {
            continue;
        }
        let neighbours: Vec<usize> = neighbours(world, p).collect();
        for q in neighbours {
            if !is_flowing_gas(world, q) {
                continue;
            }
            for &vs in &vapour_slots {
                // Re-read every time round: an earlier exchange may have
                // emptied the surface, or started it boiling.
                if world.is_gas_at(p) || world.cell_at(p).pending != NO_PENDING {
                    break;
                }
                exchange_one(world, p, q, vs, rate);
            }
        }
    }
}

/// One surface `p`, one gas cell `q`, one vapour (mixture slot `vs`): see
/// [`exchange_at_surfaces`].
fn exchange_one(world: &mut World, p: usize, q: usize, vs: usize, rate: Scalar) {
    if expanding(world, q) {
        return;
    }
    let table = world.materials();
    let vapour_id = table.slots()[vs];
    let (surface, gas) = (world.cell_at(p), world.cell_at(q));
    let surface_material = table.get(surface.material);
    // Can this surface supply this vapour, and what does the vapour become?
    let supplies = (surface_material.phase == Phase::Liquid
        && surface_material.mobility != Mobility::Static)
        .then(|| table.evaporation_of(surface.material))
        .flatten()
        .filter(|v| v.vapour == vapour_id)
        .copied();
    let condenses = table.condensation_of(vapour_id).copied();
    let Some(curve) = supplies.or(condenses) else {
        return;
    };
    if supplies.is_none() && gas.mix[vs] <= 0.0 {
        return;
    }
    let (cp, cq) = (surface.capacity(table), gas.capacity(table));
    if cp + cq <= 0.0 {
        return;
    }
    let shared = ((cp * surface.temperature + cq * gas.temperature) / (cp + cq)) as Scalar;
    let gap = equilibrium_gap(
        world,
        &curve,
        gas.mix[vs],
        shared,
        cp + cq,
        expanded_volume(world, q),
    );
    let (mut surface, mut gas) = (surface, gas);
    let energy = surface.energy(table) + gas.energy(table);
    let mut tally = Tally::default();
    if gap > 0.0 {
        if supplies.is_none() || surface.mass <= 0.0 {
            return;
        }
        let grams = (gap * rate).min(surface.mass);
        if grams <= 1e-15 {
            return;
        }
        surface.mass -= grams;
        gas.mix[vs] += grams;
        tally.evaporated_g += grams;
    } else if gap < 0.0 {
        let Some(condensate) = condenses else {
            return;
        };
        let mut grams = (-gap * rate).min(gas.mix[vs]);
        let room = table.get(condensate.liquid).density - surface.mass;
        if surface.material == condensate.liquid && room > 0.0 {
            // Straight into the pool it is touching.
            grams = grams.min(room);
            surface.mass += grams;
        } else if let Some(mist) = table.slot(condensate.liquid) {
            // Dew: droplets against the surface, still in the gas cell.
            gas.mix[mist] += grams;
        } else {
            return;
        }
        if grams <= 1e-15 {
            return;
        }
        gas.mix[vs] -= grams;
        tally.condensed_g += grams;
    } else {
        return;
    }
    gas.refresh(table);
    if surface.mass <= 0.0 {
        // Evaporated away entirely: nothing left to hold energy, so it
        // becomes an empty gas cell and the gas takes the pair's energy.
        let emptied = Cell {
            material: table.vacuum_label(),
            mass: 0.0,
            temperature: surface.temperature,
            progress: 0.0,
            pending: NO_PENDING,
            flow_dir: 0,
            mix: NO_MIX,
        };
        gas.solve_temperature(table, energy);
        world.set_cell_at(p, emptied);
        world.set_cell_at(q, gas);
    } else {
        let capacity = surface.capacity(table) + gas.capacity(table);
        if capacity > 0.0 {
            let t = ((energy - surface.stored(table) - gas.stored(table)) / capacity) as Scalar;
            surface.temperature = t;
            gas.temperature = t;
        }
        world.set_cell_at(p, surface);
        world.set_cell_at(q, gas);
    }
    world.tally.evaporated_g += tally.evaporated_g;
    world.tally.condensed_g += tally.condensed_g;
}

/// Condensation and re-evaporation inside each gas cell: every vapour that
/// has a liquid to condense into relaxes toward saturation at the cell's
/// own temperature, trading mass with that liquid's mist slot.
///
/// The cell's energy is held fixed across the trade and its temperature
/// solved from it, so condensing warms the cell by exactly the latent heat
/// released and evaporating mist cools it by exactly what that costs.
pub fn condense(world: &mut World, dt: Scalar) {
    let rate = (CONDENSATION_PER_SECOND * dt).clamp(0.0, 1.0);
    let pairs: Vec<(usize, usize, Volatile)> = world
        .materials()
        .slots()
        .iter()
        .filter_map(|&vapour| {
            let table = world.materials();
            let volatile = *table.condensation_of(vapour)?;
            Some((table.slot(vapour)?, table.slot(volatile.liquid)?, volatile))
        })
        .collect();
    if pairs.is_empty() {
        return;
    }
    let n = world.width() * world.height();
    for p in 0..n {
        if !is_flowing_gas(world, p) || expanding(world, p) {
            continue;
        }
        for &(vapour_slot, mist_slot, volatile) in &pairs {
            let mut cell = world.cell_at(p);
            // Nothing to condense and nothing to re-evaporate: the common
            // case, skipped before the saturation curve is evaluated.
            if cell.mix[vapour_slot] <= 0.0 && cell.mix[mist_slot] <= 0.0 {
                continue;
            }
            let table = world.materials();
            let gap = equilibrium_gap(
                world,
                &volatile,
                cell.mix[vapour_slot],
                cell.temperature,
                cell.capacity(table),
                expanded_volume(world, p),
            );
            // Positive: vapour condensing into mist. Negative: mist
            // evaporating back, and never more mist than there is.
            let grams = if gap < 0.0 {
                -gap * rate
            } else if cell.mix[mist_slot] > 0.0 {
                -(gap * rate).min(cell.mix[mist_slot])
            } else {
                0.0
            };
            if grams == 0.0 {
                continue;
            }
            let energy = cell.energy(table);
            cell.mix[vapour_slot] -= grams;
            cell.mix[mist_slot] += grams;
            cell.refresh(table);
            cell.solve_temperature(table, energy);
            world.set_cell_at(p, cell);
            world.tally.condensed_g += grams as f64;
        }
    }
}

/// Mist leaving the air: into its own liquid where it touches some with
/// room to take it, and as a droplet where enough has gathered in one cell.
pub fn precipitate(world: &mut World) {
    let table = world.materials();
    let mist_slots: Vec<usize> = table
        .slots()
        .iter()
        .enumerate()
        .filter(|(_, &id)| !table.is_gas(id))
        .map(|(s, _)| s)
        .collect();
    if mist_slots.is_empty() {
        return;
    }
    let n = world.width() * world.height();
    for p in 0..n {
        if !is_flowing_gas(world, p) {
            continue;
        }
        for &s in &mist_slots {
            if !world.is_gas_at(p) || world.cell_at(p).mix[s] <= 0.0 {
                continue;
            }
            settle_onto_liquid(world, p, s);
            // Against a surface, a much smaller gathering of mist is enough
            // to become a drop — that is dew, and it is how a condenser
            // actually works: vapour gives itself up on the cold wall, not
            // as fog in the middle of the room. Without it the first still
            // on vapour pressure filled its receiver with two and a half
            // grams of gin fog, spread too thin for any one cell to rain,
            // and never collected a drop.
            let threshold = if touches_surface(world, p) {
                DEW_G
            } else {
                DROPLET_G
            };
            if world.cell_at(p).mix[s] >= threshold && rain_out(world, p, s) {
                break;
            }
        }
    }
}

/// Whether gas cell `p` touches anything that is not gas — a wall, a floor,
/// a pool, a bush: somewhere for dew to form.
fn touches_surface(world: &World, p: usize) -> bool {
    neighbours(world, p).any(|q| !world.is_gas_at(q))
}

/// Pours the mist in slot `s` of gas cell `p` into a touching cell of the
/// same liquid that has room for it — rain landing on a pond, fog settling
/// onto one.
fn settle_onto_liquid(world: &mut World, p: usize, s: usize) {
    let liquid_id = world.materials().slots()[s];
    let neighbours: Vec<usize> = neighbours(world, p).collect();
    for q in neighbours {
        let dest = world.cell_at(q);
        let mist = world.cell_at(p).mix[s];
        if mist <= 0.0 {
            return;
        }
        if dest.material != liquid_id || dest.pending != NO_PENDING {
            continue;
        }
        let table = world.materials();
        let liquid = table.get(liquid_id);
        let room = liquid.density - dest.mass;
        let grams = mist.min(room);
        if grams <= 0.0 {
            continue;
        }
        let mut gas = world.cell_at(p);
        let mut dest = dest;
        let energy = dest.energy(table) + grams * liquid.specific_energy(gas.temperature);
        gas.mix[s] -= grams;
        gas.refresh(table);
        dest.mass += grams;
        dest.solve_temperature(table, energy);
        world.set_cell_at(p, gas);
        world.set_cell_at(q, dest);
        world.tally.rained_g += grams;
    }
}

/// Turns gas cell `p` into a droplet of the liquid in mist slot `s`: every
/// other thing in the cell is handed to the neighbouring gas cell at the
/// lowest pressure, and what remains — only the mist, at the cell's own
/// temperature — is exactly a small cell of that liquid, so relabelling it
/// costs nothing. Returns whether it rained.
fn rain_out(world: &mut World, p: usize, s: usize) -> bool {
    let Some(r) = neighbours(world, p)
        .filter(|&q| is_flowing_gas(world, q))
        .min_by(|&a, &b| {
            crate::gas::pressure_at(world, a)
                .partial_cmp(&crate::gas::pressure_at(world, b))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    else {
        return false;
    };
    for other in 0..crate::material::MIX_SLOTS {
        let grams = world.cell_at(p).mix[other];
        if other != s && grams > 0.0 {
            crate::gas::move_species(world, p, r, other, grams);
        }
    }
    let cell = world.cell_at(p);
    let liquid = world.materials().slots()[s];
    let drop = Cell {
        material: liquid,
        mass: cell.mix[s],
        temperature: cell.temperature,
        progress: 0.0,
        pending: NO_PENDING,
        flow_dir: 0,
        mix: NO_MIX,
    };
    world.tally.rained_g += drop.mass;
    world.set_cell_at(p, drop);
    true
}

/// The relative humidity of the gas cell at `p` for `volatile`'s vapour:
/// its partial pressure over the saturation pressure at its temperature.
pub fn relative_humidity_at(world: &World, p: usize, volatile: &Volatile) -> Scalar {
    let cell = world.cell_at(p);
    let capacity = saturation_grams(world, volatile, cell.temperature);
    if capacity <= 0.0 {
        return 0.0;
    }
    world.grams_of_at(p, volatile.vapour) / capacity
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::{terrarium as t, MaterialTable};
    use crate::physics;

    fn world(w: usize, h: usize, temperature: Scalar) -> World {
        World::new_open(w, h, MaterialTable::terrarium(), temperature)
    }

    fn water_volatile(w: &World) -> Volatile {
        *w.materials()
            .evaporation_of(t::WATER)
            .expect("water evaporates")
    }

    /// The curve reproduces its own inputs: at the boiling point the air
    /// can hold exactly one atmosphere of vapour, and at room temperature a
    /// few percent of one — close to the measured figures, from no numbers
    /// that were not already in the table.
    #[test]
    fn saturation_pressure_is_one_atmosphere_at_the_boiling_point() {
        let table = MaterialTable::terrarium();
        let water = *table.evaporation_of(t::WATER).unwrap();
        let p0 = table.reference_pressure();
        let at_boil = table.saturation_pressure(&water, 373.15);
        assert!((at_boil - p0).abs() < 1e-4 * p0, "{at_boil} vs {p0}");
        // Measured: 0.035 atm at 300 K, 0.012 atm at 283 K.
        let warm = table.saturation_pressure(&water, 300.0) / p0;
        let cold = table.saturation_pressure(&water, 283.0) / p0;
        assert!((0.03..0.05).contains(&warm), "300 K: {warm} atm");
        assert!((0.01..0.02).contains(&cold), "283 K: {cold} atm");
        // And spirit is more volatile than water at the same temperature,
        // which is the whole of why a still separates anything.
        let spirit = *table.evaporation_of(t::WASH).unwrap();
        assert!(
            table.saturation_pressure(&spirit, 340.0) > table.saturation_pressure(&water, 340.0)
        );
    }

    /// A pond evaporates without boiling, stops when the air above it is
    /// saturated, and cools while it does it.
    #[test]
    fn a_pond_evaporates_below_boiling_until_the_air_is_saturated() {
        let mut w = world(8, 8, 300.0);
        for i in 0..8 {
            w.fill(GridIndex::new(i, 0), t::WATER, 300.0);
        }
        w.rebaseline();
        let (m0, e0) = (w.total_mass(), w.total_energy());
        for _ in 0..3000 {
            physics::step(&mut w, 0.05);
        }
        let vapour = w.mass_of(t::STEAM);
        assert!(vapour > 0.0, "nothing evaporated");
        assert_eq!(w.count_of(t::STEAM), 0, "nothing should have boiled");
        let volatile = water_volatile(&w);
        let top = w.linear_index(GridIndex::new(4, 7));
        let rh = relative_humidity_at(&w, top, &volatile);
        assert!(
            (0.8..1.05).contains(&rh),
            "a sealed box over a pond should end up near saturated, got RH {rh}"
        );
        assert!(
            w.cell(GridIndex::new(4, 0)).temperature < 300.0,
            "evaporation should have cooled the pond"
        );
        assert!((w.total_mass() - m0).abs() / m0 < 1e-6, "mass moved");
        assert!(
            (w.total_energy() - e0).abs() / e0.abs() < 1e-6,
            "energy moved"
        );
    }

    /// Humid air cooled below its dew point fogs: the excess vapour becomes
    /// mist and the cell warms by the latent heat that released.
    #[test]
    fn humid_air_fogs_when_it_is_cooled_below_its_dew_point() {
        let mut w = world(1, 1, 283.0);
        let volatile = water_volatile(&w);
        // Air carrying what it could hold at 310 K, now at 283 K.
        let humid = saturation_grams(&w, &volatile, 310.0);
        let cell = Cell::gas(w.materials(), &[(t::AIR, 0.0012), (t::STEAM, humid)], 283.0);
        w.set_cell(GridIndex::new(0, 0), cell);
        w.rebaseline();
        let e0 = w.total_energy();
        for _ in 0..40 {
            condense(&mut w, 0.05);
        }
        let mist = w.grams_of_at(0, t::WATER);
        assert!(mist > 0.0, "no mist formed");
        assert!(
            w.cell_at(0).temperature > 283.0,
            "condensing should release latent heat into the cell"
        );
        assert!((w.total_energy() - e0).abs() / e0.abs() < 1e-6);
    }

    /// A cell holding a droplet's worth of mist rains it out: the drop is a
    /// small cell of water, and every gram and joule is accounted for.
    #[test]
    fn enough_mist_in_one_cell_rains_out_as_a_drop() {
        let mut w = world(3, 3, 283.0);
        let cloud = Cell::gas(
            w.materials(),
            &[(t::AIR, 0.0012), (t::WATER, DROPLET_G * 1.5)],
            283.0,
        );
        w.set_cell(GridIndex::new(1, 2), cloud);
        w.rebaseline();
        let (m0, e0) = (w.total_mass(), w.total_energy());
        precipitate(&mut w);
        assert_eq!(
            w.material_at(GridIndex::new(1, 2)),
            t::WATER,
            "no drop formed"
        );
        assert!((w.cell(GridIndex::new(1, 2)).mass - DROPLET_G * 1.5).abs() < 1e-7);
        assert!((w.total_mass() - m0).abs() / m0 < 1e-6);
        assert!((w.total_energy() - e0).abs() / e0.abs() < 1e-6);
    }

    /// The water cycle in miniature, in a box with a warm floor under a pond
    /// and a cold lid — no boiling anywhere, and still it rains.
    #[test]
    fn a_warm_pond_under_a_cold_lid_rains_without_boiling() {
        let mut w = world(12, 14, 300.0);
        for i in 0..12 {
            w.fill(GridIndex::new(i, 0), t::STONE, 330.0);
            w.fill(GridIndex::new(i, 13), t::STONE, 275.0);
            for j in 1..3 {
                w.fill(GridIndex::new(i, j), t::WATER, 320.0);
            }
        }
        w.rebaseline();
        let hob: Vec<crate::terrarium::Thermostat> = (0..12)
            .flat_map(|i| {
                [
                    crate::terrarium::Thermostat::new(GridIndex::new(i, 0), 330.0),
                    crate::terrarium::Thermostat::new(GridIndex::new(i, 13), 275.0),
                ]
            })
            .collect();
        for _ in 0..6000 {
            for th in &hob {
                th.apply(&mut w);
            }
            physics::step(&mut w, 0.05);
        }
        let tally = w.tally();
        assert!(tally.evaporated_g > 0.05, "{tally:?}");
        assert!(tally.condensed_g > 0.01, "{tally:?}");
        assert!(tally.rained_g > 0.01, "the lid never rained: {tally:?}");
        assert_eq!(w.count_of(t::STEAM), 0, "nothing should have boiled");
        let r = w.conservation_residuals();
        assert!(r.mass_relative.abs() < 1e-6, "mass {:e}", r.mass_relative);
        assert!(
            r.energy_relative.abs() < 1e-5,
            "energy {:e}",
            r.energy_relative
        );
    }
}
