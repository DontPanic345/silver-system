---
name: cycle-tranche
description: Run a tranche end to end — plan it into milestones, run milestones to their targets, finish with a tranche-scope refactor pass, and chain forward to the next tranche. This is the top of the cycle — there is no fixed end; the system runs until PLAN.md itself is exhausted, at which point planning decides what the world needs next. Use to drive the whole build, or a single named tranche.
user-invocable: true
---

# cycle-tranche — run a tranche, then keep going

Read `cycle-contract` first. You are the orchestrator for the tranche, and — unless
told to stop after one — for everything after it too. This is the top of the
hierarchy in daily use: nothing above it hard-stops the system, by design. Only
when the big universe is sufficiently full will the small world be believable, and
that is not a single sitting's work.

---

## 1. Plan the tranche

Run `cycle-plan` at tranche level. You need its intent, its **targets**, and a
first list of milestones. `PLAN.md` is the starting position for tranche intent,
not a spec to execute blindly — sharpen it with whatever has been built already.

If you were handed a specific tranche by name, plan that one. If you were handed
nothing, take the next tranche in `PLAN.md` that isn't already closed out.

---

## 2. Run milestones

For each milestone: run `cycle-milestone`. Between milestones, re-plan — `cycle-plan`
at tranche level, reading the previous milestone's closeout. This is where a
tranche adapts across milestones the way a milestone adapts across rounds.

After each milestone, ask which of the tranche's targets are now met. Targets are
measurable: measure them.

Keep going until the tranche's targets are met, or until planning concludes they
can't be as framed. There is no halting clause; there are only exit ramps, and the
one available here is: rewrite the tranche's targets or milestone breakdown in
`PLAN.md` and continue, or hand the decision to the user if it's a real ambiguity
rather than a restructuring call you can make yourself.

---

## 3. Tranche-scope refactor

When the milestones are done, run one more `cycle-refactor` with **scope: the
whole tranche**, focus chosen from what the milestone closeouts have been
complaining about. Treat its recommendation the same way a round treats Refactor's
recommendation in `cycle-round`: yours to weigh, not to obey blindly.

---

## 4. Close the tranche out

Write `cycle-log/<tranche-slug>/closeout.md`:

- each target, whether it was met, with the measurement;
- milestones run, and the timing roll-up — per milestone and the tranche total;
- what was learned that changes the plan for tranches after this one;
- open gaps and flags carried forward;
- what the cycle itself got wrong across the whole tranche — candidate fixes to
  `cycle-*` skills, named.

Mark the tranche closed in `PLAN.md`.

---

## 5. Chain forward — this is the part that makes it indefinite

Do not stop here unless told to. The system runs continuously; a tranche finishing
is a milestone in the larger sense, not a terminus.

- **If `PLAN.md` has a next tranche**, go to step 1 with it. Fold this tranche's
  closeout forward the same way a round's report gets folded into the next round's
  plan — a later tranche's intent may sharpen, or its targets may need revising,
  in light of what this one actually built.
- **If `PLAN.md` is exhausted** (all four tranches closed), that is not the end of
  the project — it's the first point where the whole planned scope actually exists
  to look at. Run `cycle-plan` at tranche level with no tranche handed to it, and
  let it decide: a new tranche entirely, a return pass deepening an earlier one now
  that later tranches reveal what it should have supported, or something the
  dictated plan never anticipated. Say explicitly that you've reached this point —
  it's worth the user knowing, even though no approval is required to keep going.

Report to the user at the end of each tranche's closeout regardless of whether you
continue automatically or were asked to run one tranche at a time — a closeout is
worth seeing, not just logging.
