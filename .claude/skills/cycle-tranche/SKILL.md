---
name: cycle-tranche
description: Run a tranche end to end — plan it into milestones, run milestones to their targets, and finish with a tranche-scope refactor pass. Stops and reports at the tranche boundary by default; chains forward into the next tranche only under an explicit standing instruction to keep going. Use to drive a single named tranche, or a whole continuous run when asked.
user-invocable: true
---

# cycle-tranche — run a tranche, then stop and report

Read `cycle-contract` first. You are the orchestrator for the tranche. Unless the
user has given a standing instruction to keep going across tranche boundaries (see
§5), your job ends when this tranche is closed out and reported — you are not the
top of an unsupervised, indefinitely-running hierarchy by default.

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

## 5. Stop and report — chain forward only when told to

**Default: stop here.** Report the tranche's closeout to the user and wait. This
is not the system's permanent shape — indefinite chaining (below) is the intent
once the cycle has earned enough trust that a tranche closing out cleanly is a
reliable signal on its own. It hasn't yet: this whole system was drafted once from
a single dictation dump and run for a full tranche before anyone reviewed it, so
the default posture is a human checks in at each tranche boundary, not the system
running on unsupervised.

Chain straight into the next tranche (skip the stop) **only** when the user has
said, for this run, to keep going without stopping at tranche boundaries — a
standing instruction for the whole run, not implied by having been told to run
"tranche 0" or any single named tranche.

When you are told to chain forward:

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
  it's worth the user knowing even under a standing keep-going instruction.

Report to the user at the end of every tranche's closeout regardless of whether
you stop there or chain forward under a standing instruction — a closeout is
worth seeing, not just logging.
