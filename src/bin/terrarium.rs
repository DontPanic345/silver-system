//! Headless runner for the gnome terrarium.
//!
//! Prints a JSON snapshot every `--every` steps, and optionally an ASCII
//! map, so a full run can be checked from a shell without a browser — this
//! repo's standing preference for verifying a simulation by numbers.
//!
//! ```sh
//! cargo run --release --bin terrarium -- --steps 4000 --every 500 --map
//! ```
//!
//! `--temps` adds a temperature map beside it (see
//! `report::temperature_map` for the key).
//!
//! `--trace` prints one line per step per gnome to stderr: where it is,
//! what it did, how much of the world it can currently get to, and what is
//! in its belly. That last pair is what "why is the colony doing that?"
//! actually reduces to — night 8 found a colony living in a two-cell pocket
//! of the garden by reading exactly these numbers, and no summary anywhere
//! else in this crate would have shown it. Use it with `--steps` small, or
//! pipe it somewhere.
//!
//! Player orders can be written on the jar from here too, so the glass pane
//! (`src/order.rs`) is checkable without a browser. Each flag may be
//! repeated, and takes a cell as `i,j`:
//!
//! ```sh
//! cargo run --release --bin terrarium -- --steps 1200 --every 300 --map \
//!     --dig 10,3 --build 12,5 --warm 20,8
//! ```

use viewer::math::GridIndex;
use viewer::order::Job;
use viewer::path::{Routes, Through};
use viewer::{gnome, report, terrarium};

/// One line per gnome: position, what it just did, how many cells it can
/// reach, and its belly — see `--trace`.
fn trace(terra: &terrarium::Terrarium, routes: &mut Routes, step: u64) {
    let who: Vec<String> = terra
        .colony
        .gnomes
        .iter()
        .map(|g| {
            routes.explore_avoiding(&terra.world, g.pos, Through::Open, |c| {
                gnome::dangerous(&terra.world, c)
            });
            format!(
                "({},{}) {:?} reach={} gin={:.0} belly={:.3}",
                g.pos.i,
                g.pos.j,
                g.last_act,
                routes.reach(),
                g.gin,
                g.belly
            )
        })
        .collect();
    eprintln!("{step}: {}", who.join(" | "));
}

/// Every `i,j` given after each occurrence of `flag`.
fn cells(args: &[String], flag: &str) -> Vec<GridIndex> {
    args.iter()
        .enumerate()
        .filter(|(_, a)| a.as_str() == flag)
        .filter_map(|(i, _)| args.get(i + 1))
        .filter_map(|v| {
            let (a, b) = v.split_once(',')?;
            Some(GridIndex::new(
                a.trim().parse().ok()?,
                b.trim().parse().ok()?,
            ))
        })
        .collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let flag = |name: &str, default: u64| -> u64 {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    };
    let steps = flag("--steps", 3000);
    let every = flag("--every", 500).max(1);
    let show_map = args.iter().any(|a| a == "--map");
    let show_trace = args.iter().any(|a| a == "--trace");
    let mut routes = Routes::default();
    let show_temps = args.iter().any(|a| a == "--temps");

    let mut terra = terrarium::default_terrarium();
    for at in cells(&args, "--dig") {
        terra.colony.order(at, Job::Dig);
    }
    for at in cells(&args, "--build") {
        terra.colony.order(at, Job::Build);
    }
    for at in cells(&args, "--warm") {
        terra.colony.order(
            at,
            Job::Temper {
                target_k: gnome::COMFORT_MAX,
            },
        );
    }
    for at in cells(&args, "--chill") {
        terra.colony.order(
            at,
            Job::Temper {
                target_k: gnome::COMFORT_MIN,
            },
        );
    }
    println!("{}", report::snapshot_json(&terra.world, &terra.colony, 0));
    for step in 1..=steps {
        terra.step(viewer::SIM_DT);
        if show_trace {
            trace(&terra, &mut routes, step);
        }
        if step % every == 0 {
            println!(
                "{}",
                report::snapshot_json(&terra.world, &terra.colony, step)
            );
            if show_map {
                eprint!("{}", report::ascii_map(&terra.world));
            }
            if show_temps {
                eprint!("{}", report::temperature_map(&terra.world));
            }
        }
    }
}
