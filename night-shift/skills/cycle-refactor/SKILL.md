---
name: cycle-refactor
description: The Refactor phase of a round — reviewer, adversarial pass, quality gate and author of changes in one. Folds the round's new code back into the existing codebase, attacks it at the scope and focus it was given, and recommends whether the round's goals have been met or the round cycles again — the orchestrator decides what actually happens next. Runs at Sonnet with high effort. Use as the third phase of a round.
user-invocable: true
---

# cycle-refactor — fold it in, attack it, call it

Read `cycle-contract` first. You should have been given: the round's goals, the
round's and milestone's intent, **your scope and focus**, the round log path, and
the files Green touched.

Start your timer (`date -Is`).

You are deliberately overloaded. You are the reviewer, you are the adversarial
pass, you are the quality gate, and you also make the changes. What you are not is
a source of new features.

**You have 30 minutes to make this codebase better.** How to spend it is your
judgement. It is free time — use it.

---

## 0. Read it cold

You have no memory of Red's or Green's reasoning, and that is the point. The single
highest-value thing this phase has ever done was read shipped code cold and find
that a sign was inverted while every test passed. Read the code as though you've
never seen the argument that produced it.

---

## 1. Fold it in

Not "tidy the diff" — **reconcile the new material with the larger system.**

- Does the new code duplicate something that already exists, or nearly-duplicate it
  with a subtle difference?
- Does it belong where it was put?
- Does it follow the conventions of the code around it, or has it introduced a
  second way of doing something?
- Do names, types and boundaries still make sense now that this exists?
- Are the doc comments true? A comment describing both the old scheme and the new
  one is worse than no comment.
- Now that the system has changed, how should the system look?

Make the changes. Keep the tests green while you do it.

---

## 2. The adversarial pass

Work the **scope** and **focus** you were given. If the planner gave you a wide
scope because nothing risky changed, genuinely go wide — a narrow reading of a wide
brief wastes the round's best opportunity.

The isolated-phase design is very good at catching errors *in the diff under
review* and blind to **latent bugs in code nobody is touching**. Your adversarial
pass is the only thing that looks there. Angles worth taking, depending on focus:

- long runs — does it still hold after 5000 steps?
- resting states — does something that should be still stay still?
- sharp interfaces and boundary cells;
- symmetry — does a symmetric setup stay symmetric?
- conservation over time — mass, energy, count;
- performance against the milestone's targets;
- the test suite itself: is it testing what it claims, is anything tautological,
  is anything so slow it will stop being run?

Write throwaway probes. Delete them or promote them to real tests, your call.

**Test-suite runtime is your budget to defend.** Collapse redundant long runs. If
the default suite is getting expensive, fix it or flag it with a number.

---

## 3. Outside your scope isn't outside your reach

You are meant to be looking past the round's own diff — that's the point of the
adversarial pass. Two different things turn up out there, and they get different
treatment:

- **Mechanical, self-contained issues** — a clippy lint, a stale doc comment, a
  dead import, a stray `dbg!`, an obviously-wrong-but-trivial-to-fix line — fix
  them the moment you see them, wherever they are, whether or not they're in the
  files this round touched. Deferring a clippy warning just means paying to
  re-discover it later; fixing it now is cheaper than reporting it. Note what you
  fixed and where, same as any other change.
- **Shipped code that's substantively wrong** — a wrong sign, a wrong ordering, a
  conserved quantity that isn't, anything where the fix requires understanding
  intent or carries real risk of a further mistake. This is a first-class outcome,
  not an awkward edge case.
  - If it is inside your scope and you can fix it with evidence, fix it and say
    what the evidence was.
  - If it is outside your scope or too large for your remaining time, report it as
    a **correctness finding** with the evidence, and call for the round to cycle
    again (or for the planner to insert a corrective round). Do not let it pass
    because it didn't fit the round's story.

---

## 4. The recommendation

You answer one question explicitly, with reasoning:

> **Have the round's goals been met to a sufficient standard?**

One of:

- **Advance** — goals met, suite green, code folded in.
- **Cycle** — the round should run again with specific required updates. List them.
- **Back to planning** — the goal as framed looks wrong or unattainable, and no
  amount of cycling on it would help.

"Sufficient standard" is a judgement call and it is yours to make. Make it, and
show your reasoning — trust and verify.

This is a **recommendation**, not a command. The orchestrator decides what actually
happens next — it has cross-round memory you don't, and may see, for instance, that
this is the third round in a row on this same goal, which changes the right answer
even if your read of this round's code is right. Give it your honest read; don't
soften it to guess at what the orchestrator wants to hear.

---

## 5. Report

Follow the report format in `cycle-contract`, and append it to the round log.
Additionally state:

- the change list, each with a one-line rationale;
- what your adversarial pass tried, including what you tried that found nothing —
  that is information for the next planner;
- any correctness findings, with evidence;
- the current suite runtime and whether it is acceptable;
- **the verdict**, with reasoning;
- what you would have done with another 30 minutes.
