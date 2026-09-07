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

**Entries older than the current and previous experiment are kept short,
deliberately.** Every night this file is read cold, in full, with no cache
carried over from the night before (see `NIGHTLY.md`) — so its size is a
real, recurring cost, not just a readability concern. Full detail on
anything summarized below is still in git history (`git log`, `git show
<commit>`) and in the shelved experiments' own retrospectives, verbatim and
un-lost; compressing the live copy here loses nothing, since nothing is
actually being deleted from the repo, only shortened where it's read.

---

**2026-08-28 — Jekyll blog scaffold.** The repo's first commit, for reasons
no longer recoverable (backfilled from git log alone; the blog itself was
dropped 2026-09-02).

**2026-08-31 — Falling-sand pivot, then terrarium.** Pivoted from the blog to
a falling-sand sim, then the same day grew into a sealed-jar terrarium goal:
*"A small system that can run itself indefinitely once sealed."* Shelved
2026-09-02 as a successful test — jar, light cycle, and a closed, conserved
water cycle all landed; plants (the real design risk) never started. Kept
under `terrarium/`.

**2026-09-02 — Stable-fluids.** A Stam-style stable-fluids prototype, test-
first over seven planned rounds, chasing a physically-driven water cycle
(boil → rise → condense → rain, conserved). Rounds 1–6 passed; round 7 hit a
real structural wall — a checkerboard null mode in the colocated pressure
projection needing a staggered/compact `project()` rewrite nobody returned
to build. Retrospective, and the source most of night-shift's design came
from: `stable-fluids/tdd-cycle-closeout.md`.

**2026-09-05 — night-shift, and its cleanup.** A self-planning
`tranche → milestone → round → phase` cycle system, dictated in
`dictation-dumps/agentic-development.md` in response to stable-fluids'
retrospective, meant to run for four tranches with no human re-briefing.
Tranche 0 and milestone M1.1 (the grid/material/scenario substrate this repo
still builds on) closed out clean; shelved the same day it hit a real
usage-budget wall one milestone into seven — the process itself was
spending more of its own budget on ceremony than the work (**~12 lines of
process log per line of shipped logic**). The goal wasn't wrong; the process
built to chase it was. Cleaned the cycle system's own vocabulary out of the
kept Rust code's comments same day, confirmed via `cargo test --lib` that
this touched no logic. Full retrospective: `night-shift/CLOSEOUT.md`.

**2026-09-06 — This journal, and its supporting docs.** Started this file
because the arc above was reconstructable only from raw git log, not written
down anywhere. Split what had been one conflated `NORTH_STARS.md` three
ways: aspiration stays in `NORTH_STARS.md`, concrete history moves here, and
a new `PRINCIPLES.md` holds the aphorisms. Drafted (but didn't yet run)
`ONE_SHOT_PROMPT.md` with its own `session-metrics.py`-based instrumentation
— since superseded by `NIGHTLY.md`. Also archived two real voice dictations
verbatim under `dictation-dumps/` after finding a NORTH_STARS.md entry had
been reconstructed secondhand rather than sourced from them directly —
nobody needs to read the dumps routinely; they exist for provenance.

**2026-09-06 — Gnomes.** The user surfaced the actual capstone every prior
experiment had unknowingly been building substrate for: not "an emergent
physics sim in the spirit of ONI" in the abstract, but a specific colony sim
reskinning ONI's dupes as gnomes, whose magic (bounded by a "Gin" mana
resource) is the one sanctioned exception to an otherwise strictly-conserved
world — explicitly designed to fix ONI's own named failures (no
conservation, gravity-defying pipes, gimmick gas, colony-failure-by-default).
Archived verbatim as `dictation-dumps/gnomes.md`, distilled in full as
`NORTH_STARS.md` #4.

**Night 1/7 — 2026-09-07 — Opus 5/low — Gnomes, built.** *(Numbered retroactively when
`NIGHTLY.md`'s seven-night compounding run was formalized: this was the
first run that extended rather than reset the substrate, which is what
qualifies it as night 1 of the real sequence, not just a date it happened
to land on. Also restored 2026-09-07: this entry was lost
during manual conflict resolution when `experiment/effort-opus-low` merged
into `main` — both branches had appended at the same point in the file, and
only one side survived the resolution. Recovered verbatim from the branch;
see `effort-level-experiment/README.md`'s `opus-low` section for the
independent verification this run got before merging.)* The first
experiment to aim at `NORTH_STARS.md` #4 directly instead of building
substrate underneath it. Stated goal: *build as much of the north stars as
one session can confidently carry*, with no process cycle at all — a
deliberate contrast with `night-shift`, whose closeout blamed ceremony
rather than the goal. What landed, on top of the kept Rust substrate:

