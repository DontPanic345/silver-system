# A constellation of stale North Stars

Every experiment run in this repo has opened by stating what it was for. None of
those statements has survived contact with what actually got learned running it —
not because the statements were wrong, but because each experiment ended for
reasons the statement itself couldn't see coming (a structural limitation, a
process that ate its own budget, a scope that turned out to be someone else's
job). This file collects them, retired, in order, so the next one doesn't quietly
restate a framing that's already been tried without knowing it.

Read this before writing a new north star. If what you're about to write is
already here in different words, that's worth knowing before, not after.

---

## 1. Terrarium (2026-09-01)

> A small system that can run itself indefinitely once sealed — not just a
> prettier sandbox.

A browser falling-sand sim: fractional-mixture cells (sand/clay/biomass/water +
a gas headspace), growing toward a sealed glass jar with a day/night cycle and a
closed water cycle (evaporate → condense → rain), nothing added or lost once
sealed.

**Status:** shelved 2026-09-02 as a successful test. Phase 0 (jar + light cycle)
and Phase 1 (closed water cycle, conserved by construction) both done. Phase 2
(plants — the real design risk) never started. Kept under `terrarium/`.

## 2. The GPU-native vision doc (2026-09-02)

> Don't script interesting outcomes — build simple interacting physical systems
> (materials, fluids, temperature, pressure, reactions) and let behaviour emerge.

A reconstructed development-handoff doc (Oxygen Not Included + Noita inspiration),
explicit that it was a first pass at capturing an idea, not settled design:
data-driven materials, GPU compute as foundational architecture (not a later
optimisation), determinism as an explicit goal, Rust floated as the core
language, phased physics → chemistry → biology → game layer.

**Status:** never built directly. Partially adopted into the Rust pivot below
(the emergent-behaviour framing, Rust, the tranche ordering); GPU-as-foundational
and determinism-as-a-standing-requirement were explicitly dropped — see
`night-shift/skills/cycle-contract`'s framing of determinism as
architecture-contingent, not standing.

## 3. stable-fluids (2026-09-02)

> A closed water cycle driven by physics — heat the water and it boils, the
> vapour rises, cools, condenses, and rains back down. Mass and energy go round
> the loop and are conserved to a good approximation.

A dependency-free browser fluid sim: Stam-style stable fluids, Jacobi pressure
projection, MUSCL advection, temperature + buoyancy, water/vapour/air phase
change. Built test-first over named rounds against a frozen acceptance-criteria
doc.

**Status:** shelved 2026-09-05. Rounds 1–6 of 7 passed; Round 7 halted on a
structural limitation — a checkerboard null mode in the colocated pressure
projection that needed a staggered or compact `project()` rewrite nobody
returned to build. Its retrospective (`stable-fluids/tdd-cycle-closeout.md`) is
where most of the next experiment's design came from.

## 4. night-shift (2026-09-05)

> A believable small world inside a large universe. A terrarium people can see
> on their screens and interact with.

The Rust restart: a `tranche → milestone → round → phase` cycle of
self-planning, self-dispatching agents, meant to run indefinitely across four
content tranches (mathematics/tooling, physics, chemistry, biology, the glass
pane) without a human re-briefing it at every step.

**Status:** shelved 2026-09-05. Tranche 0 closed out in full; tranche 1's first
milestone (M1.1) closed out before the run stopped. Genuinely proved out the
things it was built to prove — a cold Refactor pass catches what a continuous
pass misses, planning that folds the last round forward beats stable-fluids'
memoryless churn — but the process itself (built once from a single dictation
dump, run for a full tranche before anyone reviewed it) was found to be spending
a large share of its own budget on ceremony rather than the work: ~12 lines of
process log per line of shipped logic, ~1.7 comment lines per line of actual
code, and a real session usage limit hit one milestone into a seven-milestone,
four-tranche plan. See `night-shift/CLOSEOUT.md` for the full retrospective. The
Rust code it produced (`src/`, `www/`, `tests/`) is kept; the process and the
plan built around this statement are not.

---

## Next

A one-shot prompt, deliberately without a stated north star handed to it up
front, kept the Rust substrate above as its starting material. Whatever comes
out of that run — including whether it needs a north star at all, or arrives at
one on its own — is the yardstick every future structure in this repo gets
calibrated against, rather than another single unreviewed dictation dump.
