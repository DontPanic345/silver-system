---
name: cycle-milestone
description: Run one milestone end to end — plan it into rounds, run rounds until its targets are met, and finish with a milestone-scope refactor pass. Use to drive a single milestone. To run a whole tranche milestone by milestone, use cycle-tranche instead.
user-invocable: true
---

# cycle-milestone — run a milestone to its targets

Read `cycle-contract` first.

---

## 1. Plan the milestone

Run `cycle-plan` at milestone level. You need its intent, its **targets**, and a
first list of rounds. Expect that list to change; it is a starting position.

You should have been handed one milestone to run, by `cycle-tranche` or by the
user directly. If you were handed a whole tranche instead, stop and use
`cycle-tranche` — don't improvise tranche-level looping here.

---

## 2. Run rounds

For each round: run `cycle-round`. Between rounds, re-plan — `cycle-plan` at round
level, reading the previous round's log. That re-plan is not a formality; it is
where the milestone adapts.

After each round, ask which of the milestone's targets are now met, and say so
explicitly. Targets are measurable: measure them, don't estimate them.

Keep going until the targets are met, or until planning concludes they can't be as
framed — in which case take the exit ramp back to whoever called you (`cycle-tranche`,
or the user) with what was learned. There is no halting clause; there are only exit
ramps.

---

## 3. Milestone-scope refactor

When the rounds are done, run one more `cycle-refactor` with **scope: the whole
milestone**, and a focus you choose from what the round logs have been complaining
about — general code quality, performance against the milestone's targets, the test
suite, gaps, or an adversarial sweep across everything the milestone built.

This too is a recommendation, on the same terms as a round's Refactor pass in
`cycle-round` — you decide whether it means the milestone is genuinely done or
needs another pass.

---

## 4. Close out

Write `cycle-log/<tranche-slug>/<milestone-slug>/closeout.md`:

- each target, and whether it was met, with the measurement;
- rounds run, and the timing roll-up — per round and the milestone total;
- what was learned that changes the plan for what comes next;
- open gaps and flags carried forward, and where they are recorded;
- **what the cycle itself got wrong.** Every recurring slip across the round logs
  is a candidate fix to a `cycle-*` skill. Name them.

Then update `PLAN.md` if the ground has moved.

Every quantitative claim in a closeout must cite where the number came from — a
measurement you ran, or a specific round log. Do not write a number you have not
checked.

Return the closeout to whoever called you. If that's `cycle-tranche`, it decides
what runs next; this skill's job ends here.
