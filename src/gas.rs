//! Gas as a compressible *mixture* with a real pressure — the piece
//! `NORTH_STARS.md` #4 asks for by name.
//!
//! The capstone's list of things ONI got wrong has "gas behaves nothing
//! like gas" on it, and the dictation behind it (`dictation-dumps/gnomes.md`)
//! is specific about what it means: *"in a typical room you'd have a layer
//! of CO2 at the bottom and O2 on top of that ... surely we can do better
//! than that — gas is mix. CO2 is heavier but it doesn't all fall to the
//! bottom of a room."*
//!
//! Night 2 built pressure but not mixing, because one cell held one
//! material: a cell of CO₂ next to a cell of air could swap with it or push
//! it, never blend into it. The result was the very thing the dictation
//! objects to — a slab of CO₂ that fell, spread, and lay on the floor as a
//! flat layer with clean air above it. Nights 2 and 3 then added two rules
//! (`advect`, a whole-parcel swap toward low pressure, and `expand`, a
//! three-cell shove) whose only job was to move gas between cells that could
//! not share. Both are gone: a gas cell now holds a mixture (see
//! [`Cell`](crate::world::Cell)), so any two gas cells can simply exchange
//! mass, and two rules do everything the old three did and the thing none
//! of them could.
//!
//! ## The two rules
//!
//! - [`flow`] — **bulk flow**. Mixture moves from the cell at higher total
//!   pressure `P = T·Σ m·R` to the one at lower, carrying its composition
//!   with it. This is sound-speed fast: it is what levels a pressurised
//!   bottle into a room and what lets a boiled cell of steam expand to
//!   thirteen hundred times its volume, and it is clamped to the exact
//!   transfer that levels a pair, so no timestep makes it overshoot.
//! - [`interdiffuse`] — **molecular diffusion**. Each species moves down
//!   its own *partial*-pressure gradient, independently. At uniform total
//!   pressure the partial-pressure gradients of two species are equal and
//!   opposite, so this swaps them without disturbing the total — it is how
//!   CO₂ spreads into air, and it is much slower than bulk flow.
//!
//! Both move mass from one cell to another and re-solve the receiving
//! cell's temperature from its energy, so both conserve mass and energy by
//! construction.
//!
//! ## Buoyancy is still not in here
//!
//! `physics::apply_gravity` sinks and floats gas cells by the same rule that
//! drops sand through water. What it compares for two gas cells is
//! [`reference_density`] — what each would weigh at one atmosphere — so a
//! CO₂-rich parcel sinks, a steam-rich or hot one rises, and a merely
//! *compressed* one does neither, because it expands long before it could
//! fall. Heavy gas therefore does sink, and does pool: what stops it being
//! a layer forever is [`interdiffuse`] working on it the whole time it lies
//! there.

use crate::material::{MaterialId, Mobility, MIX_SLOTS};
use crate::math::{GridIndex, Scalar};
use crate::world::{World, NO_MIX};

/// How much of the pressure gap between two gas cells bulk flow closes per
/// second of simulated time, before viscosity. Deliberately far faster than
/// any timestep resolves — sound crosses a cell in microseconds — so in
/// practice every pair is levelled fully on every sweep, and the clamp to
/// the exact levelling amount is what keeps that from overshooting.
const RELAXATION_PER_SECOND: Scalar = 80.0;

/// How much of a species' partial-pressure gap between two neighbouring
/// cells molecular diffusion closes per second.
///
/// A tuned number, flagged the same way `conductivity` is in the material
/// table. Real molecular diffusion of CO₂ through air is some minutes per
/// ten centimetres; real rooms mix far faster than that because they are
/// stirred by convection this grid only partly resolves. The figure is
/// chosen so a released slab of CO₂ visibly sinks and spreads first, and
/// then visibly mixes upward over the following minute of simulated time —
/// both halves of the dictation's "heavier, but it doesn't all fall to the
/// bottom".
const DIFFUSION_PER_SECOND: Scalar = 1.5;

/// The pressure of the cell at flat position `p`, or zero if it isn't a gas.
pub fn pressure_at(world: &World, p: usize) -> Scalar {
    world.pressure_at(p)
}

/// The pressure at a grid coordinate — the reading a test or a report makes.
pub fn pressure(world: &World, index: GridIndex) -> Scalar {
    pressure_at(world, world.linear_index(index))
}

/// The partial pressure of `species` at flat position `p`.
pub fn partial_pressure_at(world: &World, p: usize, species: MaterialId) -> Scalar {
    let m = world.materials().get(species);
    world.grams_of_at(p, species) * m.gas_constant * world.cell_at(p).temperature
}

