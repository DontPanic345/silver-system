# Round 3 — Headless empirical measurement

**Milestone:** M1.1 — Substrate and harness (Tranche 1 — Physics)

**Shape:** single-pass. Reasoning: this round builds a measurement/reporting
path on top of round 1/2's already-tested `Grid`/`Scenario`, not a new
shared primitive itself. It touches the *concept* of mass conservation
(a tranche target), but not the target itself — M1.1's own step function is
identity-only (round 2), so mass conservation is trivially exact here; the
real conservation risk arrives with real physics in M1.2+, where it will be
judged on its own merits. No exit ramp taken yet on any M1.1 goal. Per
`cycle-plan` §1c step 3, defaults to single-pass.

**Push on vs. patch back:** push on. Round 2 advanced cleanly; its
carried-forward flags (out-of-bounds contract, unit mismatch, whole-crate
fmt) don't block this round — none of them are touched by a
measurement/reporting layer that reads existing `Grid`/`Material` state
without reasoning about physical units numerically beyond summing them.

## Goals

1. **A headless runner.** A function (native, no browser, no wasm-bindgen
   dependency) that takes a `Scenario`, builds its `Grid`, steps it a given
   number of times (using round 2's `Grid::step`, fed a fixed `dt` per call
   rather than real wall-clock time — a headless run doesn't have a
   browser's clock), and returns a measurement value.
2. **JSON measurements.** The measurement includes, at minimum: total mass
   per material (a placeholder unit is fine — round 1's Refactor already
   flagged `density` as unpinned; a headless-measurement round is not the
   place to invent a unit system, just to sum whatever `density` values
   exist consistently), cell counts per material, and the tick count
   reached. Serialize it to JSON — pick a crate (e.g. `serde`/`serde_json`,
   or hand-rolled if a dependency feels disproportionate for this shape;
   your call, state the reasoning) or write a minimal hand-rolled JSON
   writer if you judge that the better trade for this small a surface.
3. **A test asserts on specific JSON values, not just "it parsed."** Run
   `stone_and_water_pool()` (round 2's fixture) for a stated number of
   steps and assert the exact expected mass-per-material and cell-count
   numbers in the resulting JSON.
4. **A conservation smoke check.** Because M1.1's step is identity-only,
   total mass and every material's cell count must be *exactly* unchanged
   after any number of steps. Assert this explicitly (e.g. after 100 steps)
   — not because 0.1%-tolerance conservation is this milestone's job (it
   isn't; M1.2+ owns that empirically once matter actually moves), but
   because a measurement path that can't even prove "nothing moved, so
   nothing changed" would be useless once real conservation checks arrive.
5. **No human in the loop.** The whole path — build scenario, run, measure,
   assert — must be one `cargo test --lib`-reachable test, no manual step,
   no screenshot, no printed output a person has to read and judge.

## Intent

Headless empirical measurement: a scenario emits numbers, and a test
asserts on them without a human looking. This is milestone target 1 and the
scaffolding every later milestone's conservation/stability targets build
their own assertions on top of.

## Scope and focus

**Scope:** new code (likely `src/measure.rs` or similar — your call on the
module name), consuming round 1/2's `Grid`/`Material`/`Scenario` as-is
(read-only use; do not modify their public APIs unless something genuinely
cannot work, in which case say so loudly). May add a dependency to
`Cargo.toml` for JSON serialization if you judge it warranted — check it
does not break the `wasm32-unknown-unknown` build (native-only headless
code can be gated the way `src/bin/native_viewer.rs`'s `image` dependency
already is in `Cargo.toml`, if a chosen crate has wasm-target trouble).
**Focus:** the measurement path being genuinely usable headless (no
browser, no DOM) and its JSON shape being something round 4's renderer
context and later milestones' conservation tests can both read without
guessing at field names.
