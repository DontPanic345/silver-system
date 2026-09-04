# stable-fluids/ — shelved

A dependency-free browser fluid sim: Stam-style stable fluids with a Jacobi
pressure projection, MUSCL conservative advection, a temperature field with
buoyancy, and water/vapour/air channels with phase change.

Built test-first over Rounds 1–7 of `WATER_SIM_AC.md`. Rounds 1–6 passed; Round 7
halted on a structural limitation — a checkerboard null mode in the colocated
pressure projection, which needs a staggered or compact `project()` rewrite.

Shelved on 2026-09-05 in favour of a Rust restart. Kept for reference.

## What's here

- `index.html`, `css/`, `js/`, `test/`, `scripts/`, `package.json` — the sim.
  Run from this directory: `npm test`, `npm run shot`.
- `WATER_SIM_AC.md` — the acceptance criteria the rounds were built against.
- `tdd-cycle-log.md` — every phase report from the run, in order.
- `tdd-cycle-closeout.md` — the retrospective. Most of what the current cycle
  skills do differently traces back to a finding in here.
- `skills/` — the `tdd-*` skill family that ran it, moved out of the user's
  global skills directory. Superseded by `.claude/skills/cycle-*`.
