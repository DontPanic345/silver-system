---
name: tdd-green
description: Implement the correct solution to make specific named failing tests pass, against a ticket's acceptance criteria and stated intent. Used as the second phase of the /tdd-cycle loop, or standalone once a failing test exists.
user-invocable: true
---

# /tdd-green — make it pass

Input you should have been given: the ticket's acceptance criteria, the ticket's
intent statement, a test file path, and the specific test name(s) currently failing.
You have not been given — and shouldn't need — the reasoning behind how that test
was written; work from the ACs, the intent, and the test's own content.

## Task

1. Read the failing test(s), the ACs, and the intent statement.
2. Implement the solution, following the repo's existing conventions and best
   practices. Build it to actually satisfy the requirement — not just to defeat the
   specific assertions in front of you. Where the test or an AC pins down an
   incidental detail the intent statement flags as illustrative, satisfy the intent;
   don't over-index on a literal number that isn't actually load-bearing.
3. Run the full suite until the named tests, and everything else, pass.

## If the ACs or intent don't hold up

If implementing reveals an AC that's vague, contradictory, or conflicts with the
stated intent in a way you can't resolve by just building the sensible thing — don't
guess at what was meant. Stop and report it as a finding back to the orchestrator
instead of (or alongside) your normal output.

## Exit ramp

If a run fails for tooling/environment reasons unrelated to your code changes
(broken hook, missing binary, permission error), stop and report the exact command
and error rather than working around it. If three attempts at the same failure
produce no change in the failure signature, stop and report rather than continuing
to iterate.

## Output

Report back only:

- Pass/fail status of the full suite
- Files touched