/// What the gas cell at `p` would weigh per cell volume at the table's
/// reference pressure — the density buoyancy compares between gas cells.
///
/// Comparing actual masses instead (which is what night 2 did for two cells
/// of the same gas) makes a compressed pocket sink through thinner gas of
/// its own kind. That is not what a compressed pocket does: it expands, at
/// the speed of sound, long before it could fall. What decides whether a
/// parcel of gas rises or sinks is its density *at the pressure around it*:
///
/// ```text
/// ρ* = m / V_ref,   V_ref = P / P_ref  +  Σ m_mist / ρ_liquid
/// ```
///
/// — the volume its gas would take up at one atmosphere, plus the volume of
/// any droplets hanging in it. For a pure gas at one atmosphere that is its
/// nominal density; hotter gas comes out lighter (so it rises); gas with
/// more CO₂ in it comes out heavier (so it sinks); and a cell that is all
/// mist comes out exactly as dense as the liquid it is made of, which is
/// what a raindrop is. An empty cell weighs nothing.
pub fn reference_density(world: &World, p: usize) -> Scalar {
    let cell = world.cell_at(p);
    if cell.mass <= 0.0 {
        return 0.0;
    }
    let table = world.materials();
    let reference = table.reference_pressure();
    let mut volume = if reference > 0.0 {
        cell.pressure(table) / reference
    } else {
        0.0
    };
    for (&id, &m) in table.slots().iter().zip(cell.mix.iter()) {
        let material = table.get(id);
        if !table.is_gas(id) && m > 0.0 && material.density > 0.0 {
            volume += m / material.density;
        }
    }
    if volume <= 0.0 {
        return table.get(cell.material).density;
    }
    cell.mass / volume
}

/// Whether the cell at `p` is a gas the rules here may move.
fn is_flowing_gas(world: &World, p: usize) -> bool {
    world.is_gas_at(p) && world.material_of(p).mobility != Mobility::Static
}

/// How many Gauss-Seidel sweeps of pressure relaxation run per step,
/// alternating direction, so a disturbance propagates both ways.
const FLOW_SWEEPS: usize = 2;

/// How fast a flowing gas loses its momentum, per second — viscosity, in
/// the only form this grid has. See [`flow`].
///
/// Lower is more like a real gas and less like syrup: the pressure drop a
/// steady current needs to keep going is roughly this times what it would
/// need with no momentum at all. Too low and every disturbance rings round
/// a sealed room for seconds as a standing wave. Tuned, and flagged as such.
const FLOW_DAMPING_PER_SECOND: Scalar = 2.0;

/// The most of a cell's contents a single face may carry away on momentum
/// in one step. The faces are applied one after another, so even a cell
/// being emptied through all four at once keeps a remainder.
const MAX_CARRIED_FRACTION: Scalar = 0.5;

