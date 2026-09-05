# night-shift — closeout

**Shelved 2026-09-05.** This was the `tranche → milestone → round → phase` cycle
system: `.claude/skills/cycle-*` (now `night-shift/skills/`), the tranche/milestone
plan (`PLAN.md`, now `night-shift/PLAN.md`), and every round's log
(`cycle-log/`, now `night-shift/cycle-log/`).

## What it was for

A hierarchy of self-dispatching agents — tranche, milestone, round, phase — each
with its own planner, meant to let the system adapt as it learned instead of
executing a fixed brief, and to catch mistakes through cold, isolated review
(Red/Green/Refactor) rather than one continuous agent grading its own work.

## What it proved

It worked, as far as it went. Tranche 0 (mathematics/tooling foundations) closed
out with all targets met. Tranche 1's first milestone (M1.1 — grid/material
substrate, scenario harness, headless measurement, a minimal wasm renderer, a
performance budget) closed out the same way: five targets, each independently
verified, real committed code. Compared to the two things it was implicitly
benchmarked against:

- **A single agent racing to finish a large brief in one pass** tends to declare
  victory early and skip the parts that don't show. This system's structural
  answer — a cold Refactor pass that owes nothing to Red's or Green's reasoning —
  is a real, working countermeasure. It caught things a continuous pass wouldn't.
- **[[stable-fluids]]**, the previous experiment, iterated test-first but had no
  real mechanism for updating its own plan mid-run: agents got confused about the
  task, re-derived context they should have already had, churned, and eventually
  halted on a structural problem (the Round 7 checkerboard mode) without a way to
  route around it. This system's planning layer — re-plan between every round,
  fold the last log forward, exit ramps back to planning instead of grinding — is
  what stable-fluids was missing, and it worked when exercised.

So: **a successful experiment**, on its own terms.

## What shelved it anyway

The whole apparatus — the vocabulary, the tranche/milestone/round/phase hierarchy,
the report format, the north star framing in `PLAN.md` and `CLAUDE.md` — was
drafted in one sitting from a single dictation dump and run for a full tranche
before anyone looked at it critically. (See the churn memory captured mid-run:
the user had warned beforehand against unconditionally running the full
Red/Green/Refactor triad, and that warning wasn't captured anywhere until tranche
0 had already proven it out the hard way.)

Measured near the end of tranche 1's first milestone:

- **~12 lines of process log per line of shipped production logic** (7,251 lines
  under `cycle-log/` against 614 non-comment, non-test lines in `src/`), and
  **~1.7 comment lines for every line of actual code** in that production code —
  a real, quantified "polished boilerplate" effect, not just an impression.
- Two background orchestration legs for a single milestone burned roughly 157k
  subagent tokens, ~41% of total account usage at the time, driven mostly by
  fixed cold-dispatch entry fees (every tranche/milestone/round-level agent
  re-reading `cycle-contract`, the relevant skill, `PLAN.md`, and prior logs
  before doing any actual work) rather than by the work itself.
- The system hit a real session usage limit one milestone into a seven-milestone
  tranche, one of four tranches — at that rate, the ambition in `PLAN.md` was not
  affordable under this shape.

The deeper finding, in the user's own words: **the cycle, the skills, and the
user's own upfront prompting were getting in the way of the model doing its best
work** — the opposite of the intended effect. A structure meant to make an agent's
judgement more reliable had, in practice, spent a large share of its budget on
its own ceremony, and had done so specifically because it was designed once,
sight unseen, rather than calibrated against a baseline.

## What comes next

Before designing another structure, run a **one-shot prompt** with no cycle,
no skills, and minimal upfront process instruction — keeping the Rust
substrate this experiment built (`src/`, `www/`, `tests/`; see the top-level
README) as the starting material, since it is itself mostly harmless output, not
process. That run is the new yardstick: whatever structure gets built after it
gets calibrated against what a comparatively unconstrained model actually does,
not against another single, unreviewed dictation dump.

The north star statements this experiment opened with (`PLAN.md`, `CLAUDE.md`)
are retired to [`NORTH_STARS.md`](../NORTH_STARS.md) along with every prior
experiment's, rather than carried forward as settled.

## Candidate fixes, if this shape ever gets revived

Named here so they aren't rediscovered from scratch:

- Collapse round-level cold dispatch into the milestone agent for single-pass
  rounds; reserve a genuinely fresh, isolated `Agent` call for rounds judged
  risky, where the isolation is actually earning its cost.
- Cap comment density in `cycle-green`/`cycle-refactor` — nothing in the report
  format or Refactor's brief currently asks for brevity, only for correctness and
  truthfulness, and Sonnet at high effort reliably over-documents without a
  counter-instruction.
- Trim the skill files themselves — every cold dispatch pays to read them in
  full, in every dispatch, for the life of the run.
