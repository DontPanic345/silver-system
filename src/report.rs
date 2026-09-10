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
///  "vapour":{"evaporated_g":F,"condensed_g":F,"rained_g":F},
///  "materials":[{"name":S,"cells":N,"mass_g":F},...],
///  "colony":{"gnomes":N,"embodied":N,"ethereal":N,"total_gin":F}}
/// ```
pub fn snapshot_json(world: &World, colony: &Colony, step: u64) -> String {
    let r = world.conservation_residuals();
    let ledger = world.ledger();

    let tally = world.tally();

    format!(
        "{{\"step\":{step},\"mean_temperature_k\":{:.4},\"total_mass_g\":{:.6},\
         \"total_energy_j\":{:.4},\"residual_mass_g\":{:.9},\"residual_energy_j\":{:.6},\
         \"residual_mass_relative\":{:e},\"residual_energy_relative\":{:e},\
         \"ledger\":{{\"mass_conjured_g\":{:.6},\"energy_conjured_j\":{:.4},\"gin_spent\":{:.4}}},\
         \"vapour\":{{\"evaporated_g\":{:.6},\"condensed_g\":{:.6},\"rained_g\":{:.6}}},\
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
        tally.evaporated_g,
        tally.condensed_g,
        tally.rained_g,
        materials_json(world),
        colony.gnomes.len(),
        colony.embodied_count(),
        colony.ethereal_count(),
        colony.total_gin(),
    )
}

/// The gas chamber's JSON snapshot — the same conservation fields
/// [`snapshot_json`] reports, minus the colony (the chamber has no gnomes
/// in it) and plus the numbers that scenario is actually about: the spread
/// of pressures across the room, how high the CO₂ is sitting, and the
/// CO₂ fraction row by row.
///
/// Shape:
///
/// ```text
/// {"step":N,"mean_temperature_k":F,"total_mass_g":F,
///  "residual_mass_relative":F,"residual_energy_relative":F,
///  "ledger":{"mass_conjured_g":F,"energy_conjured_j":F},
///  "pressure_min":F,"pressure_max":F,
///  "co2_mean_height":F,"co2_cells":N,"co2_mass_g":F,
///  "co2_profile":[F,...],
///  "materials":[{"name":S,"cells":N,"mass_g":F},...]}
/// ```
///
/// The pressures are over *every* gas cell. They used to be over air alone,
/// because two gases could not share a cell and so could not share a
/// pressure; now they mix, and the room has one pressure.
///
/// `co2_profile` is the CO₂ mole fraction of each row's gas, bottom row
/// first (`-1` for a row with no gas in it) — the number the dictation's
/// claim is about: a layer is a profile that is 1 at the floor and 0 above
/// it, a mixed room one that is the same all the way up.
///
/// `co2_mean_height` is `-1` when the world holds no CO₂ at all, so the
/// field is always a number and a reader never has to handle `null`.
pub fn chamber_json(world: &World, step: u64) -> String {
    let r = world.conservation_residuals();
    let ledger = world.ledger();
    let (lo, hi) = crate::gas::pressure_range(world).unwrap_or((0.0, 0.0));
    let co2_height = crate::gas::mean_height_of(world, terrarium::CO2).unwrap_or(-1.0);
    let profile: Vec<String> = crate::gas::fraction_profile(world, terrarium::CO2)
        .iter()
        .map(|f| format!("{:.5}", f.unwrap_or(-1.0)))
        .collect();

    format!(
        "{{\"step\":{step},\"mean_temperature_k\":{:.4},\"total_mass_g\":{:.6},\
         \"residual_mass_relative\":{:e},\"residual_energy_relative\":{:e},\
         \"ledger\":{{\"mass_conjured_g\":{:.6},\"energy_conjured_j\":{:.4}}},\
         \"pressure_min\":{:.6},\"pressure_max\":{:.6},\
         \"co2_mean_height\":{:.4},\"co2_cells\":{},\"co2_mass_g\":{:.6},\
         \"co2_profile\":[{}],\
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
        world.mass_of(terrarium::CO2),
        profile.join(","),
        materials_json(world),
    )
}

/// The per-material list both snapshots carry: how many cells each
/// material labels, and how many grams of it the world holds anywhere —
/// whole cells, its share of every gas mixture, and mist.
fn materials_json(world: &World) -> String {
    terrarium::ALL
        .iter()
        .map(|&id| {
            format!(
                "{{\"name\":\"{}\",\"cells\":{},\"mass_g\":{:.6}}}",
                terrarium::name(id),
                world.count_of(id),
                world.mass_of(id)
            )
        })
        .collect::<Vec<_>>()
        .join(",")
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
                "wash" => 'm',
                "spirit" => 'v',
                "gin" => 'g',
                "charcoal" => 'x',
                _ => '?',
            });
        }
        out.push('\n');
    }
    out
}

/// A temperature picture of the world, one character per cell, top row
/// first: `0`–`9` then `A`–`F` for each ten kelvin from 250 K up to 410 K,
/// `-` below that range and `+` above it. So `4` is 290–300 K (a
/// comfortable room), `C` is 370–380 K (water's boiling point), and a map
/// that is all one character is a world at equilibrium.
///
/// The material map says what is where; this says why it is about to
/// move. Both are cheap enough to print every snapshot.
pub fn temperature_map(world: &World) -> String {
    const RAMP: &[u8] = b"0123456789ABCDEF";
    let mut out = String::new();
    for j in (0..world.height() as i32).rev() {
        for i in 0..world.width() as i32 {
            let t = world.cell(crate::math::GridIndex::new(i, j)).temperature;
            let bucket = ((t - 250.0) / 10.0).floor();
            out.push(if bucket < 0.0 {
                '-'
            } else if bucket >= RAMP.len() as f64 {
                '+'
            } else {
                RAMP[bucket as usize] as char
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
            "\"pressure_min\"",
            "\"pressure_max\"",
            "\"co2_profile\"",
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
