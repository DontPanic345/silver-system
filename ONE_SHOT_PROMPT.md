# One-shot kickoff prompt

Not run yet. Paste this verbatim to start the next experiment — see
`JOURNAL.md`'s 2026-09-06 entries for why it's shaped this way (a
calibration baseline, deliberately given no stated goal up front, run on the
project's yardstick model rather than a stronger one, so later structure
gets judged against this and not against "a smarter model also would have
done fine").

---

> Pick a small thing to build for `silver-system` that could plausibly serve the vision laid out in `NORTH_STARS.md` eventually, and build it using your own judgment for everything else — including whether the existing Rust substrate under `src/` (kept from a shelved experiment) is worth using, reworking, or ignoring entirely.
>
> When you stop, run `python3 scripts/session-metrics.py` and add one dated `JOURNAL.md` entry with its output plus a few honest bullets on what happened.

---

## Why this shape

- **Two sentences, on purpose.** Earlier drafts of this prompt grew a whole
  briefing (the Gnomes vision spelled out, the experiment history, a list of
  principles) before settling here. Building that up and then cutting it
  back down turned into its own small case of the thing this run exists to
  avoid — over-specifying the setup instead of just running the experiment.
  The final cut: point at `NORTH_STARS.md` and trust the agent to read it,
  rather than pre-digesting it.
- **Known tradeoff, accepted deliberately:** this makes the comparison
  against `night-shift` less clean than originally intended.
  `night-shift` got a fully fleshed `PLAN.md` (four tranches, intent
  statements, primitive scenarios) before any cycle process even started —
  this run gets a pointer to a doc and one word of scope ("small"). So this
  isn't purely isolating "process vs. no process" anymore; it's also
  carrying "richly specified goal vs. minimal goal" as a second variable. Worth
  knowing when reading whatever this run produces, not worth re-litigating
  by re-adding detail back — see `JOURNAL.md`'s 2026-09-06 entries for the
  actual back-and-forth.
- **"Small" stayed in deliberately** — it's the one word doing real
  scope-bounding work between an agent that freezes deciding what counts as
  in-scope and one that tries to over-reach at whatever it read in
  `NORTH_STARS.md`.
- **No cycle system** — night-shift's own retrospective found the process
  spending a large share of its budget on ceremony rather than the work
  (~12 lines of process log per line of shipped logic). This run is a
  control for that specifically, with the caveat above.
- **The metrics script's output is machine-derived, not self-reported** —
  pulled from the harness's own per-session `cost-state` tracking
  (`scripts/session-metrics.py`), so it's comparable run-to-run without
  trusting anyone's (including an agent's) sense of "that took a while." This
  is instrumentation, not process — it survived every round of cutting above
  on purpose, because without it there's no way to read the result of the
  experiment at all.
- **"A few honest bullets," not a report template** — deliberately doesn't
  prescribe a `## Debug notes` heading or a bullet checklist the way earlier
  drafts did. That structure traces back to stable-fluids' original
  `## Cycle debug` instruction (`stable-fluids/WATER_SIM_AC.md`
  §"Process instructions for `/tdd-cycle`"), which worked well there but is
  itself a small piece of process — worth reinstating explicitly only if
  this run's own free-form version turns out too thin or too padded to
  compare against later runs.
