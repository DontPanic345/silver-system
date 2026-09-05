# One-shot kickoff prompt

Not run yet. Paste this verbatim to start the next experiment — see
`JOURNAL.md`'s 2026-09-06 entries for why it's shaped this way (a
calibration baseline, deliberately given no stated goal up front, run on the
project's yardstick model rather than a stronger one, so later structure
gets judged against this and not against "a smarter model also would have
done fine").

---

> You're picking up work in `silver-system`. Here's the briefing — it's everything you need to start; the files it points to have more depth if you want it, but reading them isn't a prerequisite:
>
> This repo is heading toward a specific game, **Gnomes**: a colony sim fixing Oxygen Not Included's own named failures (no real mass/energy conservation, liquids defying gravity through pipes, gimmick gas/liquid behaviour, colony-failure-by-default) by reskinning its "dupes" as gnomes whose magic is the *one sanctioned exception* to an otherwise strictly-conserved simulation — bounded by a brewed mana resource called Gin, with death replaced by an escape to an "ethereal layer" rather than colony collapse. Nobody's built toward this directly yet; every experiment so far (a browser terrarium with a closed water cycle, a Stam stable-fluids solver that got as far as buoyancy before halting on a pressure-solver limitation, a Rust `tranche/milestone/round/phase` self-planning cycle that proved out some good ideas but spent too much of its own budget on ceremony) has instead built substrate this could eventually sit on — see `JOURNAL.md` for the real dates and specifics if you want them, and `NORTH_STARS.md` #4 for the fuller Gnomes distillation. Kept from the last of those: `src/`, `www/`, `tests/`, `scripts/` — a Rust wasm/canvas substrate (grid, material, timestep, scenario, headless measurement, rendering); `README.md` has build/test commands if you use it.
>
> Four working principles, earned the hard way (`PRINCIPLES.md` has the reasoning): believability comes from the rules underneath being genuinely full, not from what's shown on screen. The only way to go fast is to go well. Do one thing at a time. Trust and verify.
>
> There is no stated goal handed to you for this run. Decide what small physical system to build next — it doesn't have to serve Gnomes directly, but it shouldn't contradict it either (e.g. don't bake in unconserved mass/energy as a shortcut). Use the kept Rust substrate if it's a genuine fit for what you decide to build; rework or discard any part of it that isn't. Don't preserve it out of inertia, and don't feel obliged to keep working in Rust if you decide it's the wrong fit.
>
> No cycle system, no imposed process, no milestone/round structure. Work the way a competent engineer would on their own: plan briefly if it helps you, then build, test, and verify as you go. This repo's convention is headless verification — numbers or JSON a test can assert on, not screenshots. Commit as you make real progress, with real commit messages, on a branch off `main`.
>
> When you stop — because the system feels done, you hit a real structural wall, or you're deliberately pausing to hand it back for review — add a dated entry to `JOURNAL.md` covering what you built, why you stopped there, and whatever you'd want the next person (or the next you) to know before continuing. Add to `NORTH_STARS.md` only if you land on a genuine aspirational statement worth keeping (not the same thing as this run's tactical goal), and to `PRINCIPLES.md` only if you land on a real, reusable aphorism — most runs will touch neither. Update `CLAUDE.md`'s "Current experiment" section either way.
>
> Before writing that entry, run `python3 scripts/session-metrics.py` and paste its output verbatim into it, under a `## Session metrics` heading — this is quantitative, harness-derived data (wall-clock time, model/tool time, token usage, cost, lines changed), not something to restate in prose or round off. Then add a `## Debug notes` section in your own words: a handful of blunt bullets, not a transcript. Cover whatever applies — what took disproportionately long and why, anything unclear or missing that you had to work around, mistakes you backed out of, anything you were unsure was in your remit. If nothing notable happened, say so in one line rather than padding. These two sections exist to be compared against other experiments' entries later, so keep them short enough that they actually get read.

---

## Why this shape

- **No tactical goal up front** — the run gets the aspirational thread as a
  paragraph in the briefing, not a scoped target the way `night-shift`'s
  `PLAN.md` handed one down. The point of this run is to be the yardstick,
  not to hit a target someone else already picked.
- **The briefing is self-contained, not a reading list** — five files
  (`CLAUDE.md`, `README.md`, `JOURNAL.md`, `NORTH_STARS.md`, `PRINCIPLES.md`)
  add up to real weight before any code gets written, and that's its own
  kind of ceremony, the same failure mode this run exists to avoid. The
  prompt now carries the essential facts inline; the files stay linked for
  whoever wants the full history, not required reading to start.
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
