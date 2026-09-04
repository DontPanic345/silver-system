# silver-system

Building a small world that is part of a much larger universe.

## Principles

- **Only when the big universe is sufficiently full will the small world be believable.**
- **The only way to go fast is to go well.**
- **Do one thing at a time.**
- **Trust and verify.**

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

## Determinism

Bit-identical determinism is **architecture-contingent**, not a standing
requirement. GPU execution is the intended direction. Don't engineer for
determinism unless a goal asks for it.
