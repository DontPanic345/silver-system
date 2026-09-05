# silver-system

## The north star

> **A believable small world inside a large universe.**

> **A terrarium people can see on their screens and interact with.**

Both statements, together, are the goal. The first is the standard — believability
comes from the universe underneath being genuinely full, not from what the camera
shows. The second is the deliverable. `PLAN.md` opens with the long version; every
tranche states how it serves them.

## Principles

- **Only when the big universe is sufficiently full will the small world be believable.**
- **The only way to go fast is to go well.**
- **Do one thing at a time.**

## How work is organised

`tranche → milestone → round → phase`, defined in `.claude/skills/cycle-contract`.
Read that skill before working inside the cycle. In short:

- A **phase** is one agent's turn — Red, Green or Refactor. Never call it a step;
  "step" is reserved for algorithmic steps (a simulation step, a solver step).
- A **round** is one Red → Green → Refactor pass. Rounds have **goals**.
- A **milestone** is a group of rounds. Milestones have **intent** and **targets**.
- A **tranche** is a group of milestones. Tranches have **intent** and **targets**.

**Targets** are measurable and objective — they are met or they aren't.
**Goals** require interpretation against the stated intent. Once an implementation
phase has started on a goal, that goal is frozen for that phase.

Don't say "acceptance criteria". Rounds have goals; milestones and tranches have
targets.

## Current state

The work plan is `PLAN.md`. The language is Rust.

`terrarium/` and `stable-fluids/` are shelved experiments, kept for reference — do
not extend them.

## Determinism and conservation

Bit-identical determinism is **architecture-contingent**, not a standing
requirement. GPU execution is the intended direction. Don't engineer for
determinism unless a goal asks for it.

Conservation (mass, energy, carbon, nitrogen, ...) follows the same logic but is
not optional the way determinism is: every "conserved" target means conserved to a
**stated numerical tolerance**, not literal zero drift. Measure it, record the
tolerance, don't chase bit-exact equality past what the tolerance needs.
