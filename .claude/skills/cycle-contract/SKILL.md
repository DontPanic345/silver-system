---
name: cycle-contract
description: The shared contract every cycle agent works under — the tranche/milestone/round/phase vocabulary, the 30/60-minute timing protocol, exit ramps, git hygiene, and the report format. Loaded by cycle-plan, cycle-round, cycle-red, cycle-green and cycle-refactor rather than restated in each. Read it when working anywhere inside the cycle.
---

# cycle-contract — the rules every phase works under

This is the common ground. Every other `cycle-*` skill assumes you have read it.

---

## 1. Vocabulary

```
tranche  →  milestone  →  round  →  phase
```

- A **phase** is one agent's turn: Red, Green or Refactor. Never call a phase a
  "step" — *step* is reserved for algorithmic steps (a simulation step, a solver
  step).
- A **round** is one pass at a goal — a single self-verifying phase by default, or
  a Red → Green → Refactor triad when `cycle-plan` judges the round risky (see
  `cycle-plan` §1c). Which shape a round takes is a planning decision, not fixed
  by the vocabulary.
- A **milestone** is a group of rounds.
- A **tranche** is a group of milestones. `PLAN.md` lists them.

**Intent** — what this level is actually for, in plain language. Tranches and
milestones always have one. A round may.

**Targets** belong to tranches and milestones. They are measurable, quantifiable,
objective facts. They may or may not be achieved, and whether they were is not a
judgement call.

**Goals** belong to rounds. They are open to interpretation *given the intent*, and
they require interpretation — so they should be clear. A planner or another
iterative agent may reinterpret or reword a goal. But **once an implementation
phase has started on a goal, that goal is frozen for that phase.** Finish against
the goal as you received it; if it is wrong, say so in your report.

Do not use the term "acceptance criteria" anywhere. It carries a frozen, clinical
connotation this system deliberately rejects.

---

## 2. Scope and focus

Every phase is dispatched with a **scope** and a **focus**, set by the planner.

- **Scope** is the surface under consideration: the current diff, this round, the
  whole milestone, the test suite, the whole codebase.
- **Focus** is what to look for within it: the current change, general code
  quality, performance, the test suite, security, gaps.

Work the scope you were given. If you believe the scope or focus is wrong, do the
work anyway and say so in your report — the planner folds that back in next time.

---

## 3. Autonomy

All decisions can be made without human input. Nobody is waiting to adjudicate.
Use your judgement, act, and report what you decided and why. Your report is the
verification surface: **trust and verify.**

The lever the whole flow relies on is your ability to reason and choose the next
best move. Choose it.

---

## 3a. Orchestrator self-dispatch, across long sessions

A `cycle-tranche` or `cycle-milestone` run can itself run long enough that the
orchestrating agent's own context becomes the bottleneck. When you need to hand
a whole milestone (or tranche) off to isolate it from your own growing context,
dispatch it as a **fresh, cold `Agent` call** (e.g. `subagent_type: "claude"`),
not `subagent_type: "fork"`.

This was learned the hard way, twice (tranche 0, M0.2 and M0.3): a forked agent
inherits the caller's context, but it **cannot itself spawn further subagents**.
Handed a whole milestone, it has no way to give `cycle-round`'s Red/Green/Refactor
their own fresh, isolated `Agent` calls — the one thing `cycle-round` exists to
guarantee — so it collapses the milestone into one continuous, unisolated pass.
That silently throws away the cold-Refactor-catches-Green's-blind-spot property
the whole phase design rests on.

A cold agent has no such restriction: it can plan, then dispatch true isolated
Red/Green/Refactor phases underneath itself exactly as `cycle-round` describes.
It costs a short, self-contained prompt (point it at `PLAN.md`, the relevant
`cycle-log/**` files, and which `cycle-*` skills to read) instead of free
inherited context — a real cost, but a small one, and it buys back the phase
isolation a fork quietly loses. Prefer it for any milestone- or tranche-level
self-dispatch.

---

## 4. Timing protocol

At the very start of your phase, run `date -Is` and record it. Check the clock
again whenever you finish a meaningful chunk of work.

- **At 30 minutes, stop and decide.** If there is a clear path to finishing within
  another 30 minutes, continue into overtime. Otherwise stop now and report.
- **"This could not be achieved in 30 minutes" is an excellent report.** It goes
  back to planning to restructure the goal — more groundwork, a smaller change,
  different technology. It is not a failure and it is never worth disguising by
  grinding on.
- **At 60 minutes, stop.** Start no new endeavours. Report what you have.

Report your elapsed time and where it went. Timing is rolled up per round, per
milestone and per tranche.

---

## 5. Exit ramps

There is **no halting clause**. The cycle runs until the work is complete. What
exists instead are exit ramps back to planning.

Take an exit ramp when:

- the goal looks unattainable as framed;
- the 30-minute decision point has no clear path forward;
- you hit a tooling or environment failure — a missing dependency, a broken hook,
  a permission error. Do not work around tooling problems; report the exact command
  and the exact error;
- three attempts have not changed the failure signature at all.

An exit ramp returns to the planning level, which may restructure the goal, split
it into groundwork rounds, choose a different approach, or come back to it later.

---

## 6. Git hygiene

- **Leave the working tree clean.** If you stop with uncommitted work — including
  on an exit ramp — commit it and state plainly in the commit message and your
  report what is finished, what is not, and what state the tests are in.
- `git add` **explicit paths only**. Never `git add -A`.
- One commit per phase is the norm. Say what changed and why, not how.

---

## 7. The round log

Every round has its own log:

```
cycle-log/<tranche-slug>/<milestone-slug>/round-NN.md
```

Each phase **appends** its report to that round's log before finishing. Create the
file and the directories if they don't exist. Never write into another round's log,
and never keep a single log for the whole run.

---

## 8. Report format

Every phase returns a report, and appends the same text to the round log. Use these
headings:

```markdown
## <Round NN> — <Red|Green|Refactor> — <ISO start> → <ISO end> (<elapsed>)

**What I did.**

**Successes.**

**What was difficult, and where the time went.**

**Compromises I made.**

**Token usage.** Your own total token usage for this phase, if you have any way
to see it (a context/usage indicator, a final-turn summary, whatever your harness
surfaces). State the number plainly if you have it. If you have no visibility into
your own usage, say so explicitly — "not visible to me" is a valid report, a
missing heading is not.

**Gaps and flags.**  Anything the next phase or the planner should know about,
including things outside your scope that you noticed.

**General comments.**
```

Report the truth. If tests fail, say so and include the output. If you skipped
something, say you skipped it. If it works and you verified it, say so plainly
without hedging.

An orchestrator dispatching a milestone or tranche cold (per §3a) has no visibility
into the sub-agents it spawns beyond what they report back — it cannot read their
token usage off the `Agent` call the way a fork's caller can. So a milestone or
tranche closeout's timing/usage roll-up is only as complete as the phase reports
that fed it: if a phase said "not visible to me," the roll-up carries that gap
forward honestly rather than guessing a number.

---

## 9. Principles

- Only when the big universe is sufficiently full will the small world be
  believable.
- The only way to go fast is to go well.
- Do one thing at a time.
- Trust and verify.
