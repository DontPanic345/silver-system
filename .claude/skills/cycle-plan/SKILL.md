---
name: cycle-plan
description: Plan a tranche into milestones, a milestone into rounds, or a round into its goals and phase prompts. Reads the previous rounds' logs and folds their findings forward, sets each phase's scope and focus, and decides whether to push on or cycle back and patch gaps. Use before starting a tranche, before starting a milestone, and before every round.
user-invocable: true
---

# cycle-plan — plan the next tranche, milestone, or round

Read `cycle-contract` first. Then say which level you are planning: **tranche**,
**milestone**, or **round**.

Planning is not a fixed ceremony. It is the point where the system adapts to what
it has learned, and it is dynamic by design — the shape of the next round is not
knowable before the last one finished.

---

## 0. Read what happened

Before planning anything, read the logs of the rounds that have already run at this
level — `cycle-log/**/round-NN.md`, most recent first — and `PLAN.md`.

You are looking for:

- gaps and flags nobody has picked up yet;
- goals that turned out to be wrong, or that got reworded in flight;
- exit ramps taken, and what they said was unattainable;
- where the time actually went, round over round;
- anything a phase noticed outside its own scope.

**Folding this forward is your main job.** A planner that ignores the last round's
report is just a scheduler. The system is built to handle unknown unknowns, and
this is the only place they get handled.

---

## 1a. Planning a tranche

Output: an intent statement and a list of milestones, written to
`cycle-log/<tranche-slug>/plan.md`.

- Restate the tranche's **intent** from `PLAN.md` in your own words, sharpened by
  whatever has been built since it was written.
- Set the tranche's **targets** — measurable, quantifiable, objectively checkable.
  Not "pressure feels right"; "water in a U-pipe levels to within 1 cell across
  both arms and stays there for 5000 steps".
- Break it into milestones, each with its own intent and targets. Order them so
  that each is buildable on what precedes it. Say explicitly which milestone
  produces the first thing a human can watch.

## 1b. Planning a milestone

Output: an intent statement and a list of rounds, written to
`cycle-log/<tranche-slug>/<milestone-slug>/plan.md`.

- Milestone **intent** and **targets**, same standard as above.
- A list of rounds, each with a one-line goal. This list is a starting position,
  not a contract — expect to revise it after almost every round.
- Note any round whose goal is only honestly testable once a *later* round's
  mechanism exists. Hidden ordering dependencies are the most expensive thing a
  planner can miss.

## 1c. Planning a round

Output: the round's header, written to a fresh
`cycle-log/<tranche-slug>/<milestone-slug>/round-NN.md`, and the prompts you will
dispatch.

Decide, in this order:

1. **Push on, or patch back?** Balance finishing the milestone against closing gaps
   the last rounds surfaced. Striking that balance is the job. Say which you chose
   and why.
2. **The round's goals.** Small enough to land in one round, meaningful enough to
   matter. Write them so they are clear, knowing they will be interpreted. Give the
   round an intent if the goals alone would mislead.
3. **Refactor's scope and focus.** Every round gets a Refactor pass at round scope
   at minimum. If nothing risky changed this round, spend that budget on a *larger*
   scope and a broader focus — the milestone, the test suite, performance, a
   security or gap sweep. If significant feature work landed, aim Refactor at that
   work and at folding it into the existing codebase.
4. **Adversarial focus, if any.** The useful adversarial angles are not knowable
   before the project is known; they come from the domain. Long runs, resting
   states, sharp interfaces, symmetry, conservation over time, boundary cells. Pick
   the one this round's changes most plausibly broke, and hand it to Refactor.
5. **The phase prompts.** Write a self-contained prompt for each of Red, Green and
   Refactor, each carrying: the round's goals, the round's and milestone's intent,
   its own scope and focus, and the round log path. Nothing else — no reasoning
   from another phase, no file contents.

---

## 2. Re-planning after an exit ramp

When a phase came back saying the goal was unattainable, do not re-issue the same
goal. Choose one:

- **Groundwork first** — insert one or more rounds that build what was missing.
- **Smaller** — cut the goal down to the part that is reachable now.
- **Different approach** — the technique was wrong, not the ambition.
- **Later** — the goal depends on something a future milestone provides. Record
  the dependency where the future planner will see it.

Say which you chose and why, in the plan file.

---

## 3. Report

Follow the report format in `cycle-contract`. Additionally state:

- what you folded forward from previous rounds, and from which round;
- the decision from step 1 (push on vs. patch back) and its reasoning;
- anything you deliberately deferred, and where you recorded it.