/// Bulk flow: gas with momentum, driven by pressure.
///
/// Every face between two gas cells carries a mass flux, stored on the
/// world, and each step does two things with it:
///
/// 1. **Carry.** Move the mass the flux says — a parcel of the upstream
///    cell's mixture, composition and all — across each face.
/// 2. **Push.** Relax pressure between every adjacent pair, Gauss-Seidel,
///    [`FLOW_SWEEPS`] times; whatever mass that moves across a face is the
///    push the pressure gradient gave it, and is added to the face's flux
///    for next step, which then decays by [`FLOW_DAMPING_PER_SECOND`].
///
/// The first version had step 2 only, and gas had no momentum: every step,
/// pressure had to start every current from rest. That is fine for a room
/// levelling out and fatal for anything that has to *carry* gas somewhere
/// steadily. Pressure relaxation between neighbours can pass only a
/// fraction of a cell's contents across a face per sweep, so a steady
/// current needs a steep gradient to drive it, and through a narrow opening
/// the gradient needed is atmospheres. A still showed it at once: its pot
/// had to sit at two atmospheres to push vapour through its lyne arm, which
/// put spirit's boiling point above the pot's own walls, so the vapour
/// condensed back into the pot. With momentum a current, once going, keeps
/// going; pressure only has to correct it, and the gradient a steady flow
/// needs falls by roughly the damping factor. That is also, not by
/// coincidence, the first piece of *fluid dynamics* in this simulation, as
/// opposed to fluid statics.
///
/// Both halves move mass through [`move_fraction`], so both conserve mass
/// and energy by construction. The flux itself is not energy the world
/// accounts for — there is no kinetic energy in the books, just as there is
/// no `P·dV` work — and it cannot become any, because all it ever does is
/// decide which conserving transfer happens next.
pub fn flow(world: &mut World, dt: Scalar) {
    if dt <= 0.0 {
        return;
    }
    let (w, h) = (world.width(), world.height());
    let n = w * h;
    let mut carried_x = vec![0.0; n];
    let mut carried_y = vec![0.0; n];
    for p in 0..n {
        let (i, j) = (p % w, p / w);
        for axis in 0..2 {
            let q = match axis {
                0 if i + 1 < w => p + 1,
                1 if j + 1 < h => p + w,
                _ => continue,
            };
            let flux = if axis == 0 {
                world.flux_x[p]
            } else {
                world.flux_y[p]
            };
            if flux == 0.0 {
                continue;
            }
            if !is_flowing_gas(world, p) || !is_flowing_gas(world, q) {
                // Something that is not gas now stands in the way; the
                // current stops against it.
                if axis == 0 {
                    world.flux_x[p] = 0.0;
                } else {
                    world.flux_y[p] = 0.0;
                }
                continue;
            }
            let (up, down) = if flux > 0.0 { (p, q) } else { (q, p) };
            let upstream = world.cell_at(up).mass;
            if upstream <= 0.0 {
                continue;
            }
            let fraction = (flux.abs() * dt / upstream).min(MAX_CARRIED_FRACTION);
            let moved = move_fraction(world, up, down, fraction) * flux.signum();
            if axis == 0 {
                carried_x[p] = moved;
            } else {
                carried_y[p] = moved;
            }
        }
    }

    let mut pushed_x = vec![0.0; n];
    let mut pushed_y = vec![0.0; n];
    let (wi, hi) = (w as i32, h as i32);
    let sub_dt = dt / FLOW_SWEEPS as Scalar;
    for sweep in 0..FLOW_SWEEPS {
        let forward = sweep % 2 == 0;
        for jj in 0..hi {
            for ii in 0..wi {
                let (i, j) = if forward {
                    (ii, jj)
                } else {
                    (wi - 1 - ii, hi - 1 - jj)
                };
                let p = world.linear_index(GridIndex::new(i, j));
                if !is_flowing_gas(world, p) {
                    continue;
                }
                let step = if forward { 1 } else { -1 };
                for n_idx in [GridIndex::new(i + step, j), GridIndex::new(i, j + step)] {
                    if !world.in_bounds(n_idx) {
                        continue;
                    }
                    let q = world.linear_index(n_idx);
                    if !is_flowing_gas(world, q) {
                        continue;
                    }
                    let moved = bulk_exchange(world, p, q, sub_dt);
                    if moved == 0.0 {
                        continue;
                    }
                    // Record it on the face, signed toward +i / +j.
                    match (n_idx.i - i, n_idx.j - j) {
                        (1, _) => pushed_x[p] += moved,
                        (-1, _) => pushed_x[q] -= moved,
                        (_, 1) => pushed_y[p] += moved,
                        _ => pushed_y[q] -= moved,
                    }
                }
            }
        }
    }

    let keep = (-FLOW_DAMPING_PER_SECOND * dt).exp();
    for p in 0..n {
        let fx = (carried_x[p] + pushed_x[p]) / dt * keep;
        let fy = (carried_y[p] + pushed_y[p]) / dt * keep;
        // A current too small to move anything a test could see is let go,
        // so a settled room is genuinely still rather than faintly ringing.
        world.flux_x[p] = if (fx * dt).abs() > 1e-12 { fx } else { 0.0 };
        world.flux_y[p] = if (fy * dt).abs() > 1e-12 { fy } else { 0.0 };
    }
}

