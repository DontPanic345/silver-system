---
name: tdd-cycle
description: Run a Red → Green → Refactor TDD loop against a ticket's acceptance criteria, one test file per round. Gates on AC quality first and distills the ticket's actual intent so overspecified/incidental AC details don't get treated as gospel. Each phase runs as an isolated fresh agent (no memory of the others' reasoning) and only the AC list, intent statement, and minimal file-path handoffs pass between them; any phase can flag an AC/intent problem back to the orchestrator. Use when asked to implement a ticket test-first, or to run a TDD cycle.
user-invocable: true
---

# /tdd-cycle — orchestrated Red/Green/Refactor

You are the orchestrator. Your job is to drive the loop and pass the smallest
possible handoff between phases — never paste file contents or a phase's reasoning
into the next phase's prompt, and no need to re-verify what a phase already confirmed
itself.

## 0. Setup

Get the ticket's acceptance criteria (fetch from AzDO if given a ticket number, or
take them as given), plus the ticket description and parent ticket if one exists —
you'll need more than the literal AC bullets for the next step.

If you were handed only a plan, prompt, or rough description and no agreed
acceptance criteria, stop and run `/tdd-acceptance` first to produce a frozen AC
list and intent statement with the user, then resume here with that as input. Don't
invent the ACs yourself inside this cycle.

Determine whether this is the first round (no test file yet) or a continuation (an
existing test file to extend, e.g. because a phase flagged a gap last round).

## 1. AC quality gate

Before any code gets written, assess whether the ACs are actually dev-ready:

- Are they individually testable/verifiable — not vague ("works well", "handles
  errors gracefully" with no specifics)?
- Are they internally consistent — no bullet contradicts another?
- Do they cover what the ticket is actually asking for, or is something essential
  obviously missing?

If they're too vague, contradictory, or missing something essential to proceed,
**stop here and push back to the user** with specifics — what's unclear,
contradictory, or missing, and what you'd need to move forward. Don't guess, and
don't silently patch the ACs yourself.

Separately, distill the ticket's **intent** — the actual outcome for the end user,
in plain language, drawn from the ticket description and ticket, independent
of the literal AC wording. ACs are frequently a concretization of that intent that
can distract from it or overspecify incidental values (an exact pixel size, a
specific threshold) when what's actually wanted is softer ("looks natural", "feels
responsive"). Where you notice this, write the intent statement to say so explicitly
— e.g. "AC says 16px margin; treat as 'visually balanced spacing', not a hard
requirement" — so downstream phases know which details are load-bearing and which
are illustrative.

From here on, every phase gets **both** the AC list and this intent statement, not
ACs alone. The literal AC wording is a means, not the goal — when a phase has to
choose between satisfying literal AC wording and serving the stated intent, the
intent wins, and the phase should say so in its report rather than silently picking
one.

## 2. Red

Spawn a fresh agent (`Agent` tool — do not use `subagent_type: "fork"`, it inherits
context, which defeats the isolation) with a self-contained prompt: invoke the
`tdd-red` skill, giving it the AC list, the intent statement, and the existing test
file path if any.

Take its report — file path + failing test names — at face value. Don't read the
test file yourself or re-run it to confirm; Red already did that.

## 3. Green

Spawn another fresh agent. Prompt: invoke `tdd-green`, giving it the AC list, the
intent statement, the test file path, and the failing test names from Red's report.
Don't include anything else — no mention of Red's approach (you don't have it
anyway, since you only received its report, not its reasoning).

Take its pass/fail report at face value.

## 4. Refactor

Spawn another fresh agent. Prompt: invoke `tdd-refactor`, giving it the AC list, the
intent statement, and the file paths Green touched. Don't include Green's
implementation notes beyond the file paths — Refactor is meant to look at the
result cold.

Take its report: change list with rationale, any flagged gaps, final green
confirmation.

## 5. Handling AC/intent findings from any phase

Any phase — not just Refactor — may report back that it hit a flaw in the ACs or a
conflict with the stated intent, instead of (or alongside) its normal output: a
literal AC that's vague, contradictory, or that conflicts with the intent statement
in a way that changes what should be built. A phase should never quietly reinterpret
the ACs and plough on — it reports the finding and stops, and **you decide how to
proceed**, not the phase:

- If it's a minor interpretation call (e.g. confirms an illustrative detail really is
  illustrative), update the intent statement and re-run that phase.
- If it changes real scope, or is genuinely ambiguous, pause and ask the user rather
  than guessing.
- If it's a gap in coverage or behaviour (Refactor's usual case), feed it into the
  next round as new input, same as step 6 below.

## 6. Final gate

Run the test suite once yourself, after Refactor reports done. This is the only
point you independently verify anything — a single cheap check, not a re-audit of
each phase.

## 7. Loop or close out

- If a phase flagged a gap (or you resolved an AC/intent finding into new scope):
  that becomes the next round's input — go back to step 2 with it as the new slice
  to cover.
- Else if there's more of the AC list to cover: go back to step 2 for the next
  slice.
- Else (ACs fully covered, final gate passed): report the round(s) summary to the
  user and stop. Don't proceed to opening a PR or further work without being asked —
  that's a separate step (`/pr-description`, `/code-review`).

## Exit ramps

Each phase owns its own tooling/environment exit ramp (see the phase skills). If a
phase reports a tooling/environment failure instead of a normal result, don't retry
it yourself or attempt a workaround — stop the cycle and surface the exact failure
to the user.
