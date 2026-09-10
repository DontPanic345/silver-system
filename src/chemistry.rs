//! Reactions: the chemistry tier, run as one generic pass over touching
//! pairs of cells.
//!
//! `NORTH_STARS.md` #2 states the content order this project works
//! through — *physics, then chemistry, then biology, then a game layer* —
//! and nights 1 and 2 finished the headline items of the physics tier
//! (movement, heat, phase change, pressure). This is the first of the next
//! one, and it is deliberately the smallest thing that deserves the name:
//! a table of rules saying which two materials, in contact and past a
//! temperature, become which other two.
//!
//! ## Conservation, by construction, again
//!
//! Both invariants fall out of how the rewrite is written rather than being
//! checked afterwards:
//!
//! - **Mass.** Each of the two cells keeps its own mass across the
//!   reaction; only the material label changes. That is stronger than the
//!   global invariant the crate promises, and it is why there is no ledger
//!   entry anywhere in this file.
//! - **Energy.** The pair's total energy is computed *before* the rewrite
//!   and the pair's shared temperature *after* is solved from it, using the
//!   product materials' heat capacities and enthalpy offsets. So the heat
//!   of a reaction is emergent: it is the enthalpy the participants dropped,
//!   showing up as temperature, and it cannot be anything else.
//!
//! The heat a given reaction releases is therefore set entirely by the
//! materials' [`Material::latent_energy`] offsets, which
//! [`MaterialTable::with_chemistry`] solves from the declared figures in
//! the reaction table — the same solve that already derived them from
//! latent heats of melting and boiling. One solve, so a material that takes
//! part in both cannot end up with two different answers.
//!
//! ## What this deliberately is not
//!
//! There is no stoichiometry here, and there cannot be while a cell holds
//! one material at one mass: a reaction says what a touching pair *becomes*,
//! not in what proportion. Where a proportion genuinely matters — a gram of
//! liquid becoming a thousand cells' worth of vapour — the phase-change
//! machinery in `src/physics.rs` already moves mass between materials
//! honestly, and the brewing chain uses it for exactly that half of the job:
//! mashing is a reaction, distilling is two phase transitions.
//!
//! [`Material::latent_energy`]: crate::material::Material::latent_energy
//! [`MaterialTable::with_chemistry`]: crate::material::MaterialTable::with_chemistry

use crate::material::{Direction, Reaction};
use crate::math::{GridIndex, Scalar};
use crate::world::{World, NO_PENDING};

/// Fires every reaction whose conditions are met, at most once per cell per
/// step.
///
/// Cells with a phase change already in flight are skipped, for the same
/// reason `src/gas.rs` skips them: their latent progress is measured
/// per unit mass against a specific transition, and rewriting the material
/// under it would either duplicate or discard that progress. One step's
/// delay costs nothing.
pub fn react(world: &mut World) {
    let reactions: Vec<Reaction> = world.materials().reactions().to_vec();
    if reactions.is_empty() {
        return;
    }
    let (w, h) = (world.width() as i32, world.height() as i32);
    let mut reacted = vec![false; (w * h) as usize];

    for j in 0..h {
        for i in 0..w {
            let p = world.linear_index(GridIndex::new(i, j));
            if reacted[p] || world.cell_at(p).pending != NO_PENDING {
                continue;
            }
            // All four neighbours, not just the +i/+j pair a symmetric rule
            // would need: a reaction is written from the subject's point of
            // view, so "juniper to my left" and "juniper to my right" are
            // different matches of the same rule and both must be tried.
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
                if reacted[q] || world.cell_at(q).pending != NO_PENDING {
                    continue;
                }
                let Some(reaction) = matching(world, p, q, &reactions) else {
                    continue;
                };
                fire(world, p, q, &reaction);
                reacted[p] = true;
                reacted[q] = true;
                break;
            }
        }
    }
}

/// The first reaction in the table whose subject matches the cell at `p`,
/// whose partner matches the cell at `q`, and whose threshold the pair has
/// crossed.
fn matching(world: &World, p: usize, q: usize, reactions: &[Reaction]) -> Option<Reaction> {
    let (a, b) = (world.cell_at(p).material, world.cell_at(q).material);
    reactions
        .iter()
        .find(|r| {
            r.subject.from == a
                && r.partner.from == b
                && crossed(r, equilibrium_temperature(world, p, q))
        })
        .copied()
}

fn crossed(reaction: &Reaction, temperature: Scalar) -> bool {
    match reaction.direction {
        Direction::Heating => temperature > reaction.threshold_k,
        Direction::Cooling => temperature < reaction.threshold_k,
    }
}

/// The temperature the two cells would settle at if left to conduct against
/// each other — the pair's heat-capacity-weighted mean.
///
/// This, rather than either cell's own temperature, is what a reaction's
/// threshold is tested against, and the choice is load-bearing rather than
/// cosmetic. A bush at 315 K hit by cold rain is not a warm tun: the rain
/// carries four times the heat capacity and the pair is really at 287 K.
/// Testing the bush alone would ferment it; testing the pair does not.
pub fn equilibrium_temperature(world: &World, p: usize, q: usize) -> Scalar {
    let (a, b) = (world.cell_at(p), world.cell_at(q));
    let cap_a = a.capacity(world.materials());
    let cap_b = b.capacity(world.materials());
    if cap_a + cap_b <= 0.0 {
        return 0.0;
    }
    ((cap_a * a.temperature + cap_b * b.temperature) / (cap_a + cap_b)) as Scalar
}