/// Moves mixture from the higher-pressure cell of a pair to the lower, as
/// far toward equal pressure as `dt` allows. Returns the grams moved from
/// `p` to `q` (negative if they went the other way).
fn bulk_exchange(world: &mut World, p: usize, q: usize, dt: Scalar) -> Scalar {
    let (pp, pq) = (pressure_at(world, p), pressure_at(world, q));
    // Already level, to the precision anything downstream could notice —
    // most of a settled room, skipped before any cell is rewritten.
    if (pp - pq).abs() <= 1e-12 * pp.max(pq) {
        return 0.0;
    }
    let (src, dst, high, low) = if pp > pq {
        (p, q, pp, pq)
    } else {
        (q, p, pq, pp)
    };
    let (source, dest) = (world.cell_at(src), world.cell_at(dst));
    let ts = source.temperature;
    // An empty cell's temperature is a leftover label, not a property of
    // anything; whatever flows in will bring its own.
    let td = if dest.mass > 0.0 {
        dest.temperature
    } else {
        ts
    };
    if high <= 0.0 || ts <= 0.0 || td <= 0.0 {
        return 0.0;
    }
    // The fraction `f` of the source's contents that levels the pair,
    // holding temperatures fixed: from `(1 − f)·P_s == P_d + f·P_s·T_d/T_s`.
    let levelling = (high - low) * ts / (high * (ts + td));
    let viscosity = world.material_of(src).viscosity;
    let rate = (RELAXATION_PER_SECOND * dt / (1.0 + viscosity)).clamp(0.0, 1.0);
    let moved = move_fraction(world, src, dst, (levelling * rate).clamp(0.0, 1.0));
    if src == p {
        moved
    } else {
        -moved
    }
}

/// Molecular diffusion: every gas species relaxes its own partial-pressure
/// gradient between every adjacent pair of gas cells.
///
/// Mist is left alone — droplets do not diffuse, they are carried by bulk
/// flow and they fall.
pub fn interdiffuse(world: &mut World, dt: Scalar) {
    let rate = (DIFFUSION_PER_SECOND * dt).clamp(0.0, 0.5);
    if rate <= 0.0 {
        return;
    }
    let (w, h) = (world.width() as i32, world.height() as i32);
    for j in 0..h {
        for i in 0..w {
            let p = world.linear_index(GridIndex::new(i, j));
            if !is_flowing_gas(world, p) {
                continue;
            }
            for n in [GridIndex::new(i + 1, j), GridIndex::new(i, j + 1)] {
                if !world.in_bounds(n) {
                    continue;
                }
                let q = world.linear_index(n);
                if is_flowing_gas(world, q) {
                    exchange_species(world, p, q, rate);
                }
            }
        }
    }
}

/// One pair's worth of [`interdiffuse`]: every gas species moves `rate` of
/// the way to equal partial pressure, all at once, and each cell's
/// temperature is solved once from what it gained and lost.
///
/// Doing the species one at a time would be the same arithmetic and four
/// times the work, and this runs on every adjacent pair of gas cells every
/// step; a pair that is already level — most of any settled room — is
/// skipped before anything is written.
fn exchange_species(world: &mut World, p: usize, q: usize, rate: Scalar) {
    let (mut a, mut b) = (world.cell_at(p), world.cell_at(q));
    // An empty cell's temperature is only a label; level against the
    // other's.
    let ta = if a.mass > 0.0 {
        a.temperature
    } else {
        b.temperature
    };
    let tb = if b.mass > 0.0 {
        b.temperature
    } else {
        a.temperature
    };
    if ta <= 0.0 || tb <= 0.0 {
        return;
    }
    let props = *world.materials().slot_props();
    let negligible = 1e-12 * (a.mass + b.mass);
    // Grams of each species, q -> p, that would equalise its partial
    // pressure across the pair. Its gas constant cancels: only its own
    // masses and the two temperatures matter.
    let mut moved = NO_MIX;
    let mut any = false;
    for (s, slot) in moved.iter_mut().enumerate() {
        if !props.is_gas[s] {
            continue;
        }
        let levelling = (b.mix[s] * tb - a.mix[s] * ta) / (ta + tb);
        let grams = (levelling * rate).clamp(-a.mix[s], b.mix[s]);
        if grams.abs() > negligible {
            *slot = grams;
            any = true;
        }
    }
    if !any {
        return;
    }
    let table = world.materials();
    let (ea, eb) = (a.energy(table), b.energy(table));
    // Energy into `a`: what arrives from `b` at `b`'s temperature, less what
    // leaves `a` at its own.
    let mut into_a = 0.0;
    for (s, &grams) in moved.iter().enumerate() {
        if grams == 0.0 {
            continue;
        }
        let from_t = if grams > 0.0 {
            b.temperature
        } else {
            a.temperature
        };
        into_a += grams * (props.heat_capacity[s] * from_t + props.latent_energy[s]);
        a.mix[s] += grams;
        b.mix[s] -= grams;
    }
    a.refresh(table);
    b.refresh(table);
    a.solve_temperature(table, ea + into_a);
    b.solve_temperature(table, eb - into_a);
    world.set_cell_at(p, a);
    world.set_cell_at(q, b);
}

