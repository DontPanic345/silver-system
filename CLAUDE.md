# silver-system

An experimental small-world simulation project — emergent physical behaviour
from simple interacting rules, in the spirit of Oxygen Not Included and Noita.
Each experiment here has run under its own approach; none has been kept once it
stopped earning its keep. Three files track different halves of that history:
[`JOURNAL.md`](JOURNAL.md) for the dated, concrete record of what was tried,
when, and why it ended; [`NORTH_STARS.md`](NORTH_STARS.md) for the vague,
aspirational statements that motivated it, which don't retire the way an
experiment does; [`PRINCIPLES.md`](PRINCIPLES.md) for the aphorisms distilled
along the way. Read `JOURNAL.md` before writing a new experiment, so it isn't
a repeat.

## What's here

- `src/`, `www/`, `tests/`, `scripts/` — Rust substrate, originally scaffolded
  under the `night-shift` experiment (grid/material types, a scenario harness,
  a minimal wasm renderer, a native fallback), now carrying real gravity/
  density physics (`src/grid.rs`) under active process — see `README.md` for
  how to build and test it.
- `terrarium/`, `stable-fluids/`, `night-shift/` — shelved experiments, kept for
  reference. Do not extend them.

## Current experiment

**Gravity/density falling-sand physics on the Rust substrate**, started
2026-09-05. `src/grid.rs`'s per-cell step went from a no-op identity
transform to a real, generic (data-driven, not per-material) movement rule:
denser cells swap into strictly-less-dense, non-`Solid` neighbours, in
priority order (straight down, diagonal-down, then — liquids only —
sideways, gated so it settles instead of sloshing forever). `Phase` grew a
`Granular` variant distinct from immovable `Solid`; `MaterialTable::reference`
grew a fourth material, sand. Every move is a swap of two cells' contents,
never a creation or deletion, so per-material cell counts are exactly
conserved by construction — see `src/measure.rs`'s and `src/grid.rs`'s own
conservation tests. Watchable live at `www/physics.html`
(`step_and_paint_physics_demo`), headless-verified both as unit/integration
tests (`cargo test --lib`) and as a real-browser e2e check
(`tests/e2e/physics_demo.test.mjs`, reading real canvas pixels after real
wall-clock time, not a screenshot). Not yet done: gas movement/buoyancy,
temperature, pressure, or anything past physics in the phased
physics→chemistry→biology→game-layer ordering (`NORTH_STARS.md` #2/#3).
