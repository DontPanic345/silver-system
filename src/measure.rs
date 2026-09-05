//! Headless measurement: run a [`Scenario`] for a fixed number of steps with
//! no browser, no DOM, no human reading a screenshot, and get back numbers a
//! test can assert on directly.
//!
//! This is the first of `Scenario`'s two consumers named in its module doc
//! comment — a headless runner (`src/render.rs`'s renderer is the other).
//! The whole point is that nothing here requires a human in the loop: build
//! a scenario, run it, measure it, assert on the result, all inside one
//! `cargo test --lib`-reachable test.
//!
//! ## JSON: hand-rolled, not `serde`
//!
//! The measurement shape is small (a tick count plus one small record per
//! material) and known in full at the point it is written — a
//! `serde`/`serde_json` dependency would pull in derive machinery and a
//! general-purpose parser/writer for a handful of fields this module already
//! knows the exact layout of. Hand-rolling keeps the output format under
//! this module's direct control (an "assert exact values" requirement — the
//! string is exactly what [`Measurement::to_json`] says it is, not whatever
//! a derive macro's field-ordering happens to produce) and adds no
//! dependency, so there is nothing new to check against the
//! `wasm32-unknown-unknown` build. If the measurement shape later grows
//! enough that hand-rolling becomes the wrong trade (nested/optional
//! fields, real escaping needs), that is worth revisiting then.
//!
//! ## Units: consistent with, not resolving, an open flag
//!
//! `total_mass` here is `cell_count * Material::density`, and `density` is
//! still an unpinned unit, flagged in `src/material.rs`. This module does
//! not invent a unit system — it sums whatever `density` values exist,
//! consistently, and leaves the unit question open for whichever later code
//! first needs `total_mass` to mean something in a real physical unit.

use crate::grid::Grid;
use crate::material::{MaterialId, MaterialTable};
use crate::math::Scalar;
use crate::scenario::Scenario;
use crate::timestep::FixedTimestep;

/// One material's measured state at the moment [`Measurement`] was taken:
/// how many cells hold it, and their combined mass.
///
/// Plain data, in a fixed field order — [`Measurement::to_json`] writes
/// these fields in exactly the order declared here, so this order *is* the
/// JSON shape later readers rely on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialMeasurement {
    /// The raw [`MaterialId`] value this record describes.
    pub material_id: u16,
    /// How many grid cells currently hold this material.
    pub cell_count: usize,
    /// `cell_count as Scalar * that material's Material::density` — see the
    /// module doc comment's unit caveat.
    pub total_mass: Scalar,
}

/// A headless run's result: how many ticks were actually reached, and one
/// [`MaterialMeasurement`] per material in the scenario's table, ordered by
/// ascending [`MaterialId`].
///
/// Holds "total mass per material..., cell counts per material, and the
/// tick count reached" — plus [`Measurement::to_json`] to serialize it as
/// JSON.
#[derive(Debug, Clone, PartialEq)]
pub struct Measurement {
    /// The number of fixed steps [`run_headless`] actually applied — not
    /// merely the `num_steps` it was asked for, though for a `dt` that
    /// exactly matches each call's fed duration (as [`run_headless`] uses)
    /// the two coincide. Kept as the *earned* count, not the requested one,
    /// so this field means the same thing a future non-uniform-duration
    /// caller would need it to mean.
    pub ticks: u32,
    /// Per-material records, ordered by ascending `material_id` — the same
    /// order [`MaterialTable`] itself stores them in.
    pub materials: Vec<MaterialMeasurement>,
}