/// Moves `fraction` of *everything* in one gas cell into another — the
/// parcel keeps the source's composition, mist and all.
///
/// The source's temperature does not change (taking part of a body away
/// leaves the rest exactly as hot as it was); the destination's is
/// re-solved from its energy plus the parcel's. That is the whole of why
/// this conserves.
pub(crate) fn move_fraction(world: &mut World, from: usize, to: usize, fraction: Scalar) -> Scalar {
    if fraction <= 0.0 {
        return 0.0;
    }
    let mut source = world.cell_at(from);
    let mut dest = world.cell_at(to);
    let mut parcel = NO_MIX;
    for (x, &m) in parcel.iter_mut().zip(source.mix.iter()) {
        *x = m * fraction;
    }
    let table = world.materials();
    let props = table.slot_props();
    let parcel_energy: f64 = (0..MIX_SLOTS)
        .map(|s| parcel[s] * (props.heat_capacity[s] * source.temperature + props.latent_energy[s]))
        .sum();
    let dest_energy = dest.energy(table) + parcel_energy;
    for (s, &x) in parcel.iter().enumerate() {
        source.mix[s] -= x;
        dest.mix[s] += x;
    }
    source.refresh(table);
    dest.refresh(table);
    dest.solve_temperature(table, dest_energy);
    world.set_cell_at(from, source);
    world.set_cell_at(to, dest);
    parcel.iter().sum()
}

/// Moves `grams` of the species in mixture slot `slot` from one gas cell to
/// another. Same bookkeeping as [`move_fraction`], one species at a time.
pub(crate) fn move_species(world: &mut World, from: usize, to: usize, slot: usize, grams: Scalar) {
    if grams <= 0.0 {
        return;
    }
    let mut source = world.cell_at(from);
    let mut dest = world.cell_at(to);
    let table = world.materials();
    let species = table.get(table.slots()[slot]);
    let dest_energy = dest.energy(table) + grams * species.specific_energy(source.temperature);
    source.mix[slot] -= grams;
    dest.mix[slot] += grams;
    source.refresh(table);
    dest.refresh(table);
    dest.solve_temperature(table, dest_energy);
    world.set_cell_at(from, source);
    world.set_cell_at(to, dest);
}

/// The gas rules, in the order `physics::step` runs them.
pub fn step(world: &mut World, dt: Scalar) {
    flow(world, dt);
    interdiffuse(world, dt);
}

/// The spread of pressures across every gas cell in the world, as
/// `(min, max)` — the number a headless test asserts on to say "the
/// atmosphere has equalised", and `None` if there is no gas at all. Now that
/// gases mix, this is the meaningful figure: CO₂ and air share cells and
/// share one pressure.
pub fn pressure_range(world: &World) -> Option<(Scalar, Scalar)> {
    range_over(world, None)
}

/// [`pressure_range`] restricted to gas cells whose largest component is
/// `material`.
pub fn pressure_range_of(world: &World, material: MaterialId) -> Option<(Scalar, Scalar)> {
    range_over(world, Some(material))
}

fn range_over(world: &World, only: Option<MaterialId>) -> Option<(Scalar, Scalar)> {
    let mut range: Option<(Scalar, Scalar)> = None;
    for p in 0..world.width() * world.height() {
        if !world.is_gas_at(p) {
            continue;
        }
        if only.is_some_and(|m| world.cell_at(p).material != m) {
            continue;
        }
        let value = pressure_at(world, p);
        range = Some(match range {
            None => (value, value),
            Some((lo, hi)) => (lo.min(value), hi.max(value)),
        });
    }
    range
}

/// Mass-weighted mean height of a material, in cell rows, counting its
/// share of every mixture — how a test says "the CO₂ ended up low in the
/// room" in a number rather than a picture. `None` if the world holds none.
pub fn mean_height_of(world: &World, material: MaterialId) -> Option<f64> {
    let w = world.width();
    let mut mass = 0.0f64;
    let mut weighted = 0.0f64;
    for p in 0..w * world.height() {
        let grams = world.grams_of_at(p, material);
        mass += grams;
        weighted += grams * (p / w) as f64;
    }
    if mass <= 0.0 {
        None
    } else {
        Some(weighted / mass)
    }
}

