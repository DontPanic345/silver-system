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
///  "life":{"grown_g":F,"respired_g":F,"co2_taken_g":F,"oxygen_made_g":F},
///  "air":{"oxygen_g":F,"co2_g":F,"min_breathable_atm":F,"daylight":F},
///  "materials":[{"name":S,"cells":N,"mass_g":F},...],
///  "colony":{"gnomes":N,"embodied":N,"ethereal":N,"total_gin":F,
///            "carried_g":F,"respired_g":F,"min_breath":N,
///            "who":[{"i":N,"j":N,"gin":F,"belly_g":F,"breath":N,
///                    "embodied":B,"act":S},...]},
///  "orders":{"open":N,"unreachable":N,"completed":N,"cancelled":N}}
/// ```
///
/// `carried_g` and the `orders` block are the player's half of the glass
/// pane (see `src/order.rs`): what the colony has been asked to do, how much
/// of it is done, and how many grams are in its hands rather than in the
/// world. That last one is the number that makes a half-finished wall
/// legible — it is exactly the ledger entry the digging opened.
pub fn snapshot_json(world: &World, colony: &Colony, step: u64) -> String {
    let r = world.conservation_residuals();
    let ledger = world.ledger();

    let tally = world.tally();
    let life = world.life_tally();
    let atmosphere = breathable_range(world);

    format!(
        "{{\"step\":{step},\"mean_temperature_k\":{:.4},\"total_mass_g\":{:.6},\
         \"total_energy_j\":{:.4},\"residual_mass_g\":{:.9},\"residual_energy_j\":{:.6},\
         \"residual_mass_relative\":{:e},\"residual_energy_relative\":{:e},\
         \"ledger\":{{\"mass_conjured_g\":{:.6},\"energy_conjured_j\":{:.4},\"gin_spent\":{:.4}}},\
         \"vapour\":{{\"evaporated_g\":{:.6},\"condensed_g\":{:.6},\"rained_g\":{:.6}}},\
         \"life\":{{\"grown_g\":{:.8},\"respired_g\":{:.8},\"co2_taken_g\":{:.8},\
         \"oxygen_made_g\":{:.8}}},\
         \"air\":{{\"oxygen_g\":{:.6},\"co2_g\":{:.6},\"min_breathable_atm\":{:.5},\
         \"daylight\":{:.4}}},\
         \"materials\":[{}],\
         \"colony\":{{\"gnomes\":{},\"embodied\":{},\"ethereal\":{},\"total_gin\":{:.3},\
         \"carried_g\":{:.6},\"respired_g\":{:.6},\"min_breath\":{},\"who\":[{}]}},\
         \"orders\":{{\"open\":{},\"unreachable\":{},\"completed\":{},\"cancelled\":{}}}}}",
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
        life.grown_g,
        life.respired_g,
        // The two directions of the carbon cycle, as grams rather than as a
        // proxy: what living things have taken out of the air, and what they
        // have put back as oxygen. Net of everything — a garden growing, a
        // bush respiring in the dark, a compost heap rotting.
        -world.life_moved(terrarium::CO2),
        world.life_moved(terrarium::OXYGEN),
        world.mass_of(terrarium::OXYGEN),
        world.mass_of(terrarium::CO2),
        atmosphere,
        world.sky,
        materials_json(world),
        colony.gnomes.len(),
        colony.embodied_count(),
        colony.ethereal_count(),
        colony.total_gin(),
        colony.carried_g(),
        colony.respired_g(),
        // Steps of held breath left in the worst-off gnome — the direct
        // answer to "is anybody suffocating", which `min_breathable_atm`
        // above is not. That one is the thinnest air *in the jar*, and a
        // dense garden grows sealed pockets inside its own canopy whose
        // oxygen the plants in them breathe down to nothing overnight, so it
        // reads 0.00 atm from about step 20000 of a long run while every
        // gnome is breathing perfectly well somewhere else. Full is
        // `gnome::BREATH_STEPS`; anything less means somebody is holding it.
        colony.min_breath(),
        gnomes_json(colony),
        colony.orders.len(),
        // Orders nobody could walk to when the colony last looked — the
        // headless half of the dashed marker the renderer draws. A queue
        // that is not going down reads very differently once you can see
        // whether anyone can get there.
        colony.orders.iter().filter(|o| !o.reachable).count(),
        colony.orders.completed(),
        colony.orders.cancelled(),
    )
}

