//! Headless runner for the gas chamber.
//!
//! Prints the chamber's JSON snapshot every `--every` steps — pressures,
//! how high the CO₂ sits, and the CO₂ fraction row by row — so the claim
//! the chamber exists to make ("heavier than air, but it mixes") can be
//! checked from a shell without a browser.
//!
//! ```sh
//! cargo run --release --bin chamber -- --steps 4000 --every 500 --map
//! ```
//!
//! `--temps` adds a temperature map (see `report::temperature_map`).

use viewer::{chamber, report};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let flag = |name: &str, default: u64| -> u64 {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    };
    let steps = flag("--steps", 4000);
    let every = flag("--every", 500).max(1);
    let show_map = args.iter().any(|a| a == "--map");
    let show_temps = args.iter().any(|a| a == "--temps");

    let mut c = chamber::default_chamber();
    println!("{}", report::chamber_json(&c.world, 0));
    for step in 1..=steps {
        c.step(viewer::SIM_DT);
        if step % every == 0 {
            println!("{}", report::chamber_json(&c.world, step));
            if show_map {
                eprint!("{}", report::ascii_map(&c.world));
            }
            if show_temps {
                eprint!("{}", report::temperature_map(&c.world));
            }
        }
    }
}