/// Rewrites a reacting pair: new materials, the same masses, and one shared
/// temperature solved so the pair's total energy is exactly what it was.
///
/// An arm acting on a gas cell rewrites one *species* of its mixture rather
/// than the whole cell: burning juniper beside a cell of air turns that
/// cell's air into CO₂ and leaves any steam or spirit in it alone. (The
/// table refuses, at construction, an arm that would turn a gas into a
/// non-gas — there is no honest answer to where the rest of the cell goes.)
fn fire(world: &mut World, p: usize, q: usize, reaction: &Reaction) {
    let (mut a, mut b) = (world.cell_at(p), world.cell_at(q));
    let table = world.materials();
    let energy = a.energy(table) + b.energy(table);

    for (cell, arm) in [(&mut a, reaction.subject), (&mut b, reaction.partner)] {
        if cell.is_gas(table) {
            if let (Some(from), Some(to)) = (table.slot(arm.from), table.slot(arm.to)) {
                cell.mix[to] += cell.mix[from];
                cell.mix[from] = 0.0;
            }
            cell.refresh(table);
        } else {
            cell.material = arm.to;
            cell.progress = 0.0;
            cell.pending = NO_PENDING;
        }
    }

    // energy == Σ m·(c·T + L) over both cells' new contents, solved for T.
    let capacity = a.capacity(table) + b.capacity(table);
    if capacity <= 0.0 {
        return;
    }
    let temperature = ((energy - a.stored(table) - b.stored(table)) / capacity) as Scalar;
    a.temperature = temperature;
    b.temperature = temperature;
    world.set_cell_at(p, a);
    world.set_cell_at(q, b);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::{terrarium as t, MaterialTable};
    use crate::physics;

    fn world(w: usize, h: usize) -> World {
        World::new(w, h, MaterialTable::terrarium(), t::AIR, 291.0)
    }

    #[test]
    fn warm_juniper_in_water_mashes_into_wash() {
        let mut w = world(4, 4);
        w.fill(GridIndex::new(1, 1), t::JUNIPER, 320.0);
        w.fill(GridIndex::new(2, 1), t::WATER, 320.0);
        w.rebaseline();
        react(&mut w);
        assert_eq!(w.material_at(GridIndex::new(1, 1)), t::WASH);
        assert_eq!(w.material_at(GridIndex::new(2, 1)), t::WASH);
    }

    #[test]
    fn cold_juniper_in_water_stays_a_bush() {
        let mut w = world(4, 4);
        w.fill(GridIndex::new(1, 1), t::JUNIPER, 291.0);
        w.fill(GridIndex::new(2, 1), t::WATER, 291.0);
        w.rebaseline();
        react(&mut w);
        assert_eq!(w.material_at(GridIndex::new(1, 1)), t::JUNIPER);
        assert_eq!(w.material_at(GridIndex::new(2, 1)), t::WATER);
    }

    /// The threshold reads the *pair*, not the bush: a hot bush hit by cold
    /// rain must not ferment, because the two together are not warm.
    #[test]
    fn a_hot_bush_under_cold_rain_does_not_mash() {
        let mut w = world(4, 4);
        w.fill(GridIndex::new(1, 1), t::JUNIPER, 330.0);
        w.fill(GridIndex::new(2, 1), t::WATER, 280.0);
        w.rebaseline();
        react(&mut w);
        assert_eq!(w.material_at(GridIndex::new(1, 1)), t::JUNIPER);
    }

    #[test]
    fn mashing_conserves_mass_and_energy_exactly() {
        let mut w = world(4, 4);
        w.fill(GridIndex::new(1, 1), t::JUNIPER, 320.0);
        w.fill(GridIndex::new(2, 1), t::WATER, 320.0);
        w.rebaseline();
        let (m0, e0) = (w.total_mass(), w.total_energy());
        react(&mut w);
        assert!((w.total_mass() - m0).abs() < 1e-9, "mass moved");
        assert!(
            (w.total_energy() - e0).abs() / e0.abs() < 1e-6,
            "energy moved: {e0} -> {}",
            w.total_energy()
        );
    }

    /// Mashing is declared enthalpy-neutral, so it must not warm or chill
    /// the tun — the check that the declared figure actually reaches the
    /// runtime, rather than being decoration on a table nothing reads.
    ///
    /// "Neutral" is only exactly neutral *at the declared threshold*, and
    /// the tolerance below is why. Enthalpy is `c·T + L`, so once the
    /// participants' heat capacities differ, the enthalpy change of a
    /// material swap drifts with temperature — mashing at 320 K, ten kelvin
    /// above its 310 K threshold, absorbs a little over a kelvin's worth.
    /// That is Kirchhoff's law falling out of the representation rather
    /// than a bug, and the honest thing is to allow for it rather than
    /// flatten it.
    #[test]
    fn an_enthalpy_neutral_reaction_leaves_the_temperature_alone() {
        let mut w = world(4, 4);
        w.fill(GridIndex::new(1, 1), t::JUNIPER, 320.0);
        w.fill(GridIndex::new(2, 1), t::WATER, 320.0);
        w.rebaseline();
        react(&mut w);
        let after = w.cell(GridIndex::new(2, 1)).temperature;
        assert!(
            (after - 320.0).abs() < 2.0,
            "mashing shifted the tun to {after}"
        );
    }

    /// ...and at the threshold itself it is neutral to the last bit the
    /// arithmetic allows, which is the claim the declared `0.0` actually
    /// makes.
    #[test]
    fn an_enthalpy_neutral_reaction_is_exactly_neutral_at_its_threshold() {
        let mut w = world(4, 4);
        w.fill(GridIndex::new(1, 1), t::JUNIPER, 310.001);
        w.fill(GridIndex::new(2, 1), t::WATER, 310.001);
        w.rebaseline();
        react(&mut w);
        let after = w.cell(GridIndex::new(2, 1)).temperature;
        assert!(
            (after - 310.0).abs() < 0.05,
            "mashing at its own threshold shifted the tun to {after}"
        );
    }

    #[test]
    fn burning_juniper_releases_heat_and_leaves_charcoal() {
        let mut w = world(4, 4);
        w.fill(GridIndex::new(1, 1), t::JUNIPER, 700.0);
        w.rebaseline();
        let e0 = w.total_energy();
        react(&mut w);
        assert_eq!(w.material_at(GridIndex::new(1, 1)), t::CHARCOAL);
        // The air it burned in became CO2 — one of the four neighbours.
        let co2 = w.count_of(t::CO2);
        assert_eq!(co2, 1, "expected exactly one cell of exhaust, got {co2}");
        let flame = w.cell(GridIndex::new(1, 1)).temperature;
        assert!(
            flame > 900.0 && flame < 2500.0,
            "flame front at {flame} K, expected roughly 1300"
        );
        assert!(
            (w.total_energy() - e0).abs() / e0.abs() < 1e-6,
            "combustion is not free: {e0} -> {}",
            w.total_energy()
        );
    }

    /// A cell reacts at most once per step, so a bush surrounded by water
    /// mashes exactly one of them and no more.
    #[test]
    fn one_bush_mashes_exactly_one_cell_of_water_per_step() {
        let mut w = world(5, 5);
        w.fill(GridIndex::new(2, 2), t::JUNIPER, 320.0);
        for n in [(1, 2), (3, 2), (2, 1), (2, 3)] {
            w.fill(GridIndex::new(n.0, n.1), t::WATER, 320.0);
        }
        w.rebaseline();
        react(&mut w);
        assert_eq!(w.count_of(t::WASH), 2, "{}", crate::report::ascii_map(&w));
    }

    /// End to end, in one pot: water and juniper held above mashing
    /// temperature turn into wash, and wash held above ethanol's boiling
    /// point (but below water's) boils off as spirit while the plain water
    /// beside it stays liquid. That separation is the entire point of a
    /// still, and nothing in the code names any of these materials.
    #[test]
    fn a_warm_tun_ferments_and_then_distils_while_the_water_stays_put() {
        let mut w = World::new(6, 8, MaterialTable::terrarium(), t::AIR, 360.0);
        for i in 0..6 {
            w.fill(GridIndex::new(i, 0), t::STONE, 365.0);
        }
        for i in 1..5 {
            w.fill(GridIndex::new(i, 1), t::WATER, 360.0);
        }
        w.fill(GridIndex::new(2, 1), t::JUNIPER, 360.0);
        w.rebaseline();

        // A still needs a fire under it, and this is why: boiling off a
        // gram of spirit costs 846 J, which is more than a six-by-eight
        // box of stone and water holds above the boiling point. Without a
        // declared, ledgered heat source the pot ferments and then simply
        // stalls at 351.5 K, half-way through a phase change it cannot
        // afford. Held at 365 K, it distils.
        let hob: Vec<crate::terrarium::Thermostat> = (0..6)
            .map(|i| crate::terrarium::Thermostat::new(GridIndex::new(i, 0), 365.0))
            .collect();

        let mut ever_wash = false;
        for _ in 0..900 {
            for th in &hob {
                th.apply(&mut w);
            }
            physics::step(&mut w, 0.05);
            ever_wash |= w.count_of(t::WASH) > 0;
        }
        assert!(ever_wash, "the warm tun never fermented");
        assert!(
            w.count_of(t::SPIRIT) > 0 || w.count_of(t::GIN) > 0,
            "the wash never boiled off:\n{}",
            crate::report::ascii_map(&w)
        );
        assert_eq!(w.count_of(t::STEAM), 0, "the water boiled too — too hot");
        let r = w.conservation_residuals();
        assert!(r.mass_relative.abs() < 1e-6, "mass {:e}", r.mass_relative);
        assert!(
            r.energy_relative.abs() < 1e-5,
            "energy {:e}",
            r.energy_relative
        );
    }
}
