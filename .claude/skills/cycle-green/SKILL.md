---
name: cycle-green
description: The Green phase of a round — make Red's failing tests pass by filling in the skeleton, and nothing else. Use as the second phase of a round.
user-invocable: true
---

# cycle-green — make it pass

Read `cycle-contract` first. You should have been given: the round's goals, the
round's and milestone's intent, your scope and focus, the round log path, the test
file path(s), and the names of the failing tests and the stubs Red left for you.

Start your timer (`date -Is`).

---

## 1. Make it pass

Fill in the skeleton Red left. The signatures, paths and parameter order are
already decided — take them as given. If one of them genuinely cannot work, change
it, and say so loudly in your report; don't silently reshape the API Red told the
round it was building.

Implement against the **intent**, using the goals as its concrete restatement. Where
the intent flags a number as illustrative, serve the intent.

**Actually run the tests.** Run the test binary and read its output. Importing a
module, compiling it, or reasoning that it should pass is not running it.

---

## 2. Rules that exist because they were broken before

- **Start from constants the tests already prove.** If a working value for a
  coefficient exists anywhere in the suite, start there. Don't re-guess a magic
  number from scratch — first guesses have driven physics the wrong way and leaked
  more than half the mass in past rounds.
- **Don't hide a cost in a knob.** If the honest fix is out of scope, say the
  honest fix is out of scope. Cranking an iteration count, widening a tolerance, or
  inventing a constant that papers over a defect is how a structural bug survives
  four rounds of green suites.
- **Propose a number, don't loosen silently.** If a threshold in a test is wrong,
  do not weaken it to make the failure disappear. Report the measured value, the
  evidence, and the threshold you propose instead — and leave the test failing if
  you can't justify the change.
- **No fictional constants.** A name that claims something ("the shipped grid
  size") must be true.

---

## 3. Self-diff before you finish

Read your own diff, start to finish, once. It takes a minute and it reliably
catches:

- dead compute — something calculated every step and then overwritten;
- doc comments describing the old scheme alongside the new one;
- a leftover experiment, a stray `dbg!`, a commented-out branch;
- anything you added while chasing a bug and no longer need.

Refactor will find these. Finding them yourself is cheaper.

---

## 4. Stay in your lane

Green adds behaviour to make the round's tests pass. It does not restructure the
codebase, rename things at large, or improve neighbouring code — that is Refactor's
budget, and it is protected on purpose. Note what you'd have changed; don't change
it.

---

## 5. Report

Follow the report format in `cycle-contract`, and append it to the round log.
Additionally state:

- every file you touched;
- the exact command you ran and its result — pass counts, failure counts, output
  for anything still failing;
- any signature you had to change from Red's skeleton, and why;
- any number you propose changing, with the measurement behind it;
- what you noticed but deliberately left for Refactor.

If a test is still failing when you stop, say so plainly with the output. A round
that reports an honest failure is worth more than one that reports a green suite
it achieved by moving the goalposts.
