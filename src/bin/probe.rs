//! Temporary diagnostic.
use viewer::math::GridIndex;
use viewer::order::Job;
use viewer::path::{self, Routes, Through};
use viewer::{report, terrarium};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let flag = |name: &str, default: u64| -> u64 {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    };
    let warm = flag("--warm", 0);
    let watch = flag("--watch", 40);
    let mut terra = terrarium::default_terrarium();
    terra.colony.order(GridIndex::new(2, 4), Job::Dig);
    for _ in 0..warm {
        terra.step(viewer::SIM_DT);
    }
    let mut routes = Routes::default();
    for s in 0..watch {
        terra.step(viewer::SIM_DT);
        let line: Vec<String> = terra
            .colony
            .gnomes
            .iter()
            .map(|g| {
                routes.explore_avoiding(&terra.world, g.pos, Through::Open, |c| {
                    viewer::gnome::dangerous(&terra.world, c)
                });
                format!(
                    "({},{}) {:?} reach={} b={:.3}",
                    g.pos.i,
                    g.pos.j,
                    g.last_act,
                    routes.reach(),
                    g.belly
                )
            })
            .collect();
        println!("{}: {}", warm + s, line.join(" | "));
    }
    eprint!("{}", report::ascii_map(&terra.world));
    let _ = path::passable;
}
