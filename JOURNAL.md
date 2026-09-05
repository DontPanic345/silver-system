# Journal

A dated, append-only narrative of how this repo actually got where it is —
the connective thread `NORTH_STARS.md` doesn't carry, because that file only
records a stated goal and its retrospective once an experiment already ended.
This is written at (or near) the time a real decision happens, not
reconstructed afterward from commit messages.

Entries before 2026-09-06 are a backfill from git history — see each entry's
note on how confident it is. Entries from 2026-09-06 on are written live.

---

**2026-08-28 — Jekyll blog scaffold.** The repo's actual first commit: a
Jekyll "bug-fix blog" scaffold (`_config.yml`, `Gemfile`, `index.md`,
`_posts/`), deployed to GitHub Pages the same day. *(Backfilled from git log
alone — no record survives of why a blog was the starting point, or what it
was even for. If you're reading this wondering the same thing: the reasoning
is genuinely gone, not just buried.)*

**2026-08-31 — Pivot to a falling-sand simulation.** Three days later,
`Add falling sand simulation` lands directly on top of the blog scaffold,
followed same-day by a rework from single-material cells to mixture cells.
*(Backfilled — no recorded reasoning for the pivot away from the blog
either.)*

**2026-08-31/09-01 — Terrarium.** Same day as the mixture-cell rework, a
"terrarium roadmap handoff doc" is added, and the sim grows a sealed glass
jar, a day/night light cycle, and (next day) a closed water cycle. The blog
scaffold is finally dropped once the terrarium is confirmed live on Pages.
Shelved 2026-09-02 as a successful test — see `NORTH_STARS.md` #1.

**2026-09-02 — Stable-fluids.** Terrarium shelved same day a Stam-style
stable-fluids prototype starts (grid solver, mouse-driven dye), then grows
test-first over seven planned rounds (conservation-checked advection,
temperature, buoyancy, phase change). Round 7 stops on a structural limit
(checkerboard pressure-projection mode) — see `NORTH_STARS.md` #3 for the
full retrospective.

**2026-09-05 — night-shift.** The stable-fluids halt becomes the
retrospective that shapes the next attempt: a `tranche → milestone → round →
phase` self-planning cycle system, in Rust this time, meant to run
indefinitely across four tranches without a human re-briefing it each step.
Tranche 0 (toolchain) and milestone M1.1 (grid/material/scenario substrate)
both close out clean. Shelved same day the session hit a real usage-budget
wall one milestone into a seven-milestone plan — the process itself was
found to be spending a large share of its own budget on ceremony rather than
the work (~12 lines of process log per line of shipped logic). Full
retrospective: `night-shift/CLOSEOUT.md`, `NORTH_STARS.md` #4.

**2026-09-05 — Cleanup pass.** Stripped the shelved cycle-system's own
vocabulary (milestone/round numbers, `cycle-log` paths, Green/Refactor
phase language) out of the kept Rust code's comments, since it was about to
outlive the process that produced it and would otherwise read as still-live
process rather than history. Confirmed via `cargo test --lib` that this
touched no logic. Rust substrate (`src/`, `www/`, `tests/`, `scripts/`) kept
as starting material for whatever runs next.

**2026-09-06 — This file.** Realized mid-conversation that the arc above
(blog → falling-sand → terrarium → vision doc → stable-fluids → night-shift)
was reconstructable only by reading raw git log, not written down anywhere
as a narrative — and that the very first pivot's reasoning had already been
lost for good. Started this journal so the next pivot doesn't disappear the
same way. Drafted a one-shot kickoff prompt for the next experiment
(deliberately with no north star handed to it up front, per
`NORTH_STARS.md`'s own "Next" section) — not yet run.

**2026-09-06 — Metrics for the one-shot prompt.** stable-fluids had a
`## Cycle debug` instruction appended to every phase prompt (see
`stable-fluids/WATER_SIM_AC.md`) that worked well: a handful of blunt
bullets, collected at close-out into one improvement summary. Under
night-shift it drifted into an unfiltered, everything-that-happened log
instead. Rebuilt it in two parts for `ONE_SHOT_PROMPT.md`: a
`## Session metrics` section pulled verbatim from the harness's own
per-session `cost-state` tracking via the new `scripts/session-metrics.py`
(wall-clock time, model/tool time, token usage, cost, lines changed — no
self-reporting, so it's actually comparable across experiments), plus a
`## Debug notes` section that reinstates stable-fluids' original bounded,
blunt-bullets discipline for whatever isn't quantifiable.
