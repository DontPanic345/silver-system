//! Gas as a compressible fluid with a real pressure — the piece
//! `NORTH_STARS.md` #4 asks for by name.
//!
//! The capstone's list of things ONI got wrong has "gas behaves nothing
//! like gas" on it: CO2 does not settle, layers are simplified to the point
//! that players build gimmicks around the gaps. Up to tonight this
//! simulation had exactly the same hole from the other direction. A gas
//! cell was a *light solid*: it held one material at a fixed mass, it could
//! be displaced by something heavier falling into it, and it did nothing
//! else. Open a valve on a pressurised tank and nothing happened, because
//! nothing in the world knew what a pressure was.
//!
//! ## What a gas cell is now
//!
//! The [`Cell::mass`] field has been there all along and was, for gases,
//! always equal to the material's density. It is now genuinely variable:
//! a gas cell holds however many grams of gas are packed into it, and with
//! [`Material::gas_constant`] that gives it a pressure,
//!
//! ```text
//! P = m · R · T
//! ```
//!
//! per unit cell volume. Two rules act on that field, and both are
//! conserving by construction:
//!
//! - [`diffuse`] moves **mass** between adjacent cells of the *same* gas,
//!   down the pressure gradient. What leaves one cell arrives in the other
//!   and carries its own energy with it, so both totals are unchanged to
//!   floating-point noise. This is the rule that makes a gas fill the room
//!   it is let into and equalise, and it is clamped to the exact transfer
//!   that levels the pair, so no timestep can make it overshoot.
//! - [`advect`] moves **whole parcels** of gas — a swap of two cells —
//!   toward lower pressure, when the two cells hold *different* gases and
//!   therefore cannot merge. A swap cannot change a sum either.
//!
//! Buoyancy is not in this module at all, deliberately. `physics::
//! apply_gravity` sinks and floats gas by the same one rule that drops sand
//! through water, with no gas-specific code and no per-material `if`: two
//! cells of the same gas are compared by their actual masses (so a
//! compressed pocket sinks through a thin one), two cells of different
//! materials by their nominal densities (so CO2 sinks through air) — see
//! `pick_target` for why that second case cannot use mass.
//!
//! ## The honest limitation
//!
//! One cell holds one material, so two *different* gases in contact cannot
//! interdiffuse: CO2 cannot spread into a cell of air the way it really
//! would, it can only be carried there bodily by [`advect`] or sink there
//! by buoyancy. Partial pressures of a mixture are not modelled. That means
//! a lone over-pressured parcel of one species surrounded by another will
//! wander down the gradient rather than dissolve into it. Bulk flow and
//! stratification are right; molecular mixing is absent. Modelling it
//! properly needs a cell that can hold a mixture, which is a bigger change
//! than tonight, and pretending otherwise in the meantime would be exactly
//! the sort of gap-plugging hack the north star complains about.
//!
//! [`Cell::mass`]: crate::world::Cell::mass
//! [`Material::gas_constant`]: crate::material::Material::gas_constant

use crate::material::{Mobility, Phase};
use crate::math::{GridIndex, Scalar};
use crate::world::World;

/// How much of the pressure gap between two cells of the same gas is closed
/// per second of simulated time, before viscosity is taken into account.
/// Large enough that a room equalises in a visible fraction of a minute,
/// and harmless if it overshoots — the transfer is clamped to the exact
/// levelling amount regardless.
const RELAXATION_PER_SECOND: Scalar = 8.0;

/// Relative pressure excess a parcel needs before it will bodily move into
/// a neighbouring cell of a *different* gas. A margin rather than a bare
/// `>`: without one, two cells whose pressures differ in the last bit swap
/// back and forth forever and the whole atmosphere shimmers.
const ADVECTION_MARGIN: Scalar = 0.05;

