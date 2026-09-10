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
*(Trimmed 2026-09-07 to match this file's own lean-entry policy — full
prose, test names, and raw session metrics recoverable via `git show
d6d2139:JOURNAL.md`.)* Replaced `Grid::step_once`'s deliberate no-op with a
real, generic density/phase-driven swap rule (never a per-material `if`
chain) directly on the kept `night-shift` substrate — a denser cell swaps
into a less-dense, non-`Solid` neighbour, down before diagonal before
(liquids only) sideways. `Phase` grew `Granular` (falls, doesn't level);
`MaterialTable::reference` grew sand. Every move is a swap, so per-material
mass is conserved by construction, proven by tests.

Two findings worth keeping: (1) a naive "swap into any less-dense neighbour"
rule made a liquid column *oscillate* between two open containers forever
instead of levelling — fixed by gating horizontal flow on column occupancy,
so a cell only flows toward a column currently holding strictly fewer cells
of its own material. (2) the first draft passed every test written for it,
including the water-levelling one, which only failed the way a debug trace
predicted (rigid oscillation) once actually run and watched frame by frame —
passing tests you wrote yourself proves less than it feels like it does, the
same lesson `xhigh` re-taught, more expensively, below.

Wired to a live demo (`www/physics.html`) with a real-browser e2e check
reading actual canvas pixels, not a screenshot. Left for later: gas
movement/buoyancy, temperature, real pressure (the current levelling rule is
a cellular approximation, not a solve) — everything past physics in
`NORTH_STARS.md` #2/#3's ordering.

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


**Night 2/7 — 2026-09-07 — Opus 5/low — Gas gets a pressure.** Chose the gas
model, out of a shortlist of brewing/distilling (Gin from a still rather than
from berries), the book-copying knowledge economy, and pressure. Gas won
because `NORTH_STARS.md` #4 names ONI's gas handling as one of the specific
things this project exists to fix — "gas behaves nothing like gas... CO2
doesn't actually settle" — and because the physics→chemistry→biology ordering
puts pressure before any of the game-layer pillars. It was also the biggest
lie in the code: a gas cell was a light solid, and opening a valve on a
pressurised tank did nothing, because nothing in the world knew what a
pressure was.

A gas cell's mass is now genuinely variable, and with a new data field
(`Material::gas_constant`) that gives it a real pressure, `P = m·R·T`.
`src/gas.rs` adds two rules on top of it, both conserving by construction:
mass transfer between cells of the *same* gas down the pressure gradient
(clamped to the exact levelling transfer, so no timestep overshoots), and a
swap that carries a whole parcel toward lower pressure when two neighbouring
cells hold different gases and so cannot merge. CO₂ is now in the material
table, and `src/chamber.rs` / `www/gases.html` is the demonstration: a
ten-atmosphere bottle behind one hole, and a slab of CO₂ released at the
ceiling that falls, spreads and settles into a flat layer on the floor.

Four things worth keeping:

1. *Conservation forbids the obvious buoyancy rule.* Comparing neighbouring
   cells by actual mass is right within one material and wrong across two: a
   gram of water that boils is still a gram, but a gram of steam is 1600
   cells' worth of gas in one cell, so by mass it is heavier than the water
   it came from and sinks. Real gas expands; a one-material-per-cell grid
   cannot let it. Nominal density is the honest stand-in across species.
2. *The old "gases must not spread sideways or the world shimmers" rule was
   half right and cost more than it saved.* What prevents shimmer is the
   strict density margin already in `pick_target` — air never spreads into
   air because air is not lighter than air. What the exclusion cost was a CO₂
   layer that could only fall, so it piled into a dune like sand. Removing it
   gives a flat, still layer, and settled-state churn measures at 8 cell
   changes per 50 steps with a vent and a scrubber both running.
3. *Pressure with no gravity term fires heavy gas at the ceiling.* Advection
   down a gradient that cannot tell up from down left speckles of CO₂ hanging
   in mid-air for buoyancy to drag back. Refusing the upward move when the
   mover is denser is the cheap stand-in for the hydrostatic term.
4. *A pretty renderer can make a browser test pass for the wrong reason.*
   Gas cells are now brightened by pressure, so as the bottle vents, every
   band of the canvas gets brighter — and the e2e check "the ceiling cleared"
   passed on brightness alone while measuring nothing. It now measures
   red-minus-green, which tracks how much CO₂ is in a band and ignores how
   compressed it is.

Verified: 118 lib tests and all five e2e checks green, clippy clean;
conservation residuals asserted relative (1e-6) rather than absolute, because
`Scalar` is `f32` and a cell of air masses a thousandth of a gram. The new
e2e reads real canvas pixels after six seconds of real time and sees the CO₂
layer arrive on the floor (mean height 17.97 → 1.75 rows, air pressure spread
0.90 → 0.037). I checked the terrarium for regressions by ASCII map, material
flip counts and pressure ranges against the same run with the gas step
disabled — behaviour matches — but I did **not** open `terrarium.html` in a
browser and look at it, and I did not look at `gases.html` by eye either; the
pixel evidence for it is the e2e's band measurements only. Not done: gas
interdiffusion (two gases cannot share a cell, so partial pressures do not
exist and a dilute heavy gas drifts along the floor as separate parcels
rather than mixing), any gravity term in the pressure field, gnome breathing
or CO₂ production, and brewing.

_Weekly usage was only 3%, thats not abitious enough. Targeting about ~10% for the rest of this week then aiming to track 14% per run after the next weekly reset. Will adjust effort level.
I'm going to refrain from providing feedback at this stage, the "picked from shortlist" is working as expected. It's creative expression, you wouldn't interrupt the artist before they have finished the work. I will analyise the transcripts and aim to provide the feedback that the Agent needs, not the feedback I want to give. I'm also not very happy with the way NIGHTLY.md reads, too prescriptive, an agent **is** allowed to reset a previous nights work - that all part of iteration.
NORTH_STARS.md isn't how I want it either, it was infact, not "written by the human". The distillation is good enough for now._


**Night 3/7 — 2026-09-08 — Opus 5/high — Chemistry, and a still that makes
the Gin.** Chose the chemistry tier, and used it for brewing. Three reasons
over the alternatives. `NORTH_STARS.md` #2's stated content order is
physics → chemistry → biology → game layer, and nights 1–2 finished the
physics tier's headline items while nothing of chemistry existed. #4 names
brewing and distilling as *core to the whole thing working, not flavour*,
and Gin — the resource every sanctioned exception to conservation is paid
for in — was still coming out of raw berries, which is the one thing the
capstone says it should not. And chemistry unblocks more than anything else
on the shortlist: combustion, respiration, the carbon cycle, steel by more
than one route are all rows in the same table. The alternatives considered
were gas mixtures with real partial pressures (a representational rewrite
with no game-layer payoff — still deferred, see below), and the book/
knowledge economy (pure game layer, skips the tier entirely).

Built: `src/chemistry.rs`, reactions between touching cells, conserving by
construction — each cell keeps its own mass, and the pair's shared
temperature afterwards is *solved* from its total energy before, so the heat
of a reaction is emergent rather than applied. `MaterialTable::
with_transitions` became `with_chemistry`, one relaxation deriving every
enthalpy offset from transitions and reaction arms together, so a material
in both cannot end up with two answers. Then the payoff: `src/still.rs` and
`www/still.html` — juniper in a warm pot ferments to wash (one row of the
reaction table), wash boils at 351.5 K while the water beside it boils at
373.15 K (two ordinary phase transitions, the 22 K gap being the whole of
distilling), the vapour crawls sideways through a gap in the pot wall
because ethanol vapour is genuinely denser than air, and it condenses to gin
on a cold floor where two thirsty gnomes drink it. Nothing in the code names
a brewing mechanic. A ledgered hatch keeps feeding botanicals so it runs as
a process rather than one batch.

Four findings worth keeping, each recorded where it bites:

1. *A vapour sitting at its own boiling point was invisible to every gas
   rule.* `is_mobile_gas` excluded cells mid-phase-change, which is
   permanently true of anything at its transition temperature. The still's
   entire charge boiled off and then lay on the wash unable to move. Only
   partial-mass transfer ever needed the check.
2. *"A heavy gas never moves up" was too strong by three orders of
   magnitude.* Night 2's blanket veto also blocks a six-hundred-atmosphere
   parcel, which is why kettles whistle. It is now a hydrostatic price
   (`LIFT × Δρ`), and the settled CO₂ layer it was written for still does
   not levitate.
3. *Mass conservation does not imply volume conservation, and the grid makes
   volume the hard one.* Boiling a cell of water gives a cell of steam at
   1300 atmospheres that could not expand, because the neighbours were a
   different species: the terrarium's boiled pool sat welded to the ceiling
   as a white lid. Fixing that (`gas::expand`, a three-cell shove built from
   transfers that already conserve) immediately exposed the mirror problem —
   condensation making *hundreds* of nearly-empty water cells, which behave
   like water because nothing looked at how full they were, and pile into a
   dune. `physics::coalesce_liquids` is the other half. Neither is optional;
   each without the other is worse than neither.
4. *The scenario has to state what the chemistry now requires.* Juniper
   standing in the terrarium's warm pool ferments — correctly — so the
   colony's Gin supply quietly turned into wash. It is now on a plinth under
   a stone shelf. Two similar ones: a still needs lagging or its head space
   refluxes gin straight back into the pot, and a gnome that wades into the
   receiver drinks the puddle it is standing in every time it runs out of
   breath, so gnomes no longer walk into liquids voluntarily (they can still
   fall in and still be flooded).

Verified: 136 lib tests, clippy and rustfmt clean, all six e2e checks green,
including a new one that runs the real page for nine wall-clock seconds and
reads canvas pixels — the receiver's floor goes from 25.7 to 157.9 mean
brightness against 29.6 at head height, with 11 g of gin and the water
charge unboiled. I also *looked at* all three pages as rendered PNGs rather
than trusting the numbers: the still reads correctly (vapour filling the
pot, spilling through the lyne, gin pooling in the receiver) and the
terrarium now reads as rain falling through a steam-filled jar instead of
the blue dune the expansion rule first produced. What I did **not** verify:
the terrarium over runs longer than 8000 steps (its bushes still ferment
eventually, and its jar settles near 405 K rather than anything cool); the
still past ~9500 steps, when the pot must run dry; and any of it at a
different grid size. One honest number: the terrarium's *relative* energy
residual now reaches ~9e-6 over 8000 steps where it used to be 1e-8 — that
is 3 J of `f32` rounding against 112 kJ of ledgered boundary flux, and it
grows sub-linearly (2.9 J at 4000, 3.2 J at 8000), so it reads as rounding
rather than a leak, but it is close enough to the tests' 1e-5 bar to be
worth watching.

Deliberately left undone: gas *mixtures* — two species still cannot share a
cell, so partial pressures do not exist and a dilute gas cannot dissolve
into another one; bulk expansion is now right, molecular mixing is still
absent, and that is the single biggest remaining lie in the physics. Also
untouched: gnome respiration and CO₂ production, the book-copying knowledge
economy, the Gnome Grandmother, farming, and buildings.

_Observed the Chemistry in action.
31% -> 36% weekly useage, still not ambitious enough.
"steel by more than one route" is an intresting sentence to pop out.
In the dictation I said 
"Now we will introduce the state changes. We can introduce burning wood.  I would like to explore iron and carbon and how to make steel in different ways to make rust."
Ah okay, now I read it I see whats happened. I shouldn't blame anyone. I said "and"
by the voice to text heard "in". So I had intened to bring up the idea of differnt
oxide states of iron, but it ended up saying something else. The different rust idea
shouldn't be elevated by this comment, just drawing attention to a live
example of the telephone game.

The one run per night was supposed to allow me to start the run, then go to bed!
And wake up to finished work. It hasn't succeeded in this regard (the going to bed part).

This is weird, I'm writing the journal for myself but I have to explain it 
to an audience.

Discussions with night 3 yielded an update to the prompt._

_2026-09-10T21:43:14+12:00 I've added /usage-check to global CLAUDE.md
updating the prompt to use that. I don't need to manually run /cost from
CC cli now. I also just built a Spellwise app, but that 5 hour window is almost closed now.
shouldn't effect the nightly run._
**Night 4/7 — 2026-09-10 — Opus 5/xhigh — Gas that mixes, and water that
evaporates.** Chose gas mixtures, the item night 3 named as the single
biggest remaining lie in the physics, over gnome respiration (which needs a
mixed atmosphere to breathe from), the biology tier (which would sit on air
that could not mix), and the knowledge economy (pure game layer). A second
reason turned up in the source. `NORTH_STARS.md` #4 says ONI's "CO2 doesn't
actually settle the way ONI's simplified layers show it", and night 2 built
exactly a CO₂ layer on the floor under clean air. The dictation behind that
line says the opposite: *"a layer of CO2 at the bottom and O2 on top ...
surely we can do better than that — gas is mix. CO2 is heavier but it
doesn't all fall to the bottom of a room."* The distillation's parenthetical
is ambiguous enough to have been built backwards; by that file's own rule
the dictation wins, and I haven't touched the file.

Built: a gas cell now holds a mixture (grams per species, plus liquid mist),
so gases share cells. Bulk flow, which now carries momentum across each cell
face, and per-species interdiffusion replace `advect` and `expand`.
`src/vapour.rs` reads each vapour's saturation curve off the boiling point
and latent heat its transition already carried (Clausius–Clapeyron), and
runs evaporation below boiling, dew on cold surfaces, mist and rain on it.
All three scenarios were re-grounded. The chamber's CO₂ sinks and then
mixes (it has a headless runner now). The still distils by evaporation,
with a relief valve and some water co-distilling. The terrarium is rebuilt
as a warm spring under a level pool: the jar sits near 316 K rather than
400 K, the gnomes no longer pay Gin to survive the weather, and for the
first time they can reach their juniper, which every earlier layout had
stranded across the pool. `Scalar` is `f64` now; `src/math.rs` records the
measured reason.

Findings worth keeping, each written up in the code where it bites:

1. *Conserving energy is not obeying thermodynamics.* Twice tonight the
   residuals sat at 1e-14 while the physics was badly wrong. Surface boiling
   at a fixed one-atmosphere point, inside a pressurised pot, pumped heat
   uphill. Condensation, relaxing toward the naive saturation gap, swung the
   still's head space above 410 K beside a 368 K hob. The ledger cannot see
   either. A temperature map can: a cell hotter than every heat source.
   `report::temperature_map` and `--temps` exist for that.
2. *Relax toward the equilibrium the step itself moves.* Latent heat going
   into a gas cell's tiny heat capacity shifts saturation more than the step
   does, so the linearised equilibrium is the right target, not the gap.
   It's `conduct_heat`'s old clamp, generalised.
3. *The grid's one-cell bubble is the standing artifact.* A boiled cell is
   one cell of vapour at hundreds of atmospheres. The vapour rules
   deliberately leave bubbles and bursts alone until flow has expanded them.
   What survives of it: the still's opening boil puts roughly a third of
   its gin back in the pot.
4. *Relaxing pressure without momentum turns every narrow opening into a
   bottleneck.* A two-cell lyne arm needed two atmospheres of drive, which
   raised spirit's boiling point above the pot's walls. Momentum fixed it:
   the pot now runs at 1.1–1.2 atm.
5. *Every test was green on a sloped pool.* Convecting air spent its cells'
   `moved` flags, so the water beside it never levelled. I only caught it by
   looking at a render. It's now a pixel check in `terrarium_canvas`.

Verified: 144 lib tests in both release and debug builds, clippy clean, and
all six e2e checks, which read real canvas pixels after real time. Those
now cover CO₂ visible above the floor, a level pool, the water cycle turning
over with nothing boiling, and gin in the receiver. I measured energy drift
per physics stage in a violent world, and every stage is exactly zero. I
looked at all three pages as browser screenshots. What I did **not**
verify: any run past ~8000 steps, any non-default grid size, or frame rate
anywhere slower than headless Chromium (pages now step on a time budget).
The terrarium's water cycle is real but hard to see: under a milligram of
condensation per step, and no visible fog on the lid; raising the dew
threshold did not change that. Gas conduction is about 200× real per cell
and dissipates a hot parcel before it can rise, so the hot-air test isolates
buoyancy rather than showing a plume. The CO₂ mixing rate is tuned (about
10× molecular). The bubble/burst heuristic has not been tried on a genuine
pressure vessel of vapour.

Left undone: splitting air into N₂/O₂ so gnomes and fires consume oxygen
and make CO₂ into a mixed room (the obvious next step now); gases dissolved
in liquids; wash as a real water–ethanol solution; pressure-dependent
boiling for submerged cells; and making the terrarium's cycle visible.
