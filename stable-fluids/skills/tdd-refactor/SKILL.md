---
name: tdd-refactor
description: Behaviour-preserving refactor of the code and tests touched by a recent change, checked against a ticket's acceptance criteria and stated intent — tightens test coverage, removes redundant/brittle tests, and looks for holistic improvements (security, structure, forward-looking design) without expanding scope. Used as the third phase of the /tdd-cycle loop, or standalone against a recent diff.
user-invocable: true
---

# /tdd-refactor — improve without changing behaviour

Input you should have been given: the ticket's acceptance criteria, the ticket's
intent statement, and the source + test file paths touched by the recent change
(plus their direct callers/dependents, which you may read for context).

This phase doesn't know what implementation choices were just made or why — look at
the code cold, judged only against the ACs, the intent, and what's actually there.

## The scope rule

You may change **how** behaviour is expressed. You may not change **what** behaviour
is guaranteed. That's the boundary for every edit:

- **In scope**: renaming, extracting, moving, deduplicating, reorganising tests
  (splitting/merging/renaming), hardening existing logic (e.g. making implicit
  validation explicit), structural cleanup — as long as the same set of guarantees
  holds after the change as before.
- **Out of scope**: adding new behaviour, or fixing a gap by writing the missing
  logic/test yourself. If you find one, that's a new requirement, not a refactor —
  see "Gaps" below.

## Task, in order

1. **Coverage pass.** For each test touched, check it against the ACs and the
   intent: does it map to a guarantee that matters, or is it pinning down an
   incidental detail the intent statement already flagged as illustrative? Before
   merging or deleting any test, confirm what still covers that behaviour
   afterward — if nothing does, it wasn't redundant, it was coverage, and it stays
   (see "Gaps and AC/intent findings" if the coverage genuinely doesn't belong
   anywhere yet).
2. **Holistic pass.** Scoped to the files touched plus their direct
   callers/dependents — not a repo-wide sweep. Look for structural improvements,
   security hardening, and forward-looking concerns that fit naturally alongside
   this change. Fit in as much genuine improvement as reasonably belongs here; don't
   let it become an unrelated rewrite.
3. **Verify.** Re-run tests after each meaningful edit. A break caused by your own
   structural change (e.g. a rename you didn't finish propagating) is yours to fix —
   that's just doing the refactor correctly.

## Gaps and AC/intent findings (do not fix inline)

If a break, or something noticed during the coverage pass, reveals that the code or
tests never actually guaranteed something the ACs (or the intent) require — that's a
gap, not a refactor target. Don't author the missing behaviour or test yourself.
Stop, leave the code in a working (green) state, and report the gap specifically:
which AC, what's missing, what you saw. The orchestrator will spin up a fresh cycle
for it.

Same rule if you notice an AC that's vague, contradictory, or conflicts with the
stated intent — that's not yours to resolve by picking an interpretation. Report it
as a finding back to the orchestrator alongside your normal output, rather than
letting it silently shape which tests you keep or how you refactor.

## Exit ramp

Tooling/environment failures (broken hook, missing binary, permission error) are not
yours to fix or bypass — stop and report the exact command and error. If three
attempts at the same failure show no change, stop and report rather than continuing.

## Output

Report back:

- Change list, with a one-line rationale for each — especially deletions/merges of
  tests, referencing the AC (or lack of one) that justifies it
- Any gaps found, reported separately and clearly flagged as "needs a new cycle,"
  not folded into the change list
- Final confirmation the suite is green
