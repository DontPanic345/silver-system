---
name: cycle-red
description: The Red phase of a round — write the failing scenario tests for the round's goals, and the shape-only skeleton code (signatures and stubs that panic) that Green will fill in. Also owns the health of the test suite: tagging, fast paths, runtime. Use as the first phase of a round, or standalone to add a failing test.
user-invocable: true
---

# cycle-red — write the failing tests and the skeleton

Read `cycle-contract` first. You should have been given: the round's goals, the
round's and milestone's intent, your scope and focus, and the round log path.

Start your timer (`date -Is`).

---

## 1. Check the goals still hold

You may have been handed a pre-baked block of goals from the milestone plan. You
are not obliged to take its structure. Confirm it still makes sense given what has
actually been built — plans written three rounds ago do not know what round three
learned.

You may restructure the slice: resequence it, split it, pull a piece forward. The
reason you have this latitude is to **catch hidden ordering problems the pre-baked
grouping didn't anticipate** — a goal that can only be honestly tested once a later
round's mechanism exists, or two goals that are really one.

If you restructure, say so and why. If a goal is vague, contradictory, or conflicts
with the stated intent in a way that changes what should be tested, don't guess —
take the exit ramp back to planning.

Aim for a slice **small enough to land in one round, meaningful enough to matter.**

---

## 2. Write the tests

- **One test is one scenario.** Write them as user stories — closer to BDD than to
  unit tests. A test's name and body should read as a restatement of the behaviour
  it guards, because Green and Refactor will only ever see the goals, the intent
  and this test, never your reasoning.
- Test against the **intent**, using the goals as its concrete restatement. Where
  intent flags a number as illustrative rather than load-bearing, test the intent,
  not the number.
- **Every scenario carries its own empirical pass/fail check.** No human looks at
  it. If a scenario's success is "the water finds a level", the test measures the
  level.
- Unit tests alongside are welcome where they help. They are **disposable**: a
  later phase may delete a unit test whose code has changed out from under it. Say
  which of your tests are scenarios (durable) and which are unit tests (disposable).
- **Green guard tests are in remit.** A cross-cutting invariant that is already
  green — conservation, boundaries, no NaNs — is worth pinning now so a later round
  can't quietly break it. And a round whose physics can't be exercised until Green's
  code exists may carry its red signal on structural assertions: that the scenario
  exists, that the entry point is callable, that the stub is gone.
- **Pin the conventions once.** For anything with a coordinate system, a grid
  orientation, or a sign convention, write one test that states which way is up and
  what a positive value means. Getting this wrong silently is the single most
  expensive class of bug in this domain.

---

## 3. Write the skeleton

Give Green the shape of the code so it never has to guess a signature.

- Real modules, real types, real function signatures, real parameter order —
  everything the tests call, existing and compiling.
- Bodies are `todo!()` or `unimplemented!("<what goes here>")`. The tests run and
  fail on a panic from a named stub, not on a compile error.
- **Shape only.** No arithmetic, no logic, no algorithm, no "helpful" partial
  implementation. If you find yourself writing an expression that computes
  something, stop — that is Green's work and writing it here destroys the isolation
  that makes Refactor's cold read valuable.

Doc comments stating what a function is *for* are fine and useful. Doc comments
stating *how* it should work are not.

---

## 4. Own the test suite

Nobody else's job. Every round, leave the suite better to work with than you found
it:

- **Tag and organise** so any later phase can run a specific collection with one
  simple command — this round's tests, this milestone's scenarios, the fast subset.
  State the exact commands in your report.
- **Keep it fast.** Long stability runs multiply. Collapse redundant ones, mark the
  slow completeness scenarios so they can be excluded from the default run, and
  flag it if the default suite is getting expensive.
- Some primitive scenarios exist for regression and completeness rather than for
  the default run. That is fine. Completeness is valued — don't delete them, tag
  them.

---

## 5. Confirm the red

Run the suite. Confirm the new tests fail **for the right reason** — a real
assertion failure, or a panic from a named stub — not a typo, a missing import, or
a compile error masquerading as red. Confirm the rest of the suite is still green.

---

## 6. Report

Follow the report format in `cycle-contract`, and append it to the round log.
Additionally state:

- the test file path(s) and the name of each currently-failing test, with a
  one-line reason each;
- the skeleton you created: every path, type and signature Green is expected to
  fill, listed exactly;
- the commands for running this round's tests, and the full suite;
- whether you restructured the slice, and why;
- which tests are durable scenarios and which are disposable unit tests;
- confirmation the rest of the suite is green.
