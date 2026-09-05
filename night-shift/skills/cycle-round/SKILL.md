---
name: cycle-round
description: Run one round — plan it, then either dispatch a single self-verifying pass or, for rounds judged risky, Red/Green/Refactor as fresh isolated agents, and decide what happens next. Use to run a single round of the cycle.
user-invocable: true
---

# cycle-round — run one round

Read `cycle-contract` first. You are the orchestrator for this round.

`cycle-plan`'s round-level planning (§1c step 3) will have already decided whether
this round is **single-pass** or **risky**. That decision is frozen for the round;
don't second-guess it mid-dispatch. Section 2 below covers both.

If you are running more than one round (e.g. a milestone orchestrator working
through a list of rounds), run them **one at a time, strictly sequentially**: this
round's phase(s) and close-out all finish before the next round's dispatch begins,
even though nothing technically stops firing them concurrently.
Tranche 0's M0.4 milestone orchestrator dispatched three rounds with overlapping
timestamps despite its own plan saying otherwise; it was harmless only because the
rounds happened to touch disjoint files — that was luck, not design. This is
CLAUDE.md's "do one thing at a time" applied at round granularity.

Your job is to dispatch phases and pass **the smallest possible handoff** between
them. Never paste file contents, and never pass one phase's reasoning to the next.

---

## 1. Plan the round

Run `cycle-plan` at round level (or take the round plan you were handed). You need,
before dispatching anything:

- the round number and its log path,
  `cycle-log/<tranche-slug>/<milestone-slug>/round-NN.md`;
- the round's goals, and the round's and milestone's intent;
- whether the round was planned single-pass or risky, and if risky, Refactor's
  scope and focus for this round, and any adversarial focus.

Create the round log with a header naming the round, its goals and its intent.

---

## 2. Dispatch the phase(s)

Every dispatch is a **fresh isolated agent** — use the `Agent` tool, and never
`subagent_type: "fork"` (a fork inherits your context, which destroys the
isolation the whole design rests on; see `cycle-contract` §3a).

### Single-pass rounds (the default)

One dispatch. Prompt it with the round's goal, its intent, and the round log
path. It builds the goal, tests it, re-reads its own diff cold against the goal
before committing, and appends one report to the round log in the standard
format — no separate Red/Green/Refactor handoff. Model **sonnet, medium effort**.

Take its report at face value the same way you would Refactor's recommendation in
a risky round: it says whether the goal was met, you decide whether that's enough
to advance (see §3).

### Risky rounds

Default model **sonnet**, medium effort. Refactor gets **sonnet, high effort** — it
has the hardest job.

**Red.** Prompt: invoke `cycle-red`. Give it the goals, the intent, its scope and
focus, the round log path, and the existing test paths if it is extending.

Take its report at face value. Don't read the test file yourself, don't re-run it.

**Green.** Prompt: invoke `cycle-green`. Give it the goals, the intent, its scope
and focus, the round log path, the test paths, the failing test names, and the
skeleton signatures Red listed. Nothing about Red's approach — you don't have it
anyway.

Take its pass/fail report at face value.

**Refactor.** Prompt: invoke `cycle-refactor`. Give it the goals, the intent, **its
scope and focus**, the round log path, and the paths Green touched. Not Green's
implementation notes — Refactor is meant to look at the result cold.

---

## 3. Decide what happens next

The round's dispatch — Refactor in a risky round, the single pass itself in a
single-pass round — answers: have the round's goals been met to a sufficient
standard? That answer is a **recommendation, not a command.** It's a cold,
isolated, single-round agent — well placed to judge whether this round's code is
right, badly placed to judge whether the *goal* is worth continuing to pursue,
because it has no memory of what happened in this goal's earlier rounds. You are
the only thing in the system with that memory. Deciding what to do with the
recommendation is your job, not a formality:

- **Advance** — the normal case when the round recommends it. Run the suite once
  yourself. This is the only thing you verify independently: one cheap check, not
  a re-audit. Then close the round out.
- **Cycle** — the round runs again with the required updates as the new input
  (Refactor's, in a risky round; the pass's own stated gaps, in a single-pass
  round). Go back to step 1 with them. A corrective round is a normal outcome,
  not a failure.
- **Back to planning** — take the exit ramp. Run `cycle-plan` with what was
  learned, per its "re-planning after an exit ramp" section.

**The loop guard, and the one place you overrule the round's own recommendation:**
if this is the **third or later** round in a row still working the same goal,
escalate to planning regardless of what this round recommends — even "advance,"
and especially another "cycle." Three fresh, independent passes reaching for
"cycle again" on the same goal is itself evidence the goal is framed wrong, not
evidence the code needs one more pass. Say so explicitly when you escalate: which
round this is on the goal, and what each pass said. A goal stuck like this is also
itself a sign it may have been mis-judged single-pass — worth naming to the
planner even if it was actually run risky throughout.

If any phase returns an exit ramp instead of a normal result — an unattainable
goal, a 30-minute decision point with no clear path, a tooling failure — do not
retry it yourself and do not work around it. Route it to `cycle-plan`.

---

## 4. Close the round out

- Confirm the working tree is clean and the round's work is committed.
- Roll up the timing: each phase's elapsed time and the round's total, appended to
  the round log.
- Report the round: goals, verdict, what changed, time, and every gap or flag that
  is still open — those are the next planner's raw material.