/// The hydrostatic head, in pressure units per unit of nominal density
/// difference: how much extra pressure a parcel must hold to climb one cell
/// against a gas lighter than itself.
///
/// This model has no gravity constant and its pressure field has no `ρgh`
/// term, so up and down look identical to [`advect`] unless something says
/// otherwise. The first version of that something (added with the gas work
/// itself) was a blanket refusal: a denser gas never moves up into a
/// lighter one, full stop. It did the job it was written for — a CO₂ vent
/// stopped firing speckles of heavy gas at the ceiling — and it was too
/// strong by a wide margin, because it is also false. A parcel at six
/// hundred atmospheres absolutely does push upward through air, which is
/// why a kettle whistles and why a pot still works at all; under the
/// blanket rule the vapour sat on top of the wash forever and no still
/// could ever be built.
///
/// So the refusal is now a *price*. Climbing costs `LIFT × Δρ` in pressure,
/// which for CO₂ standing in air is about three atmospheres — enough that a
/// settled layer never levitates and a gently-fed vent never fires upward,
/// and nothing at all to a boiling still.
const LIFT: Scalar = 500.0;

/// The pressure of the cell at flat position `p`, or zero if it isn't a gas.
pub fn pressure_at(world: &World, p: usize) -> Scalar {
    let cell = world.cell_at(p);
    world
        .materials()
        .get(cell.material)
        .pressure(cell.mass, cell.temperature)
}

/// The pressure at a grid coordinate — the reading a test or a report makes.
pub fn pressure(world: &World, index: GridIndex) -> Scalar {
    pressure_at(world, world.linear_index(index))
}

/// Whether the cell at `p` is a gas these rules may move at all: mobile,
/// with a real gas constant.
///
/// Note what this does *not* test. It used to also require that no phase
/// change was in flight, which sounded careful and was in fact a
/// show-stopper: a cell of vapour sitting at its own boiling point is
/// *permanently* part-way through a transition, because that is what
/// sitting at a boiling point means. Every such cell was invisible to both
/// rules in this module — a still's entire charge boiled off and then lay
/// on top of the wash like a solid, unable to diffuse, advect, or reach a
/// condenser. Whole-cell movement never had a reason to care: a swap
/// relocates `pending` and `progress` along with everything else. Only
/// [`diffuse`], which moves *part* of a cell's mass, has to think about it,
/// and it does so where it matters — see [`same_transition`].
fn is_movable_gas(world: &World, p: usize) -> bool {
    let m = world.material_of(p);
    m.phase == Phase::Gas && m.mobility != Mobility::Static && m.gas_constant > 0.0
}

/// Whether two cells are at the same point in the same phase change — the
/// condition under which mass may be moved between them.
///
/// Latent progress is carried *per unit mass*, so mass arriving from a cell
/// half-way through condensing has to bring its share of that progress with
/// it; [`move_gas`] mixes it by mass exactly the way it mixes temperature.
/// That mix is only meaningful when both cells are working on the same
/// transition, since `progress` is measured against one. Two cells at
/// different stages simply wait: they are within a step or two of agreeing.
fn same_transition(world: &World, p: usize, q: usize) -> bool {
    world.cell_at(p).pending == world.cell_at(q).pending
}

/// Pressure-driven mass transfer between adjacent cells of the same gas.
///
/// One transfer per adjacent *pair*, visited once, applied immediately, so
/// the sweep is Gauss-Seidel and converges in a handful of steps rather
/// than diffusing one cell per step. Conservation does not depend on the
/// order: every transfer removes from one cell exactly what it adds to the
/// other, and mixes the receiving cell's temperature by mass, which for two
/// bodies of the same material is an identity on energy.
pub fn diffuse(world: &mut World, dt: Scalar) {
    let (w, h) = (world.width() as i32, world.height() as i32);
    for j in 0..h {
        for i in 0..w {
            let p = world.linear_index(GridIndex::new(i, j));
            if !is_movable_gas(world, p) {
                continue;
            }
            for n in [GridIndex::new(i + 1, j), GridIndex::new(i, j + 1)] {
                if !world.in_bounds(n) {
                    continue;
                }
                let q = world.linear_index(n);
                if !is_movable_gas(world, q) {
                    continue;
                }
                if world.cell_at(p).material != world.cell_at(q).material
                    || !same_transition(world, p, q)
                {
                    continue;
                }
                exchange_gas(world, p, q, dt);
            }
        }
    }
}

