---
name: tdd-red
description: Write (or extend) one failing test file for the next slice of behaviour, against a ticket's acceptance criteria and stated intent. Used as the first phase of the /tdd-cycle loop, or standalone to add a failing test for a specific AC.
user-invocable: true
---

# /tdd-red — write the failing test

Input you should have been given: the ticket's acceptance criteria, the ticket's
intent statement (or the specific slice of them for this round), and the path to an
existing test file if you're extending one rather than starting fresh.

## Task

1. Pick the next slice of behaviour to cover — enough to be meaningful, small enough
   to land in one round. If extending an existing test file, add to it; otherwise
   create one at the conventional path/name for this repo's test layout (check
   nearby test files for the convention).
2. Write the test(s) for that slice against the **intent**, using the ACs as the
   concrete restatement of it — not against a guess at implementation. Where the
   intent statement flags an AC detail as illustrative rather than load-bearing
   (e.g. an exact pixel value standing in for "looks natural"), test the intent, not
   the literal number. A test's name and structure should read as a restatement of
   the behaviour it guards, since Green and Refactor will only ever see the ACs,
   the intent statement, and this test, never your reasoning.
3. Run the suite. Confirm the new test(s) fail for the right reason — a real
   assertion failure against missing/wrong behaviour, not a typo, import error, or
   syntax error masquerading as red.

## If the ACs or intent don't hold up

If, while writing the test, you find an AC that's vague, contradictory, or
conflicts with the stated intent in a way that changes what should be tested —
don't guess or silently pick an interpretation. Stop and report it as a finding
back to the orchestrator instead of (or alongside) your normal output.

## Exit ramp

If the run fails to even execute — a tooling/environment problem (broken hook,
missing dependency, config issue, permission error) rather than the test itself —
stop and report the exact command and error. Don't attempt to fix or bypass tooling
issues; that's out of scope here. Same if three attempts to get a clean red don't
change the failure signature at all — stop and report rather than keep grinding.

## Output

Report back only:

- Test file path
- Name(s) of the specific test(s) that are currently failing, with a one-line reason
  each
- Confirmation the rest of the suite is still green
