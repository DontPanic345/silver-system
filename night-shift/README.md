# night-shift/ — shelved

The `tranche → milestone → round → phase` cycle system: a hierarchy of
self-dispatching, self-planning agents built to run silver-system's Rust restart
indefinitely, tranche after tranche, without a human re-briefing it each time.

Shelved on 2026-09-05 in favour of a one-shot-prompt experiment. See
[`CLOSEOUT.md`](CLOSEOUT.md) for the full retrospective — it worked, as far as it
went, but the process itself (and the single unreviewed dictation dump it was
built from) was found to be spending a large share of its own budget on ceremony
rather than on the work.

## What's here

- `skills/` — the `cycle-*` skill family that ran it (`cycle-contract`,
  `cycle-plan`, `cycle-round`, `cycle-milestone`, `cycle-red`, `cycle-green`,
  `cycle-refactor`, `cycle-tranche`), moved out of `.claude/skills/`.
- `PLAN.md` — the tranche/milestone plan it was executing (mathematics/tooling,
  physics, chemistry, biology, the glass pane).
- `cycle-log/` — every phase, round, milestone and tranche report from the run,
  in order. Tranche 0 closed out in full; tranche 1's first milestone (M1.1)
  closed out before the run was shelved.

## What isn't here

The Rust code this run produced (`src/`, `www/`, `tests/` at the repo root) is
**not** shelved — it's kept as substrate for what comes next. See the top-level
`README.md` for how to build and test it.