/// Moves gas between two cells of the same material until their pressures
/// agree, or as far toward that as `dt` allows.
fn exchange_gas(world: &mut World, p: usize, q: usize, dt: Scalar) {
    let (a, b) = (world.cell_at(p), world.cell_at(q));
    let material = world.material_of(p);
    if a.temperature <= 0.0 || b.temperature <= 0.0 {
        return;
    }

    // The transfer q -> p that would leave both cells at one pressure,
    // holding temperatures fixed: from `(m_a + x)·T_a == (m_b - x)·T_b`.
    let levelling =
        (b.mass * b.temperature - a.mass * a.temperature) / (a.temperature + b.temperature);
    if levelling == 0.0 {
        return;
    }

    // Viscosity slows the approach; nothing about this can overshoot,
    // because the fraction is clamped into [0, 1] and applied to the exact
    // levelling amount.
    let fraction = (RELAXATION_PER_SECOND * dt / (1.0 + material.viscosity)).clamp(0.0, 1.0);
    let mut transfer = levelling * fraction;

    // Never move more than a cell actually holds.
    let available = if transfer > 0.0 { b.mass } else { a.mass };
    if transfer.abs() > available {
        transfer = available * transfer.signum();
    }
    if transfer == 0.0 {
        return;
    }

    if transfer > 0.0 {
        move_gas(world, q, p, transfer);
    } else {
        move_gas(world, p, q, -transfer);
    }
}

/// Moves `grams` of gas from one cell to another, both holding the same
/// material.
///
/// Mass conservation is arithmetic: one cell loses exactly what the other
/// gains. Energy conservation follows because the two cells hold the same
/// material, so energy is `mass·(c·T + L + progress)` on both sides, and
/// mixing *both* the destination's temperature and its latent progress by
/// mass leaves that sum over the pair unchanged. The source's temperature
/// and progress do not change — taking gas out of a body does not cool what
/// stays behind, nor un-boil it.
fn move_gas(world: &mut World, from: usize, to: usize, grams: Scalar) {
    debug_assert_eq!(world.cell_at(from).material, world.cell_at(to).material);
    debug_assert_eq!(world.cell_at(from).pending, world.cell_at(to).pending);
    let mut source = world.cell_at(from);
    let mut dest = world.cell_at(to);
    let new_mass = dest.mass + grams;
    if new_mass <= 0.0 {
        return;
    }
    dest.temperature = (dest.mass * dest.temperature + grams * source.temperature) / new_mass;
    dest.progress = (dest.mass * dest.progress + grams * source.progress) / new_mass;
    dest.mass = new_mass;
    source.mass -= grams;
    world.set_cell_at(from, source);
    world.set_cell_at(to, dest);
}

/// Bulk flow: a parcel of gas moves into a neighbouring cell of a
/// *different* gas when it is at meaningfully higher pressure.
///
/// This is the only way one species reaches a region occupied by another —
/// see the module doc's note on interdiffusion. It is a swap, so it cannot
/// change any total; it moves a high-pressure parcel toward the lowest
/// pressure it can see, and stops as soon as pressures agree to within
/// [`ADVECTION_MARGIN`], which is what stops the atmosphere churning
/// forever once it has settled.
pub fn advect(world: &mut World) {
    let (w, h) = (world.width() as i32, world.height() as i32);
    let mut moved = vec![false; (w * h) as usize];
    // Alternate the scan direction, for the same reason `apply_gravity`
    // does: a fixed order gives the whole atmosphere a slow preferred drift.
    let rightward = world.step_count.is_multiple_of(2);

    for j in 0..h {
        for i_raw in 0..w {
            let i = if rightward { i_raw } else { w - 1 - i_raw };
            let p = world.linear_index(GridIndex::new(i, j));
            if moved[p] || !is_movable_gas(world, p) {
                continue;
            }
            let here = pressure_at(world, p);
            if here <= 0.0 {
                continue;
            }

            let mut best: Option<(usize, Scalar)> = None;
            for n in [
                GridIndex::new(i - 1, j),
                GridIndex::new(i + 1, j),
                GridIndex::new(i, j - 1),
                GridIndex::new(i, j + 1),
            ] {
                if !world.in_bounds(n) {
                    continue;
                }
                let q = world.linear_index(n);
                if moved[q] || !is_movable_gas(world, q) {
                    continue;
                }
                if world.cell_at(q).material == world.cell_at(p).material {
                    continue; // same species: `diffuse` handles this pair.
                }
                // Climbing into a lighter gas costs the hydrostatic head —
                // see `LIFT`. Charged as a surcharge on the target's
                // pressure rather than as a veto, so the comparison below
                // stays one rule: a parcel moves to the cheapest place it
                // can see, and up is dearer than sideways.
                let there = pressure_at(world, q)
                    + if n.j > j {
                        LIFT * (world.material_of(p).density - world.material_of(q).density)
                            .max(0.0)
                    } else {
                        0.0
                    };
                if best.is_none_or(|(_, p_best)| there < p_best) {
                    best = Some((q, there));
                }
            }

            if let Some((q, there)) = best {
                if here > there * (1.0 + ADVECTION_MARGIN) {
                    world.swap_cells(p, q);
                    moved[p] = true;
                    moved[q] = true;
                }
            }
        }
    }
}