- **A real simulation.** `src/world.rs` replaces the material-only `Grid`
  with cells carrying mass, temperature and latent-change progress;
  `src/physics.rs` adds density-ordered movement, symmetric heat
  conduction, and data-driven phase change. Mass and energy are conserved
  *by construction* — movement is a swap, conduction is a clamped pairwise
  transfer, phase change is an algebraic rewrite — and asserted to `1e-6`
  relative over 600–4000-step runs, not merely hoped for.
- **The Gnomes game layer** (`src/gnome.rs`): Gin as a bounded mana
  resource, magic as the one accounted-for hole in the world's books,
  the ethereal layer instead of death, gnome-to-gnome rescue, juniper
  foraging, and ethereal pipes implemented as a two-cell swap so that even
  the sanctioned shortcut cannot create matter.
- **The terrarium** (`src/terrarium.rs`), a running water cycle, plus a
  browser view (`www/terrarium.html`) and a headless JSON runner
  (`cargo run --bin terrarium`) that report the same numbers.

Four findings worth keeping, each recorded in the code where it bites:

1. *A sealed jar dies.* The first terrarium was fully closed and reached
   thermal equilibrium in a couple of minutes of simulated time — correct
   physics, no cycle. Fixed by giving it a declared hot vent and cold lid
   whose flux goes through the same ledger gnome magic uses, so the jar's
   openness is a number rather than a fudge.
2. *Latent heat cannot be a threshold flip.* Melting has to accumulate
   energy at the transition point, or a cell must overshoot melting point
   by 160 K to pay for its own latent heat, and then oscillates.
3. *Falling and spreading must be separate passes.* Combined, a liquid
   slides into the hole the cell above it was about to fall through, and a
   shallow pool never fills its bottom row.
4. *Communicating vessels need a body-level rule.* Cell-local gravity
   cannot climb the far arm of a U-bend. `stable-fluids` split on the
   pressure solve; this sidesteps it by transferring a surface cell from
   the tallest column of a connected cavity to a lower one, which is the
   only consequence of the pressure field that this scenario needs.

Not built: brewing/distilling (Gin comes from berries only), the
book-copying knowledge economy, the Gnome Grandmother, buildings, and
farming. The physics still has no pressure or gas diffusion, and gases do
not spread laterally at all (deliberately — see `physics::apply_gravity`).
Also not yet reconciled: `src/grid.rs`'s material-only `Grid` (below) and
`src/world.rs`'s mass/temperature `World` are now both live, solving
overlapping problems — an open duplication, not a decision that both should
stay.

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

*(Raw session metrics for this run — wall-clock, token usage by model,
diff-stat — trimmed from the live copy of this entry 2026-09-07 to keep the
file lean for nightly reads; recoverable via `git show
d6d2139:JOURNAL.md` or the original transcript
`967f66e9-d433-446a-81ac-0f0adade057c.jsonl` if ever needed.)*

One finding worth keeping past the trim: the first draft of the physics rule
compiled and passed every test written for it — including the water-
levelling one, which only failed the way a debug trace predicted (rigid
oscillation) once actually run and watched frame by frame, not just
pass/fail. A plausible-sounding cellular rule can pass "does it compile and
do something" while being wrong in a way only a concrete trace reveals —
the same lesson `xhigh` re-taught, more expensively, in the effort-level
experiment below.

**2026-09-07 — The effort-level experiment, written up.** Follow-on from
this file's own prompt-3 entry above: re-ran that same prompt five more
times, once per `effort:` setting (`low`/`medium`/`high`/`xhigh`/`max`), each
via a dedicated Sonnet subagent on its own branch off the shared baseline
(`b6a30ea`), in series, left unmerged for review. Full background, the four
original ambition-wording prompts and their results, a verified per-level
token-usage table, every branch's own `JOURNAL.md` entry verbatim, and notes
on each (including one live-observed false claim `cargo test`/`clippy`/e2e
all missed) are in [`effort-level-experiment/README.md`](effort-level-experiment/README.md)
rather than duplicated here. Branches: `experiment/effort-low`,
`-medium`, `-high`, `-xhigh`, `-max`. A sixth run added a model-choice
dial (`experiment/effort-opus-low`, Opus 5 at `effort: low`) — the cheapest
run of the six, and the only one to solve the U-tube case or attempt the
game layer at all; folded into the same write-up.



**2026-09-07 — Motivation for nightly-self directed.**
_I wanted to let the agent pick its own goals each turn. And aim
for what it could comfortably do, what felt natural for that model+effort.
They have been tuned and optimased better than my goal setting could ever help.
Each Model+effort has been optimised for its own token usege for the kinds of tasks 
that is targeted to do, and my assumption is that each will be able to do it's respecitve
size of work, more effecently than any other size. I want to min/max my weekly session
usage_

