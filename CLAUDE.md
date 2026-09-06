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

- `src/`, `www/`, `tests/`, `scripts/` — the live Rust code. Some of it
  (`grid.rs`, `scenario.rs`, `measure.rs`, the native fallback) is leftover
  substrate from the `night-shift` experiment, kept but no longer central;
  the current experiment is listed below. See `README.md` for how to build,
  run and test.
- `terrarium/`, `stable-fluids/`, `night-shift/` — shelved experiments, kept for
  reference. Do not extend them.

## Current experiment

**Gnomes** (started 2026-09-07) — the first run aimed at `NORTH_STARS.md` #4
directly rather than at substrate under it. A conserving simulation (mass,
energy, phase change, all conserved by construction) with a colony game on
top, in which a gnome's magic is the single accounted-for exception to an
otherwise closed world, paid for in Gin and recorded in a ledger.

Live code: `src/world.rs`, `src/physics.rs`, `src/gnome.rs`,
`src/terrarium.rs`, `src/report.rs`, `www/terrarium.html`. Run it headless
with `cargo run --release --bin terrarium -- --map`. No process cycle
governs this experiment — deliberately, after `night-shift`.

The rule that matters when changing anything here: **every operation must
conserve mass and energy, or go through `World::conjure_*` so the ledger
records it.** The tests assert the residuals, so breaking this fails loudly
rather than drifting quietly.