/// Both gas rules, in the order `physics::step` runs them.
pub fn step(world: &mut World, dt: Scalar) {
    diffuse(world, dt);
    advect(world);
}

/// The spread of pressures across every gas cell in the world, as
/// `(min, max)` — the number a headless test asserts on to say "the
/// atmosphere has equalised", and `None` if there is no gas at all.
pub fn pressure_range(world: &World) -> Option<(Scalar, Scalar)> {
    range_over(world, None)
}

/// [`pressure_range`] restricted to one gas. Worth having separately
/// because two *different* gases in this model cannot share a cell and so
/// cannot equalise with each other (see the module doc): "has this gas
/// settled" is a question about one species at a time.
pub fn pressure_range_of(
    world: &World,
    material: crate::material::MaterialId,
) -> Option<(Scalar, Scalar)> {
    range_over(world, Some(material))
}

fn range_over(
    world: &World,
    only: Option<crate::material::MaterialId>,
) -> Option<(Scalar, Scalar)> {
    let mut range: Option<(Scalar, Scalar)> = None;
    for p in 0..world.width() * world.height() {
        if world.material_of(p).phase != Phase::Gas || world.material_of(p).gas_constant <= 0.0 {
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

/// Mass-weighted mean height of a material, in cell rows — how a test says
/// "the CO2 ended up underneath the air" in a number rather than a picture.
/// `None` if the world holds none of it.
pub fn mean_height_of(world: &World, material: crate::material::MaterialId) -> Option<f64> {
    let w = world.width();
    let mut mass = 0.0f64;
    let mut weighted = 0.0f64;
    for p in 0..w * world.height() {
        let cell = world.cell_at(p);
        if cell.material != material {
            continue;
        }
        mass += cell.mass as f64;
        weighted += cell.mass as f64 * (p / w) as f64;
    }
    if mass <= 0.0 {
        None
    } else {
        Some(weighted / mass)
    }
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

    /// The table's gas densities are derived from one atmosphere, so a
    /// freshly painted world of mixed gases starts out at uniform pressure
    /// — no phantom gradient to level.
    #[test]
    fn every_gas_at_room_temperature_starts_at_the_same_pressure() {
        let table = MaterialTable::terrarium();
        let pressures: Vec<Scalar> = [t::AIR, t::STEAM, t::CO2]
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
        w.rebaseline();
        // Ten times the mass in one corner cell: a pressurised bottle.
        let idx = GridIndex::new(0, 0);
        let mut cell = w.cell(idx);
        let extra = cell.mass * 9.0;
        cell.mass += extra;
        w.set_cell(idx, cell);
        w.rebaseline();

        let before = w.total_mass();
        let (lo0, hi0) = pressure_range(&w).unwrap();
        assert!(hi0 > lo0 * 5.0, "the bottle should start far from level");

        for _ in 0..400 {
            diffuse(&mut w, 0.05);
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
            "gas diffusion must not create or destroy mass: {before} -> {}",
            w.total_mass()
        );
        let r = w.conservation_residuals();
        assert!(
            r.mass_relative.abs() < 1e-6 && r.energy_relative.abs() < 1e-6,
            "gas diffusion must not create or destroy mass or energy, residuals {r:?}"
        );
    }

    #[test]
    fn gas_expands_into_a_vacuum_rather_than_leaving_it_empty() {
        let mut w = air_world(12, 4);
        // Empty the right-hand half: air cells with essentially no gas in
        // them, which is what a vacuum is here.
        for j in 0..4 {
            for i in 6..12 {
                let idx = GridIndex::new(i, j);
                let mut cell = w.cell(idx);
                cell.mass = 1e-9;
                w.set_cell(idx, cell);
            }
        }
        w.rebaseline();
        for _ in 0..300 {
            diffuse(&mut w, 0.05);
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
        let mut cell = w.cell(hot);
        cell.temperature = 600.0;
        cell.mass *= 4.0;
        w.set_cell(hot, cell);
        w.rebaseline();
        let energy = w.total_energy();
        for _ in 0..50 {
            diffuse(&mut w, 0.05);
        }
        assert!(
            (w.total_energy() - energy).abs() < 1e-6 * energy.abs(),
            "energy moved with the gas: {} -> {}",
            energy,
            w.total_energy()
        );
    }

    /// The ONI complaint, as a test: CO2 must end up under the air on its
    /// own, with no rule anywhere naming CO2.
    #[test]
    fn co2_settles_below_air_without_a_rule_that_says_so() {
        let mut w = air_world(10, 12);
        // A ceiling-high slab of CO2, released at the top of the room.
        for i in 2..8 {
            for j in 9..11 {
                w.fill(GridIndex::new(i, j), t::CO2, 291.0);
            }
        }
        w.rebaseline();
        let start = mean_height_of(&w, t::CO2).unwrap();
        physics::run(&mut w, 300, 0.05);
        let end = mean_height_of(&w, t::CO2).unwrap();
        let air = mean_height_of(&w, t::AIR).unwrap();
        assert!(
            end < 3.0,
            "CO2 should have sunk to the floor, mean height {start} -> {end}"
        );
        assert!(
            end < air,
            "CO2 should sit below the air, CO2 at {end}, air at {air}"
        );
        let r = w.conservation_residuals();
        assert!(
            r.mass.abs() < 1e-9 && r.energy.abs() < 1e-3,
            "residuals {r:?}"
        );
    }

    /// And, having settled, it must *stop*. A layered atmosphere that keeps
    /// shuffling is the shimmer this repo has already been bitten by once.
    #[test]
    fn a_settled_atmosphere_stops_moving() {
        let mut w = air_world(10, 12);
        for i in 0..10 {
            for j in 0..3 {
                w.fill(GridIndex::new(i, j), t::CO2, 291.0);
            }
        }
        w.rebaseline();
        physics::run(&mut w, 200, 0.05);
        let before: Vec<Cell> = (0..w.width() * w.height()).map(|p| w.cell_at(p)).collect();
        physics::run(&mut w, 50, 0.05);
        let changed = (0..w.width() * w.height())
            .filter(|&p| w.cell_at(p).material != before[p].material)
            .count();
        assert_eq!(
            changed, 0,
            "a stratified atmosphere should be at rest, {changed} cells still swapping"
        );
    }

    /// Buoyancy reads a cell's real mass, not its material's nominal
    /// density — so a compressed pocket of gas sinks through a thin one of
    /// the very same material.
    #[test]
    fn a_compressed_parcel_sinks_through_thinner_gas_of_the_same_material() {
        let mut w = air_world(3, 8);
        let top = GridIndex::new(1, 7);
        let mut cell = w.cell(top);
        cell.mass *= 20.0;
        w.set_cell(top, cell);
        w.rebaseline();
        let heavy = w.cell(top).mass;
        physics::apply_gravity(&mut w);
        assert!(
            w.cell(top).mass < heavy,
            "the heavy parcel should have started falling"
        );
    }
}
