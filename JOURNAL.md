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

**2026-09-06 — Gnomes.** The user surfaced the actual capstone: a full game
concept, dictated 2026-09-05, that every experiment so far has actually been
building substrate for without anyone (including this session) knowing it.
Not "an emergent physics sim in the spirit of ONI" in the abstract — a
specific colony sim reskinning ONI's dupes as gnomes whose magic (bounded by
a "Gin" mana resource, regenerated by brewing/drinking) is the one sanctioned
exception to an otherwise strictly-conserved simulation, explicitly designed
to fix ONI's own named failures (no conservation, gravity-defying pipes,
gimmick gas behaviour, colony-failure-by-default). Directly explains a
standing mystery: "this is why I wanted such an overbuilt terrarium" — the
closed water cycle work traces to this, not abstract engineering taste.
Archived verbatim as `dictation-dumps/gnomes.md`, distilled in full as
`NORTH_STARS.md` #4. Not yet reflected in `CLAUDE.md`/`README.md`'s top-level
framing — asked the user whether it should be.

**2026-09-05 — Real gravity/density physics on the kept Rust substrate.**
Started the next experiment directly on `src/`'s existing grid/material/
scenario substrate (kept from `night-shift`, otherwise idle since that
shelving) rather than starting a new one from scratch — `Grid::step_once`
had been a deliberate no-op identity transform since M1.1, waiting for
exactly this. Replaced it with a real, generic movement rule driven entirely
by `Material::density`/`Phase` data (never a per-material `if` chain): a
denser cell may swap into a strictly-less-dense, non-`Solid` neighbour, tried
in priority order (straight down, diagonal-down, then — liquids only —
sideways). `Phase` grew a `Granular` variant (falls and piles, e.g. sand)
distinct from immovable `Solid` (e.g. stone); `MaterialTable::reference`
grew sand as a fourth material. Every move is a swap of two cells' contents,
never a creation/deletion, so per-material cell counts are exactly
conserved by construction — proven directly by tests, not just argued.

Hit and fixed one real physics bug along the way: a naive "swap into any
less-dense same-row neighbour" rule made a liquid column *oscillate* between
two symmetric open containers forever (the whole column translates as a
rigid block each step, then translates right back when the tie-break side
flips next step) rather than levelling. Fixed by gating horizontal liquid
flow on column occupancy (`Grid::column_count`, read progressively from the
in-progress step so it reflects swaps already applied earlier in the same
pass): a liquid cell may only flow sideways toward the column currently
holding *strictly fewer* cells of its own material. That one condition turns
"spreads out" into "spreads out and stops once level" — a cheap, real
approximation of hydrostatic equalization, not a screenshot that merely
looks flat once. Directly answers `NORTH_STARS.md` #3's callout: "a resting
pool stays flat" and "a column of water finds its level" are now both
pinned as exact, non-oscillating test assertions
(`src/grid.rs::a_sealed_resting_pool_stays_exactly_unchanged_under_gravity`,
`a_column_of_water_finds_its_level_across_an_open_container`), not just
aspiration.

Also: `mod timestep` promoted to `pub` — `Grid::step`'s own public signature
already took a `&mut FixedTimestep`, so no code outside this crate could
actually call it without that type being nameable; this had been latent
since `step` first went public and only surfaced now that something
(`src/bin/native_viewer.rs`) needed to call it from outside.

Wired the physics up as something watchable, not just headless-proven: a new
`scenario::physics_demo()` fixture (sealed stone container, a flat resting
water pool, five sand grains suspended above), a `step_and_paint_physics_demo`
wasm export driven by a real setInterval loop in the new `www/physics.html`,
and a real-browser e2e test (`tests/e2e/physics_demo.test.mjs`) that reads
actual canvas pixels after real wall-clock time passes rather than screenshot
review — the repo's own established headless-verification discipline
(`PRINCIPLES.md`'s "trust and verify"), applied to something that visibly
moves for the first time. `src/bin/native_viewer.rs`'s native fallback path
got the same fixture as a 4-frame PNG sequence for a cheap visual sanity
check. All three e2e tests (`canvas_rectangle`, `scenario_canvas`,
`physics_demo`) and the full `cargo test` suite pass; `cargo clippy
--all-targets -- -D warnings` is clean.

Left for later, in the phased physics→chemistry→biology→game-layer ordering
(`NORTH_STARS.md` #2/#3): gas movement/buoyancy (currently immobile
background, displaceable but not itself a mover), temperature, pressure as
its own tracked quantity (the current liquid-levelling rule is a cellular
approximation, not a real pressure solve), and everything past physics.
`CLAUDE.md`/`README.md` updated to describe this as the current experiment.

## Session metrics (967f66e9-d433-446a-81ac-0f0adade057c.jsonl)

- Wall-clock span (first→last transcript event): 2026-09-05T05:29:54.896Z → 2026-09-05T08:58:50.157Z
- No `cost-state` event found in this transcript; falling back to totals derived directly from the transcript's own assistant-message `usage` blocks (see `scripts/session-metrics.py`'s module doc comment for why, and what this fallback can't reconstruct — duration/model/tool-time split and lines added/removed aren't available this way).
- Token usage by model (derived, deduplicated by message id, cost not computed): `claude-sonnet-5`: 210 in, 79,847 out, 39,413 thinking, 14,258,313 cache-read, 314,587 cache-created.
- `git diff --stat` against the branch point: 8 files changed, 809 insertions(+), 90 deletions(-), plus 2 new untracked files (`tests/e2e/physics_demo.test.mjs`, `www/physics.html`).

A few honest bullets:

- The physics rule's first draft compiled and passed every test I'd written
  for it — including the water-levelling one, which failed exactly the way
  the debug trace predicted (rigid oscillation) once I actually ran it and
  looked at the printed grid state frame by frame, not just the pass/fail
  line. Worth remembering: a plausible-sounding cellular rule can pass
  "does it compile and do something" while being wrong in a way only a
  concrete trace reveals.
- Chose to build directly on the existing (if idle) Rust substrate rather
  than start a new experiment from zero, since `CLAUDE.md` explicitly left
  it "kept as starting material" and the grid/material/scenario shape was
  already exactly what real physics needed — this felt like continuing
  night-shift's actual unfinished work (M1.2 onward) more than starting a
  fresh, differently-named experiment, and I didn't rename it as one.
- Deliberately scoped to physics only (gravity + density), matching
  `NORTH_STARS.md`'s own physics→chemistry→biology→game-layer ordering,
  rather than reaching for temperature/pressure/reactions in the same
  session — gas movement in particular is a visible, named gap (`Phase::Gas`
  doc comment) rather than a silent omission.
- Didn't touch `terrarium/`, `stable-fluids/`, or `night-shift/` — all still
  shelved, referenced only for context.
