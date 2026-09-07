//! Headless reporting: a JSON snapshot of a running world and colony.
//!
//! This repo's standing habit is to verify simulations with numbers a test
//! can assert on rather than by looking at a picture (see `JOURNAL.md`).
//! Everything worth knowing about a run — how much of each material there
//! is, how hot it is on average, what the gnomes are doing, and above all
//! what the conservation residuals are — comes out of here as JSON, so a
//! run can be checked from a shell pipeline without a browser.
//!
//! Written by hand rather than with `serde`: the crate has one dependency
//! today and the output is a dozen fixed fields.

use crate::gnome::Colony;
use crate::material::terrarium;
use crate::world::World;

/// A JSON object describing the world and colony as they stand.
///
/// Shape (field order is stable; readers may rely on it):
///
/// ```text
/// {"step":N,"mean_temperature_k":F,"total_mass_g":F,"total_energy_j":F,
///  "residual_mass_g":F,"residual_energy_j":F,
///  "residual_mass_relative":F,"residual_energy_relative":F,
///  "ledger":{"mass_conjured_g":F,"energy_conjured_j":F,"gin_spent":F},
///  "materials":[{"name":S,"cells":N,"mass_g":F},...],
///  "colony":{"gnomes":N,"embodied":N,"ethereal":N,"total_gin":F}}
/// ```
pub fn snapshot_json(world: &World, colony: &Colony, step: u64) -> String {
    let r = world.conservation_residuals();
    let ledger = world.ledger();

    let materials: Vec<String> = terrarium::ALL
        .iter()
        .map(|&id| {
            format!(
                "{{\"name\":\"{}\",\"cells\":{},\"mass_g\":{:.6}}}",
                terrarium::name(id),
                world.count_of(id),
                world.mass_of(id)
            )
        })
        .collect();

    format!(
        "{{\"step\":{step},\"mean_temperature_k\":{:.4},\"total_mass_g\":{:.6},\
         \"total_energy_j\":{:.4},\"residual_mass_g\":{:.9},\"residual_energy_j\":{:.6},\
         \"residual_mass_relative\":{:e},\"residual_energy_relative\":{:e},\
         \"ledger\":{{\"mass_conjured_g\":{:.6},\"energy_conjured_j\":{:.4},\"gin_spent\":{:.4}}},\
         \"materials\":[{}],\
         \"colony\":{{\"gnomes\":{},\"embodied\":{},\"ethereal\":{},\"total_gin\":{:.3}}}}}",
        world.mean_temperature(),
        world.total_mass(),
        world.total_energy(),
        r.mass,
        r.energy,
        r.mass_relative,
        r.energy_relative,
        ledger.mass_conjured,
        ledger.energy_conjured,
        ledger.gin_spent,
        materials.join(","),
        colony.gnomes.len(),
        colony.embodied_count(),
        colony.ethereal_count(),
        colony.total_gin(),
    )
}

/// The gas chamber's JSON snapshot — the same conservation fields
/// [`snapshot_json`] reports, minus the colony (the chamber has no gnomes
/// in it) and plus the two numbers that scenario is actually about: the
/// spread of pressures across the air, and how high the CO₂ is sitting.
///
/// Shape:
///
/// ```text
/// {"step":N,"mean_temperature_k":F,"total_mass_g":F,
///  "residual_mass_relative":F,"residual_energy_relative":F,
///  "ledger":{"mass_conjured_g":F,"energy_conjured_j":F},
///  "air_pressure_min":F,"air_pressure_max":F,
///  "co2_mean_height":F,"co2_cells":N,
///  "materials":[{"name":S,"cells":N,"mass_g":F},...]}
/// ```
///
/// `co2_mean_height` is `-1` when the world holds no CO₂ at all, so the
/// field is always a number and a reader never has to handle `null`.
pub fn chamber_json(world: &World, step: u64) -> String {
    let r = world.conservation_residuals();
    let ledger = world.ledger();
    let (lo, hi) = crate::gas::pressure_range_of(world, terrarium::AIR).unwrap_or((0.0, 0.0));
    let co2_height = crate::gas::mean_height_of(world, terrarium::CO2).unwrap_or(-1.0);

    let materials: Vec<String> = terrarium::ALL
        .iter()
        .map(|&id| {
            format!(
                "{{\"name\":\"{}\",\"cells\":{},\"mass_g\":{:.6}}}",
                terrarium::name(id),
                world.count_of(id),
                world.mass_of(id)
            )
        })
        .collect();

    format!(
        "{{\"step\":{step},\"mean_temperature_k\":{:.4},\"total_mass_g\":{:.6},\
         \"residual_mass_relative\":{:e},\"residual_energy_relative\":{:e},\
         \"ledger\":{{\"mass_conjured_g\":{:.6},\"energy_conjured_j\":{:.4}}},\
         \"air_pressure_min\":{:.6},\"air_pressure_max\":{:.6},\
         \"co2_mean_height\":{:.4},\"co2_cells\":{},\
         \"materials\":[{}]}}",
        world.mean_temperature(),
        world.total_mass(),
        r.mass_relative,
        r.energy_relative,
        ledger.mass_conjured,
        ledger.energy_conjured,
        lo,
        hi,
        co2_height,
        world.count_of(terrarium::CO2),
        materials.join(","),
    )
}

/// An ASCII picture of the world, one character per cell, top row first —
/// for eyeballing a headless run in a terminal without a canvas.
pub fn ascii_map(world: &World) -> String {
    let mut out = String::new();
    for j in (0..world.height() as i32).rev() {
        for i in 0..world.width() as i32 {
            let id = world.material_at(crate::math::GridIndex::new(i, j));
            out.push(match terrarium::name(id) {
                "air" => ' ',
                "steam" => '~',
                "water" => 'w',
                "ice" => '*',
                "sand" => '.',
                "stone" => '#',
                "lava" => '@',
                "juniper" => 'Y',
                "co2" => 'c',
                _ => '?',
            });
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrarium::default_terrarium;

    #[test]
    fn a_snapshot_reports_every_material_and_the_residuals() {
        let terra = default_terrarium();
        let json = snapshot_json(&terra.world, &terra.colony, 0);
        for name in [
            "air", "steam", "water", "ice", "sand", "stone", "lava", "juniper",
        ] {
            assert!(
                json.contains(&format!("\"name\":\"{name}\"")),
                "missing {name}"
            );
        }
        assert!(json.contains("\"residual_mass_g\":0.000000000"));
        assert!(json.contains("\"colony\":{\"gnomes\":4,\"embodied\":4,\"ethereal\":0"));
    }

    #[test]
    fn a_chamber_snapshot_reports_pressure_and_the_co2_layer() {
        let mut c = crate::chamber::default_chamber();
        for _ in 0..200 {
            c.step(0.05);
        }
        let json = chamber_json(&c.world, c.steps);
        for field in [
            "\"air_pressure_min\"",
            "\"air_pressure_max\"",
            "\"co2_mean_height\"",
            "\"co2_cells\"",
            "\"residual_mass_relative\"",
        ] {
            assert!(json.contains(field), "missing {field} in {json}");
        }
    }

    #[test]
    fn the_ascii_map_has_one_character_per_cell() {
        let terra = default_terrarium();
        let map = ascii_map(&terra.world);
        let lines: Vec<&str> = map.lines().collect();
        assert_eq!(lines.len(), terra.world.height());
        assert!(lines
            .iter()
            .all(|l| l.chars().count() == terra.world.width()));
    }
}
