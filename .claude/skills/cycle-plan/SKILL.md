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

## The north star

`PLAN.md` opens with it, in two statements: **a believable small world inside a
large universe**, and **a terrarium people can see on their screens and interact
with**. Both are load-bearing.

Every plan you write states, in its own words, how the work it is planning serves
them. Not a ritual sentence — a real claim you could be wrong about. If you cannot
say how the work in front of you serves the north star, **that is the finding**:
report it rather than planning the work anyway.

## Plan only what this tranche needs, defer the rest

A tranche's scope is **what it needs to support the north star right now** —
not the fullest scope you can honestly justify. `PLAN.md` records the thinnest
sketch of what's wanted for a reason: the cycle is meant to keep running,
indefinitely, tranche after tranche. A reach item you fold into this tranche
because it might be useful later is scope this tranche didn't need, carried by
work happening now instead of by a future tranche planned for it — the flow will
get there on its own.

So when you notice something that belongs in the project but isn't this tranche's
job — a primitive a later tranche will want, a piece of tooling that would help
down the line — **name it and defer it**, don't fold it in. Record it where the
relevant future tranche's planner will find it (a note in `PLAN.md`, or a flag in
this tranche's closeout under "open gaps"). That's a real, tracked commitment to
build it later, not a decision to drop it.

Fold something in now only when the tranche's own intent genuinely can't be met
without it — not "would benefit from," but "this tranche does not support the
north star without this." Say explicitly which test you applied when you do.

The counterweight is still the round: a round is small, and a phase has 30
minutes. Do one thing at a time — and now, do only the thing the tranche actually
needs.

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
  whatever has been built since it was written, and say how it serves the north
  star.
- What does this tranche need, beyond what `PLAN.md` mentions, that its own intent
  genuinely can't be met without? Fold that in, and say why it was necessary, not
  just useful. Anything a *later* tranche would want but this one doesn't strictly
  need — name it and defer it (see "Plan only what this tranche needs" above)
  rather than building it early.
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
3. **Risky, or single-pass?** Full Red/Green/Refactor is expensive — three agent
   dispatches and a handoff between each — and running it by default on every
   round, regardless of what the round actually is, generates its own overhead:
   tranche 0 proved this concretely (a static rectangle and a handful of math
   primitives each took multiple rounds of phase ceremony, and a real share of
   what got "found and fixed" was the ceremony itself misfiring — forks that
   couldn't spawn subagents, rounds dispatched out of sequence — not domain work).
   So default to **single-pass**: one fresh agent builds the round's goal, tests
   it, reviews its own diff against the goal, and commits — one report, the same
   format as any phase's. Reserve full Red/Green/Refactor for a round you judge
   **risky**:
   - it touches a shared primitive or interface other code already depends on
     (the blast radius reaches past this round);
   - it bears on a stated conservation/determinism target, where a cold second
     look is the point, not a formality;
   - the same goal already took an exit ramp or a cycle-again — it's proven
     tricky once already;
   - it changes something hard to reverse (a public interface, a deploy path, a
     data format);
   - you are genuinely unsure the approach is right and want an independent,
     isolated check on it.
   State which you chose for this round, and why, in the round header — this is
   frozen for the round the same way a goal is once a phase has started on it.
4. **Refactor's scope and focus** (risky rounds only). Every risky round gets a
   Refactor pass at round scope at minimum. If nothing risky changed this round,
   spend that budget on a *larger* scope and a broader focus — the milestone, the
   test suite, performance, a security or gap sweep. If significant feature work
   landed, aim Refactor at that work and at folding it into the existing codebase.
5. **Adversarial focus, if any** (risky rounds only). The useful adversarial
   angles are not knowable before the project is known; they come from the
   domain. Long runs, resting states, sharp interfaces, symmetry, conservation
   over time, boundary cells. Pick the one this round's changes most plausibly
   broke, and hand it to Refactor.
6. **The phase prompt(s).** For a single-pass round, write one self-contained
   prompt carrying the round's goal, its intent, and the round log path. For a
   risky round, write one for each of Red, Green and Refactor, each carrying: the
   round's goals, the round's and milestone's intent, its own scope and focus, and
   the round log path. Nothing else — no reasoning from another phase, no file
   contents.

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
