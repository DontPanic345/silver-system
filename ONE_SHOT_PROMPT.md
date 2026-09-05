# One-shot kickoff prompt

Not run yet. Paste this verbatim to start the next experiment — see
`NORTH_STARS.md`'s "Next" section and `JOURNAL.md`'s 2026-09-06 entry for why
it's shaped this way (a calibration baseline, deliberately given no stated
goal up front, run on the project's yardstick model rather than a stronger
one, so later structure gets judged against this and not against "a smarter
model also would have done fine").

---

> You're picking up work in `silver-system`. Start by reading `CLAUDE.md`, `README.md`, and `NORTH_STARS.md` in full — they tell you what this repo is, what's been tried before and why each attempt was retired, and what's kept as reusable starting material (`src/`, `www/`, `tests/`, `scripts/`: a Rust wasm/canvas substrate — grid, material, timestep, scenario, headless measurement, rendering — from a shelved experiment).
>
> There is no stated goal handed to you for this run. Decide what small physical system to build next — something that produces emergent behaviour from simple interacting rules, in the spirit of Oxygen Not Included/Noita — and then build it. Use the kept Rust substrate if it's a genuine fit for what you decide to build; rework or discard any part of it that isn't. Don't preserve it out of inertia, and don't feel obliged to keep working in Rust if you decide it's the wrong fit.
>
> No cycle system, no imposed process, no milestone/round structure. Work the way a competent engineer would on their own: plan briefly if it helps you, then build, test, and verify as you go. This repo's convention is headless verification — numbers or JSON a test can assert on, not screenshots. Commit as you make real progress, with real commit messages, on a branch off `main`.
>
> When you stop — because the system feels done, you hit a real structural wall, or you're deliberately pausing to hand it back for review — add a dated entry to `JOURNAL.md` covering what you built, why you stopped there, and whatever you'd want the next person (or the next you) to know before continuing. Update `NORTH_STARS.md` if this experiment gets retired, and `CLAUDE.md`'s "Current experiment" section either way.
>
> Before writing that entry, run `python3 scripts/session-metrics.py` and paste its output verbatim into it, under a `## Session metrics` heading — this is quantitative, harness-derived data (wall-clock time, model/tool time, token usage, cost, lines changed), not something to restate in prose or round off. Then add a `## Debug notes` section in your own words: a handful of blunt bullets, not a transcript. Cover whatever applies — what took disproportionately long and why, anything unclear or missing that you had to work around, mistakes you backed out of, anything you were unsure was in your remit. If nothing notable happened, say so in one line rather than padding. These two sections exist to be compared against other experiments' entries later, so keep them short enough that they actually get read.

---

## Why this shape

- **No north star up front** — the point of this run is to be the yardstick,
  not to hit a target someone else already picked.
- **No cycle system** — night-shift's own retrospective found the process
  spending a large share of its budget on ceremony rather than the work
  (~12 lines of process log per line of shipped logic). This run is the
  control for that: same repo, same kind of goal, none of the process.
- **`## Session metrics` is machine-derived, not self-reported** — pulled
  from the harness's own per-session `cost-state` tracking
  (`scripts/session-metrics.py`), so it's comparable run-to-run without
  trusting anyone's (including an agent's) sense of "that took a while."
- **`## Debug notes` is deliberately bounded** — this is stable-fluids'
  original `## Cycle debug` instruction (see `stable-fluids/WATER_SIM_AC.md`
  §"Process instructions for `/tdd-cycle`"), reinstated after it drifted into
  an unfiltered everything-log under night-shift. "A handful of bullets, be
  blunt, say so in one line if nothing happened" is the whole point — it's
  useful *because* it's short enough to actually read across several
  experiments, not because it's complete.