impl Measurement {
    /// Serializes this measurement to a JSON object string:
    /// `{"ticks":<u32>,"materials":[{"material_id":<u16>,"cell_count":<usize>,"total_mass":<f32>},...]}`.
    ///
    /// Field order and names are fixed (see each field's doc comment above)
    /// — this is the shape any later conservation tests read without
    /// guessing at field names. `total_mass` is written via `{:?}` (Rust's
    /// float `Debug`), which always includes a decimal point (`2.0`, not
    /// `2`) so a reader can tell at a glance it is a float field, not an
    /// integer one — the same reasoning applies to `cell_count`/`ticks`
    /// never carrying a decimal point, since they are genuinely integer
    /// counts.
    pub fn to_json(&self) -> String {
        let materials_json: Vec<String> = self
            .materials
            .iter()
            .map(|m| {
                format!(
                    "{{\"material_id\":{},\"cell_count\":{},\"total_mass\":{:?}}}",
                    m.material_id, m.cell_count, m.total_mass
                )
            })
            .collect();
        format!(
            "{{\"ticks\":{},\"materials\":[{}]}}",
            self.ticks,
            materials_json.join(",")
        )
    }
}

/// Reads `grid`'s current state into a [`Measurement`]: one
/// [`MaterialMeasurement`] per material `materials` knows about (by table
/// position, i.e. ascending [`MaterialId`]), counting cells and summing mass
/// with a single pass over [`Grid::cells`] — read-only use of `Grid`'s
/// existing public API.
fn measure(grid: &Grid, materials: &MaterialTable, ticks: u32) -> Measurement {
    let mut counts = vec![0usize; materials.len()];
    for &id in grid.cells() {
        counts[id.0 as usize] += 1;
    }

    let materials = counts
        .into_iter()
        .enumerate()
        .map(|(i, cell_count)| {
            let id = MaterialId::new(i as u16);
            let density = materials.get(id).density;
            MaterialMeasurement {
                material_id: i as u16,
                cell_count,
                total_mass: cell_count as Scalar * density,
            }
        })
        .collect();

    Measurement { ticks, materials }
}

