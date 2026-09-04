---
name: tdd-acceptance
description: Turn a plan, prompt, design doc, or ticket stub into a frozen, dev-ready acceptance-criteria list plus an intent statement — the input `/tdd-cycle` expects at step 0. Collaborative by design: drafts AC, you refine, it locks. Use before a TDD cycle when you have an idea or plan but no agreed acceptance criteria yet.
user-invocable: true
---

# /tdd-acceptance — plan → acceptance criteria

You produce two artifacts the user signs off on: a **testable AC list** and an
**intent statement**. Together they are the handoff into `/tdd-cycle` (its step 0),
`/creating-tickets`, or a standalone TDD phase. Nothing downstream re-derives them —
so the sign-off here is the load-bearing moment.

This step is interactive. Draft, show, revise with the user, lock. Do not silently
decide what the ACs are and move on.

## 1. Gather the source material

Take whatever the user has: a plan, a prompt, a design doc, a rough ticket, a
paragraph of intent, links to related tickets/PRs. Read the linked material rather
than restating it. If a parent ticket or existing spec exists, read that too — you
need more than the one prompt to know what's actually wanted.

If there's genuinely not enough to work from (a one-line request with no context and
no obvious domain), ask the user for what's missing before drafting. Don't pad thin
input into fake precision.

## 2. Distill the intent

Write the **intent statement** first — the actual outcome for the end user, in plain
language, independent of any AC phrasing. This is the thing the ACs will be a
concretization of. Keep it short: what changes for the user and why.

Where the source material fixes an incidental value (an exact pixel size, a specific
threshold, a named colour) but what's really wanted is a quality ("looks natural",
"feels responsive", "visually balanced"), say so in the intent statement — e.g.
"source says 200ms; treat as 'feels immediate', not a hard number". Downstream
phases use this to tell load-bearing details from illustrative ones.

## 3. Draft the acceptance criteria

Turn the intent into a list where each bullet is:

- **Individually testable** — a concrete, verifiable statement, not "works well" or
  "handles errors gracefully" with no specifics. Someone should be able to write a
  test or click through a check from the bullet alone.
- **Behavioural, not implementational** — what the system does, observable from
  outside, not how it's built.
- **Internally consistent** — no bullet contradicts another.
- **Scoped to this change** — cover what the intent asks for; don't smuggle in
  adjacent work. Flag anything you think is missing from the user's plan as a
  question, not as an extra AC you invented.

Aim for the smallest set that fully covers the intent. Split a bullet that hides two
behaviours; merge bullets that only differ by an incidental value.

## 4. Review with the user

Show the intent statement and the AC list. Call out explicitly:

- Any bullet where you had to make an interpretation call, and what you assumed.
- Anything in the source material you deliberately treated as illustrative (with the
  intent-statement note).
- Any gap you noticed in the plan — as an open question for the user to answer.

Revise on their feedback. Repeat until they accept it.

## 5. Lock and hand off

Once the user signs off, restate the final frozen artifacts in one place:

- **Intent statement** — the plain-language outcome, plus any illustrative-detail
  notes.
- **Acceptance criteria** — the numbered list.

Then say what comes next and stop — don't start it yourself:

- `/tdd-cycle` to implement test-first against these.
- `/creating-tickets` to file them as an AzDO work item.

Do not write tests, code, or a ticket from here. This skill's output is the frozen
input those steps consume.