/// One cell, as JSON — the inspector behind the glass pane.
///
/// The browser view calls this on hover, so a human can point at anything in
/// the jar and be told what it actually is: what it is made of, how hot, how
/// heavy, what pressure it is under, what gases are mixed into it, how much
/// light reaches it, and whether the player has written anything on it. Every
/// number here is read straight off the simulation, not out of a second
/// summary kept beside it.
///
/// ```text
/// {"i":N,"j":N,"material":S,"temperature_k":F,"mass_g":F,"pressure_atm":F,
///  "light":F,"order":S|null,"mix":[{"name":S,"grams":F},...]}
/// ```
pub fn cell_json(world: &World, colony: &Colony, at: crate::math::GridIndex) -> String {
    if !world.in_bounds(at) {
        return "{}".to_string();
    }
    let cell = world.cell(at);
    let p = world.linear_index(at);
    let mix: Vec<String> = terrarium::ALL
        .iter()
        .filter_map(|&id| {
            let grams = cell.grams_of(world.materials(), id);
            (grams > 1e-9).then(|| {
                format!(
                    "{{\"name\":\"{}\",\"grams\":{:.6}}}",
                    terrarium::name(id),
                    grams
                )
            })
        })
        .collect();
    let order = match colony.orders.at(at).map(|o| o.job) {
        Some(crate::order::Job::Dig) => "\"dig\"".to_string(),
        Some(crate::order::Job::Build) => "\"build\"".to_string(),
        Some(crate::order::Job::Supply { material }) => {
            format!("\"supply:{}\"", terrarium::name(material))
        }
        Some(crate::order::Job::Temper { target_k }) => format!("\"temper:{target_k:.0}\""),
        None => "null".to_string(),
    };
    // Who is standing here. A gnome is drawn over the cell it is in, so
    // anything reading this cell's pixels — the inspector under a person's
    // mouse, a browser test checking that a dug cell now looks like a hole —
    // needs to be able to tell "empty" from "empty, with somebody in it".
    let gnomes = colony
        .gnomes
        .iter()
        .filter(|g| g.is_embodied() && g.pos == at)
        .count();
    format!(
        "{{\"i\":{},\"j\":{},\"material\":\"{}\",\"temperature_k\":{:.2},\"mass_g\":{:.5},\
         \"pressure_atm\":{:.4},\"light\":{:.3},\"order\":{},\"gnomes\":{},\"mix\":[{}]}}",
        at.i,
        at.j,
        terrarium::name(cell.material),
        cell.temperature,
        cell.mass,
        cell.pressure(world.materials()) / world.materials().reference_pressure(),
        world.light_at(p),
        order,
        gnomes,
        mix.join(",")
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

/// The lowest breathable partial pressure in any gas cell in the world, in
/// atmospheres.
///
/// Over the whole world, so a pocket of foul air shows up even when the
/// room's average is fine — which is what it is for, and also why it is not
/// the number to ask "is the colony in trouble" (see `min_breath` in
/// [`snapshot_json`]). A garden dense enough to seal a cell inside its own
/// canopy takes that cell to nothing overnight and leaves it there, and this
/// reads zero for the rest of the run while every gnome breathes freely.
fn breathable_range(world: &World) -> f64 {
    let reference = world.materials().reference_pressure();
    if reference <= 0.0 {
        return 0.0;
    }
    (0..world.width() * world.height())
        .filter(|&p| world.is_gas_at(p) && world.cell_at(p).mass > 0.0)
        .map(|p| world.cell_at(p).breathable_pressure(world.materials()) / reference)
        .fold(f64::INFINITY, f64::min)
        .min(9.9)
}

/// Each gnome, one object apiece: where it is, what it is holding in flask
/// and belly, and what it did on the step just simulated.
///
/// A colony total says whether anybody is in trouble; this says which of
/// them and where, which is the difference between noticing a long run has
/// gone wrong and being able to say why. The first thing it was ever used
/// for was finding which gnome was standing in a puddle.
fn gnomes_json(colony: &Colony) -> String {
    colony
        .gnomes
        .iter()
        .map(|g| {
            format!(
                "{{\"i\":{},\"j\":{},\"gin\":{:.2},\"belly_g\":{:.5},\"breath\":{},\
                 \"embodied\":{},\"act\":\"{:?}\"}}",
                g.pos.i,
                g.pos.j,
                g.gin,
                g.belly,
                g.breath,
                g.is_embodied(),
                g.last_act
            )
        })
        .collect::<Vec<_>>()
        .join(",")
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
                "oxygen" => 'o',
                "glass" => '=',
                "litter" => ',',
                "fungus" => 'f',
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