/// The headless runner: builds `scenario`'s [`Grid`], steps it `num_steps`
/// times (each call feeding `dt` as the elapsed real duration — a headless
/// run has no browser clock, so it feeds a fixed `dt` per call rather than
/// any real wall-clock reading), and returns the resulting [`Measurement`].
///
/// Uses a fresh [`FixedTimestep`] built with the same `dt`, so each of the
/// `num_steps` calls carries exactly one fixed step's worth of time and
/// therefore elapses exactly one step — `Measurement::ticks` will equal
/// `num_steps` for any `dt > 0.0`. Native, no `wasm-bindgen`/browser
/// dependency: this function (and everything it calls — `Scenario`,
/// `Grid::step`, `FixedTimestep`) compiles and runs the same under
/// `cargo test --lib` as it would in any other native/headless context.
pub fn run_headless(scenario: &Scenario, num_steps: u32, dt: Scalar) -> Measurement {
    let mut grid = scenario.build_grid();
    let mut timestep = FixedTimestep::new(dt);

    let mut ticks = 0u32;
    for _ in 0..num_steps {
        ticks += grid.step(&mut timestep, dt);
    }

    measure(&grid, &scenario.materials, ticks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::stone_and_water_pool;

    // --- Scenario: a headless runner exists and is genuinely headless ---

    /// Scenario: `run_headless` runs `stone_and_water_pool()` for a small,
    /// stated number of steps and reports exactly that many ticks reached —
    /// pins that the runner is real elapsed-time-driven stepping (via
    /// `FixedTimestep`), not a bare loop that ignores timing.
    #[test]
    fn run_headless_reaches_exactly_the_requested_tick_count() {
        let scenario = stone_and_water_pool();
        let measurement = run_headless(&scenario, 3, 0.1);
        assert_eq!(measurement.ticks, 3);
    }

    // --- Scenario: exact JSON values, not just "it parsed" ---

    /// Scenario: running the named `stone_and_water_pool()` fixture for 3
    /// steps produces the exact expected mass-per-material and
    /// cell-count-per-material numbers, both as a `Measurement` value and as
    /// the JSON string `to_json` produces. The fixture is a 6x4 (24-cell)
    /// grid: a 2x2 (4-cell) stone lump, a 2-cell
    /// water pool, and the remaining 18 cells air background (see
    /// `src/scenario.rs`'s `stone_and_water_pool` doc comment). Air density
    /// is 0.0, water 1.0, stone 2.5 (`MaterialTable::reference`), so:
    /// air mass = 18 * 0.0 = 0.0, water mass = 2 * 1.0 = 2.0,
    /// stone mass = 4 * 2.5 = 10.0.
    #[test]
    fn headless_run_of_stone_and_water_pool_reports_exact_mass_and_cell_counts() {
        let scenario = stone_and_water_pool();
        let measurement = run_headless(&scenario, 3, 0.1);

        assert_eq!(
            measurement,
            Measurement {
                ticks: 3,
                materials: vec![
                    MaterialMeasurement {
                        material_id: 0,
                        cell_count: 18,
                        total_mass: 0.0,
                    },
                    MaterialMeasurement {
                        material_id: 1,
                        cell_count: 2,
                        total_mass: 2.0,
                    },
                    MaterialMeasurement {
                        material_id: 2,
                        cell_count: 4,
                        total_mass: 10.0,
                    },
                ],
            }
        );

        assert_eq!(
            measurement.to_json(),
            "{\"ticks\":3,\"materials\":[\
             {\"material_id\":0,\"cell_count\":18,\"total_mass\":0.0},\
             {\"material_id\":1,\"cell_count\":2,\"total_mass\":2.0},\
             {\"material_id\":2,\"cell_count\":4,\"total_mass\":10.0}]}"
        );
    }

    // --- Scenario: conservation smoke check ---

    /// Scenario: because the current step is identity-only, total mass and
    /// every material's cell count must be *exactly* unchanged after any
    /// number of steps — not to 0.1%-tolerance (that is later work's job,
    /// once matter actually moves), but exactly, since nothing physical
    /// happens yet. Runs `stone_and_water_pool()` for 100 steps and asserts
    /// the post-run measurement is bit-for-bit identical to the pre-run one
    /// (measured directly off the freshly built grid, step count 0),
    /// proving the measurement path itself can prove "nothing moved, so
    /// nothing changed" — the scaffolding any later real conservation
    /// targets build their own assertions on top of.
    #[test]
    fn stone_and_water_pool_mass_and_cell_counts_are_exactly_unchanged_after_100_steps() {
        let scenario = stone_and_water_pool();
        let before = run_headless(&scenario, 0, 0.1);
        let after = run_headless(&scenario, 100, 0.1);

        assert_eq!(after.ticks, 100);
        assert_eq!(
            after.materials, before.materials,
            "identity-only stepping must leave every material's mass and \
             cell count exactly unchanged, no matter how many steps elapse"
        );

        // Also pin the concrete numbers directly, so this test does not
        // silently pass if `stone_and_water_pool()`'s own layout ever
        // changes underneath both `before` and `after` together.
        let total_mass: Scalar = after.materials.iter().map(|m| m.total_mass).sum();
        let total_cells: usize = after.materials.iter().map(|m| m.cell_count).sum();
        assert_eq!(total_mass, 12.0, "18*0.0 + 2*1.0 + 4*2.5 == 12.0");
        assert_eq!(
            total_cells, 24,
            "total cells must always equal the fixture's 6*4 grid size"
        );
    }

    // --- Scenario: no human in the loop ---
    //
    // Every test above is itself proof of this: build scenario, run,
    // measure, assert — reachable by `cargo test --lib` alone, no manual
    // step, no screenshot, nothing printed for a person to read and judge.
    // Named here explicitly so the property is visible in the test list,
    // not just implied by the other tests' shape.

    /// Scenario: the entire path (`Scenario` -> `run_headless` ->
    /// `Measurement` -> `to_json`) runs to completion and produces a
    /// non-empty, self-describing JSON string with no I/O, no printed
    /// output, and no assertion left for a human to eyeball — this test's
    /// own pass/fail *is* the judgement, not something a person reads.
    #[test]
    fn full_headless_path_runs_and_asserts_with_no_human_in_the_loop() {
        let scenario = stone_and_water_pool();
        let measurement = run_headless(&scenario, 1, 0.1);
        let json = measurement.to_json();

        assert!(json.starts_with("{\"ticks\":1,\"materials\":["));
        assert!(json.ends_with("]}"));
        assert_eq!(measurement.materials.len(), scenario.materials.len());
    }
}
