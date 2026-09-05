# Journal

A dated, append-only narrative of how this repo actually got where it
is — what got tried, when, why it started, and why it ended. This is the
concrete history; `NORTH_STARS.md` holds the vague, aspirational statements
that motivated it (which don't retire the way an experiment does), and
`PRINCIPLES.md` holds the aphorisms distilled along the way. Written at (or
near) the time a real decision happens, not reconstructed afterward from
commit messages.

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
"terrarium roadmap handoff doc" (`terrarium/TERRARIUM_PLAN.md`) is added, and
the sim grows a sealed glass jar, a day/night light cycle, and (next day) a
closed water cycle. Stated goal: *"A small system that can run itself
indefinitely once sealed — not just a prettier sandbox."* The blog scaffold
is finally dropped once the terrarium is confirmed live on Pages. Shelved
2026-09-02 as a successful test: Phase 0 (jar + light cycle) and Phase 1
(closed water cycle, conserved by construction) both done; Phase 2 (plants —
the real design risk) never started. Kept under `terrarium/`.

**2026-09-02 — Stable-fluids.** Terrarium shelved same day a Stam-style
stable-fluids prototype starts (grid solver, mouse-driven dye), then grows
test-first over seven planned rounds. Stated goal: *"A closed water cycle
driven by physics — heat the water and it boils, the vapour rises, cools,
condenses, and rains back down. Mass and energy go round the loop and are
conserved to a good approximation."* Rounds 1–6 of 7 pass (conservation-
checked advection, temperature, buoyancy, phase change); round 7 stops on a
structural limit — a checkerboard null mode in the colocated pressure
projection that needed a staggered or compact `project()` rewrite nobody
returned to build. Retrospective: `stable-fluids/tdd-cycle-closeout.md`,
which is where most of night-shift's design came from.

**2026-09-05 — night-shift.** The stable-fluids halt becomes the
retrospective that shapes the next attempt — concretely: two days after that
halt, the user wrote up pointed questions on how the TDD cycle actually went
(`dictation-dumps/response-to-tdd-cycle.md`, 2026-09-03 — "maybe Red writes
skeleton code too," "who is top dog and can make the call," "would a blank
agent per AC have just been better?"), then dictated a full answer
(`dictation-dumps/agentic-development.md`, 2026-09-05) that became a
`tranche → milestone → round → phase` self-planning cycle system, in Rust
this time, meant to run indefinitely across four tranches without a human
re-briefing it each step.
Stated goal: *"A believable small world inside a large universe. A terrarium
people can see on their screens and interact with"* — see `NORTH_STARS.md`
#3, which this run restates sharply enough to be its own entry. Tranche 0
(toolchain) and milestone M1.1 (grid/material/scenario substrate) both close
out clean. Shelved same day the session hit a real usage-budget wall one
milestone into a seven-milestone plan — the process itself was found to be
spending a large share of its own budget on ceremony rather than the work
(~12 lines of process log per line of shipped logic, ~1.7 comment lines per
line of actual code). The goal wasn't wrong, and wasn't retired — the process
built to chase it was. Full retrospective: `night-shift/CLOSEOUT.md`.

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
same way. Drafted `ONE_SHOT_PROMPT.md` for the next experiment (deliberately
with no stated goal handed to it up front, kept the Rust substrate as
starting material) — not yet run.

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

**2026-09-06 — Split NORTH_STARS.md three ways.** Realized `NORTH_STARS.md`
had been conflating three different things: the vague, aspirational
statements the name actually implies (the GPU vision doc, night-shift's
two-line goal); the concrete, per-experiment stated-goal-and-why-it-ended
record that's this file's job; and aphorisms (night-shift's own
`## Principles` section, buried in a shelved `CLAUDE.md` and a shelved skill
file) that are neither aspiration nor history. Pulled the per-experiment
material into the entries above, rewrote `NORTH_STARS.md` down to just the
three aspirational fragments (each now with a pointer to its original
context — commit and file, or an honest note where no file survives), and
started `PRINCIPLES.md` to hold the aphorisms on their own. `README.md` and
`CLAUDE.md` updated to point at the right file for each kind of question.

**2026-09-06 — Archived the real dictation dumps.** The ink was barely dry
on the above when it turned out entry #3 was reconstructed from a downstream
copy (`night-shift/CLAUDE.md`) of a real source the user still had:
`Agentic Development.md`, an ~11KB voice dictation, plus
`response to tdd cycle.md`, the reflection two days earlier that shaped it.
Archived both verbatim under `dictation-dumps/` (typos and all) rather than
re-paraphrasing secondhand — corrected `NORTH_STARS.md` and `PRINCIPLES.md`'s
attributions to cite them directly, including catching that the
determinism-as-"architecture-contingent" walk-back was voiced *in this same
dump*, not between two unrelated documents as the previous draft implied.
Per the user: nobody should need to read these dumps routinely — they're
archived for provenance, with a genuine open task (noted in
`dictation-dumps/README.md`) to revisit all such material later for ideas
that never made it into a distillation the first time.