/// The mole fraction of `species` in each row's gas, bottom row first —
/// its partial pressure summed across the row over the row's total. `None`
/// for a row with no gas in it.
///
/// This is the number the dictation's claim is actually about. A layer is a
/// profile that is 1 at the floor and 0 above it; a mixed room is a profile
/// that is the same all the way up; "heavier, but it doesn't all fall to the
/// bottom" is one that leans toward the floor without reaching zero at the
/// ceiling.
pub fn fraction_profile(world: &World, species: MaterialId) -> Vec<Option<f64>> {
    let (w, h) = (world.width(), world.height());
    (0..h)
        .map(|j| {
            let (mut part, mut total) = (0.0f64, 0.0f64);
            for i in 0..w {
                let p = j * w + i;
                if !world.is_gas_at(p) {
                    continue;
                }
                part += partial_pressure_at(world, p, species) as f64;
                total += pressure_at(world, p) as f64;
            }
            (total > 0.0).then(|| part / total)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::{terrarium as t, MaterialTable};
    use crate::physics;
    use crate::world::Cell;

    fn air_world(w: usize, h: usize) -> World {
        World::new(w, h, MaterialTable::terrarium(), t::AIR, 291.0)
    }

    /// Scales everything in a gas cell by `factor` — a pressurised or
    /// evacuated cell, painted for a test.
    fn compress(w: &mut World, index: GridIndex, factor: Scalar) {
        let mut cell = w.cell(index);
        for m in cell.mix.iter_mut() {
            *m *= factor;
        }
        w.set_cell(index, cell);
    }

    fn mixture(w: &World, parts: &[(MaterialId, Scalar)], temperature: Scalar) -> Cell {
        Cell::gas(w.materials(), parts, temperature)
    }

    /// The table's gas densities are derived from one atmosphere, so a
    /// freshly painted world of mixed gases starts out at uniform pressure
    /// — no phantom gradient to level.
    #[test]
    fn every_gas_at_room_temperature_starts_at_the_same_pressure() {
        let table = MaterialTable::terrarium();
        let pressures: Vec<Scalar> = [t::AIR, t::STEAM, t::CO2, t::SPIRIT]
            .iter()
            .map(|&id| {
                let m = table.get(id);
                m.pressure(m.density, 291.0)
            })
            .collect();
        for pair in pressures.windows(2) {
            assert!(
                (pair[0] - pair[1]).abs() < 1e-4 * pair[0],
                "gas pressures should agree at room conditions, got {pressures:?}"
            );
        }
    }

    #[test]
    fn a_compressed_pocket_equalises_across_a_room_and_conserves_mass() {
        let mut w = air_world(16, 8);
        compress(&mut w, GridIndex::new(0, 0), 10.0);
        w.rebaseline();

        let before = w.total_mass();
        let (lo0, hi0) = pressure_range(&w).unwrap();
        assert!(hi0 > lo0 * 5.0, "the bottle should start far from level");

        for _ in 0..400 {
            flow(&mut w, 0.05);
        }

        let (lo, hi) = pressure_range(&w).unwrap();
        assert!(
            hi - lo < (hi0 - lo0) * 1e-3,
            "pressure should level out, spread went {} -> {}",
            hi0 - lo0,
            hi - lo
        );
        // Tolerances are relative and sized for `Scalar = f32`: a cell of
        // air masses about a thousandth of a gram, so absolute figures here
        // live where f32 has only ~7 digits to give.
        assert!(
            (w.total_mass() - before).abs() < 1e-6 * before,
            "gas flow must not create or destroy mass: {before} -> {}",
            w.total_mass()
        );
        let r = w.conservation_residuals();
        assert!(
            r.mass_relative.abs() < 1e-6 && r.energy_relative.abs() < 1e-6,
            "gas flow must not create or destroy mass or energy, residuals {r:?}"
        );
    }

    #[test]
    fn gas_expands_into_a_vacuum_rather_than_leaving_it_empty() {
        let mut w = air_world(12, 4);
        // Empty the right-hand half: cells with essentially no gas in them,
        // which is what a vacuum is here.
        for j in 0..4 {
            for i in 6..12 {
                compress(&mut w, GridIndex::new(i, j), 1e-6);
            }
        }
        w.rebaseline();
        for _ in 0..300 {
            flow(&mut w, 0.05);
        }
        let (lo, hi) = pressure_range(&w).unwrap();
        assert!(
            lo > hi * 0.95,
            "the vacuum should have filled: pressures {lo} .. {hi}"
        );
        assert!(w.conservation_residuals().mass_relative.abs() < 1e-6);
    }

    #[test]
    fn a_transfer_between_gas_cells_of_different_temperature_conserves_energy() {
        let mut w = air_world(2, 1);
        let hot = GridIndex::new(0, 0);
        let mut cell = mixture(&w, &[(t::AIR, 0.0048), (t::STEAM, 0.001)], 600.0);
        cell.flow_dir = 0;
        w.set_cell(hot, cell);
        w.rebaseline();
        let energy = w.total_energy();
        for _ in 0..50 {
            step(&mut w, 0.05);
        }
        assert!(
            (w.total_energy() - energy).abs() < 1e-6 * energy.abs(),
            "energy moved with the gas: {} -> {}",
            energy,
            w.total_energy()
        );
    }

    /// Two gases in contact, at one pressure, mix — which a
    /// one-material-per-cell grid could never do.
    #[test]
    fn two_gases_at_one_pressure_interdiffuse() {
        let mut w = air_world(12, 1);
        for i in 0..6 {
            w.fill(GridIndex::new(i, 0), t::CO2, 291.0);
        }
        w.rebaseline();
        let (p0, _) = pressure_range(&w).unwrap();
        for _ in 0..2000 {
            step(&mut w, 0.05);
        }
        let far_left = w.grams_of_at(0, t::CO2) / w.cell_at(0).mass;
        let far_right = w.grams_of_at(11, t::CO2) / w.cell_at(11).mass;
        assert!(
            far_right > 0.3 && far_left < 0.8,
            "the two halves should have mixed, CO2 mass fraction {far_left} .. {far_right}"
        );
        let (lo, hi) = pressure_range(&w).unwrap();
        assert!(
            hi - lo < 0.02 * p0,
            "mixing should not disturb the total pressure: {lo} .. {hi}"
        );
        let r = w.conservation_residuals();
        assert!(
            r.mass_relative.abs() < 1e-6 && r.energy_relative.abs() < 1e-6,
            "{r:?}"
        );
        assert!(
            (w.mass_of(t::CO2) - 6.0 * w.materials().get(t::CO2).density).abs() < 1e-7,
            "no CO2 may be made or lost by mixing"
        );
    }

    /// Bulk flow alone never lifts a heavy gas: it answers to total
    /// pressure, and a settled layer is at the same total pressure as the
    /// air above it. (Mixing is `interdiffuse`'s job, tested above.)
    #[test]
    fn bulk_flow_alone_does_not_lift_a_settled_heavy_layer() {
        let mut w = air_world(12, 12);
        for i in 0..12 {
            for j in 0..3 {
                w.fill(GridIndex::new(i, j), t::CO2, 291.0);
            }
        }
        w.rebaseline();
        let before = mean_height_of(&w, t::CO2).expect("co2");
        for _ in 0..200 {
            flow(&mut w, 0.05);
        }
        let after = mean_height_of(&w, t::CO2).expect("co2");
        assert!(
            (after - before).abs() < 0.05,
            "the layer drifted from {before} to {after}"
        );
    }

    /// The dictation's complaint, as a test. A slab of CO2 released near
    /// the ceiling is heavier than the air, so it sinks — and then it does
    /// not simply lie on the floor as a layer with clean air over it.
    #[test]
    fn co2_sinks_but_does_not_all_fall_to_the_bottom() {
        let mut w = air_world(10, 12);
        for i in 2..8 {
            for j in 9..11 {
                w.fill(GridIndex::new(i, j), t::CO2, 291.0);
            }
        }
        w.rebaseline();
        let start = mean_height_of(&w, t::CO2).unwrap();
        physics::run(&mut w, 300, 0.05);
        let sunk = mean_height_of(&w, t::CO2).unwrap();
        assert!(
            sunk < start - 3.0,
            "CO2 is heavier than air and should have sunk, mean height {start} -> {sunk}"
        );
        physics::run(&mut w, 3000, 0.05);
        let profile = fraction_profile(&w, t::CO2);
        let floor = profile[0].unwrap();
        let ceiling = profile[w.height() - 1].unwrap();
        assert!(
            floor > ceiling,
            "it should still lean toward the floor: floor {floor}, ceiling {ceiling}"
        );
        assert!(
            ceiling > 0.25 * floor,
            "but the ceiling should not be clean air: floor {floor}, ceiling {ceiling}"
        );
        let r = w.conservation_residuals();
        assert!(
            r.mass_relative.abs() < 1e-6 && r.energy_relative.abs() < 1e-6,
            "residuals {r:?}"
        );
    }

    /// And, once mixed, it must *stop*. A uniform atmosphere that keeps
    /// shuffling is the shimmer this repo has already been bitten by once.
    #[test]
    fn a_uniform_mixed_atmosphere_stays_put() {
        let mut w = air_world(10, 12);
        for i in 0..10 {
            for j in 0..12 {
                let cell = mixture(&w, &[(t::AIR, 0.0009), (t::CO2, 0.0005)], 291.0);
                w.set_cell(GridIndex::new(i, j), cell);
            }
        }
        w.rebaseline();
        let before: Vec<Cell> = (0..w.width() * w.height()).map(|p| w.cell_at(p)).collect();
        physics::run(&mut w, 50, 0.05);
        let moved = (0..w.width() * w.height())
            .filter(|&p| {
                let (a, b) = (w.cell_at(p), before[p]);
                (a.mass - b.mass).abs() > 1e-6 * b.mass
                    || (a.temperature - b.temperature).abs() > 1e-3
            })
            .count();
        assert_eq!(moved, 0, "{moved} cells of a uniform atmosphere changed");
    }

    /// A compressed parcel expands — it does not sink through thinner gas
    /// of its own kind, which is what comparing raw masses used to make it
    /// do.
    #[test]
    fn a_compressed_parcel_expands_rather_than_sinking() {
        let mut w = air_world(3, 12);
        compress(&mut w, GridIndex::new(1, 11), 20.0);
        w.rebaseline();
        physics::run(&mut w, 40, 0.05);
        let top_row: f64 = (0..3).map(|i| w.cell(GridIndex::new(i, 11)).mass).sum();
        let bottom_row: f64 = (0..3).map(|i| w.cell(GridIndex::new(i, 0)).mass).sum();
        assert!(
            (top_row - bottom_row).abs() < 0.05 * bottom_row,
            "the pocket should have spread evenly, not fallen: top {top_row}, bottom {bottom_row}"
        );
    }

    /// ...while a *hot* parcel of the same gas genuinely is lighter at the
    /// same pressure, and rises.
    ///
    /// Buoyancy and bulk flow only, without conduction, and that is a
    /// finding rather than a convenience. Every gas cell's conductivity is
    /// a per-second exchange coefficient tuned for feel, about two hundred
    /// times what real air manages across a centimetre-sized cell; against a
    /// gas cell's tiny heat capacity it levels a hot parcel with its
    /// neighbours inside a single step, so in the full step a lone hot cell
    /// of air diffuses its heat away rather than rising as a thermal. The
    /// rule under test — hot gas weighs less — is right; what the full step
    /// does with it is decided by that conductivity.
    #[test]
    fn hot_air_rises_through_cold_air() {
        let mut w = air_world(3, 12);
        w.fill(GridIndex::new(1, 0), t::AIR, 600.0);
        w.rebaseline();
        for _ in 0..30 {
            physics::apply_gravity(&mut w);
            flow(&mut w, 0.05);
        }
        let hottest = (0..w.width() * w.height())
            .max_by(|&a, &b| {
                w.cell_at(a)
                    .temperature
                    .partial_cmp(&w.cell_at(b).temperature)
                    .unwrap()
            })
            .unwrap();
        let row = hottest / w.width();
        assert!(
            row > 5,
            "the hot parcel should have risen, it is at row {row}"
        );
        let r = w.conservation_residuals();
        assert!(
            r.mass_relative.abs() < 1e-9 && r.energy_relative.abs() < 1e-9,
            "{r:?}"
        );
    }

    /// A cell of vapour that has just boiled holds a whole gram of gas —
    /// over a thousand atmospheres. It must spread out into the room.
    #[test]
    fn a_boiled_pocket_expands_into_the_room_it_is_let_into() {
        let mut w = air_world(16, 16);
        let hot = mixture(&w, &[(t::STEAM, 1.0)], 380.0);
        w.set_cell(GridIndex::new(8, 8), hot);
        w.rebaseline();
        let (m0, e0) = (w.total_mass(), w.total_energy());
        let before = pressure(&w, GridIndex::new(8, 8));

        for _ in 0..60 {
            step(&mut w, 0.05);
        }

        let with_steam = (0..w.width() * w.height())
            .filter(|&p| w.grams_of_at(p, t::STEAM) > 1e-4)
            .count();
        assert!(
            with_steam > 100,
            "a gram of steam should reach a large part of the room, got {with_steam} cells"
        );
        let (_, hi) = pressure_range(&w).expect("gas");
        assert!(
            hi < before * 0.05,
            "the pocket should have relaxed: {before} -> {hi}"
        );
        assert!((w.total_mass() - m0).abs() / m0 < 1e-5, "mass moved");
        assert!(
            (w.total_energy() - e0).abs() / e0.abs() < 1e-5,
            "energy moved: {e0} -> {}",
            w.total_energy()
        );
    }
}
