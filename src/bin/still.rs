//! Headless runner for the still.
//!
//! Prints a JSON snapshot every `--every` steps, and optionally an ASCII
//! map, so a whole brewing run can be checked from a shell without a
//! browser — this repo's standing preference for verifying a simulation by
//! numbers.
//!
//! ```sh
//! cargo run --release --bin still -- --steps 4000 --every 500 --map
//! ```

use viewer::{report, still};

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

    let mut s = still::default_still();
    println!("{}", report::snapshot_json(&s.world, &s.colony, 0));
    for step in 1..=steps {
        s.step(0.05);
        if step % every == 0 {
            println!("{}", report::snapshot_json(&s.world, &s.colony, step));
            if show_map {
                eprint!("{}", report::ascii_map(&s.world));
            }
        }
    }
}
