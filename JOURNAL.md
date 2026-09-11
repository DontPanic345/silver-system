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


_2026-09-11 The real problems of the experiment are showing up now.
The real problem is me! Obviously.
Acts of omission are still acts.
The system can only progress in the drection already set down. It would be impossible for this system to course correct by going 2 steps back to take 1 step forward. That option is not available to an agent on any single run. Something like that needs to come from outside the system.
Like a chess piece King. It starts in the center of the board, nothing at where it is currently standing. All around in every direction at the edge of the board is a finished product, every step it has to move closer to the finished product. So after the first step, the direction is already set. Each edge square is a different finsihed product, how will it move the other side of the board without turning back. It's monotonic.
This previous nights run had to go back into the dictation to resove the stratified CO2 issue. The first distillation read it one way. To me the first interptation reads weird and is ambgious. The night that added CO2 pressure did add it in a stratified way, this may have been a techincal reason and was part of pressing to mixed gasses, or it may have read the statement the wrong way. 
I highlighted the still be key to the whole thing 'to really make it pop' - I was probably wrong, this was just one thought that poped into my head one day - it shouldn't form a major govering piece. (As it stands I think the still is actually a very good proofing ground as it relys on so many mechanics).
I never reviewed the NORTH_STARS. I never reviewed the prompt. Now, even though it didn't come from my fingers, it **did** actually come from me, because I allowed it.
So the weighting that gets assigned to the previous nights work left undone/unblocked swamps anything anything else - because the best way to progress is to build on the previous work.
And all of this is completely obvious and predictable.
Note to self, after discussion with the agent, trying to get around going to f64, I was convinced it really is the best option at this point. The issue is the size of the evaporation amount compared to water that it is leaving. It just too small for the f32, given that exponent is already set, you can't fudge it any other way than by increasing the precision.
f64 and clipping the wings of the GPU is fine for this experiment, but note this point as we may be coming back!
Providing the /usage-check was huge success, the agent happily went up to 80% of the 5 hourly window. Next to optimise is when to finish, as the run had a long debugging session with lots of back forth? Best leave that out of the cache and start again with clean slate. Or has the run been going well, that agent is best placed to continue as everything is already loaded.
_
**Night 5/7 — 2026-09-11 — Opus 5/high — Biology, and a jar whose carbon goes
round.** Chose the biology tier — the next one in `NORTH_STARS.md` #2's
stated order, untouched since night 3 opened chemistry. The alternatives were
night 4's own named next step (splitting air into N₂/O₂), more gas physics,
and the knowledge economy. The commentary above is why I did not simply take
the leftover: the pull toward "whatever last night left unblocked" is real,
and the one genuinely sideways move available was to change tier rather than
to go deeper into the one we were in. Oxygen came along anyway, because
photosynthesis has to put its oxygen *somewhere* — but as a consequence of
the tier, not as the goal. The tier also unblocks more than anything else on
the list: farming is a #4 pillar, the carbon cycle is a #3 one, and neither
can start without a living process that moves mass in proportions.

Built. `Metabolism` (`src/material.rs`) is a data-driven living process:
grams in, grams out, asserted equal when the table is built. A
[`Reaction`](src/chemistry.rs) could never express this — it relabels two
cells, each keeping its own mass — and `6 CO₂ + 6 H₂O → C₆H₁₂O₆ + 6 O₂` is
nothing but proportions. The enthalpy solver that derived offsets from phase
transitions and reaction arms is generalised from pairwise constraints to
n-term ones, so all three rule kinds feed one relaxation; photosynthesis
declares its heat and its reverse is thereby forced, so a bush cannot gain on
the round trip. `src/life.rs` runs them: every parcel of mass carries its own
enthalpy and the host settles the difference, so the heat is emergent, and a
full plant seeds a neighbour by splitting its own mass. `src/light.rs` marches
sunlight down each column through a new data field (`Material::opacity`) and
a `Sun` pays for the joules through the ledger — the day/night cycle the JS
terrarium had in August and lost. Oxygen is a real species; "air" is now the
inert bulk, the table declares an atmosphere mixture, and breathing reads the
partial pressure of what is actually in a cell rather than a flag on its
label. Gnomes respire — belly biomass plus oxygen becomes CO₂ and water
vapour at the table's own photosynthesis proportions — and plant cuttings out
of the same belly. The terrarium got a glass lid, a watered garden and a day.

Five findings, each written up where it bites:

1. *Conserving mass and energy does not conserve **atoms**.* Both residuals
   sat at 1e-13 while the jar's carbon drifted, because two gnome cuttings had
   overwritten cells of air that had CO₂ in them. `conjure_mass` books the
   loss, so the ledger was right and the chemistry was wrong. `gas::displace`
   is the fix and `the_carbon_in_the_jar_goes_round` is the test: 0.4 g per
   gram of plant, 12/44 per gram of CO₂, plus whatever is in the bellies,
   constant to a part in a billion.
2. *A parcel must leave and arrive at the same temperature.* Debiting the host
   at its pre-step temperature and crediting the acceptor at its post-step one
   leaked 2e-8 relative in a few hundred steps. Found by the conservation
   test, invisible to inspection.
3. *A jar with a stone lid is a jar in the dark* — and nothing noticed until
   something cared about light, at which point the garden could only ever run
   the night half of its books. Glass is stone with `opacity` changed.
4. *Gin and food were the same appetite, and that quietly broke the cycle.* A
   colony in a comfortable jar spends no Gin, so its flasks stay full, so it
   never eats, so it never exhales, so the garden starves. A gnome is hungry
   because it is alive.
5. *Night shading fights hue-based pixel checks, and holding blue back fixes
   both at once.* A flat dim deep enough to read as night takes water's blue
   under the threshold the pool check uses; taking more out of red and green
   makes the scene a third darker **and** more blue-dominant, so the checks
   get stronger as the picture gets darker.

Verified: 167 lib tests in release, clippy and rustfmt clean, all six e2e
checks — including a new one that walks the live page until it has seen both
a midday and a midnight frame and compares mean canvas brightness (79 against
66), and a rewritten still check that counts gin-coloured pixels (0% → 17–22%
on the receiver floor, 0% at head height) instead of comparing brightness,
because gnome breath now fogs the condenser. I looked at rendered day and
night frames rather than trusting the numbers. Over 12000 headless steps the
garden goes 2.50 → 2.59 g and 5 cells → 13, CO₂ climbs 0 → 0.096 g, oxygen
falls 0.331 → 0.281 g, residuals stay at 1e-13 relative.

What I did **not** verify: any run past 12000 steps, or any non-default grid
size. **The colony is a net oxygen sink** — the garden supplies roughly 40% of
what four gnomes burn, and extrapolating the slope it would reach the
suffocation threshold somewhere around step 26000; I did not run that far, so
I do not know whether the garden catches up or whether someone goes ethereal.
One run showed the thinnest air in the jar dipping to 0.082 atm against a
0.080 limit; that is a local pocket around a breathing gnome, and I did not
chase it. I did not look at `gases.html` or `still.html` by eye this session.
The gnome respiration rate is tuned, not derived, and the note in
`src/gnome.rs` says why there is no consistent scale to derive it from.

Deliberately left undone: **decomposition** — there is no fungus, no humus,
and nothing happens to a plant that drowns or freezes, so carbon only returns
to the air through something's lungs; that is the obvious next piece of the
tier and `NORTH_STARS.md` #3 names it. Also untouched, and carried over: the
book-copying knowledge economy, the Gnome Grandmother, buildings, nitrogen,
gases dissolved in liquids, wash as a real water–ethanol solution, and
pressure-dependent boiling.

**Night 6/7 — 2026-09-11 — Opus 5/high — The glass pane, and a wall that is
the hole you dug.** Chose the fourth tier — human interaction — over the
obvious leftover, which was decomposition (night 5's own named next step,
and the piece that would close the carbon cycle without lungs). Three
reasons. `NORTH_STARS.md` #2's stated order is physics → chemistry →
biology → game layer, and after five nights the first three had all been
opened and the fourth had not been touched at all; #3's capstone sentence is
"a terrarium people can see on their screens **and interact with**", and
until tonight every page in this repo was a window you could only watch; and
it is night six of seven, so a tier left unopened now stays unopened. The
commentary above night 5 is the fourth reason. The king moves outward and
cannot come back, and the strongest pull each night is toward whatever the
last night left unblocked — so when a genuinely sideways move is available
and defensible, take it.

Built: `src/order.rs`. The player never touches the world; the player writes
an order on a cell and a gnome walks over and does it, after it has breathed
and eaten and never before. **Dig** takes a cell of solid into a gnome's
hands as real grams at the temperature it came out at; **build** puts that
load back down; **temper** warms or chills a cell and is the magic, priced in
Gin at the same rate a gnome pays to save its own life. The colony carries
the queue, the renderer draws the markers and a pip of what each gnome is
holding, `report::cell_json` is the inspector, and `www/terrarium.html` has
a tool palette, click-to-designate, hover-to-inspect and an orders panel.
`--dig/--build/--warm/--chill i,j` do the same thing headlessly.

Four findings worth keeping:

1. *A gnome's hands are its belly's sibling* — mass that has left the world,
   booked out of the ledger until it is put back. That one decision is the
   whole game design: there is no resource counter anywhere, so **you cannot
   build what you have not dug**, and the wall you get is the hole you made,
   gram for gram and kelvin for kelvin. `NORTH_STARS.md` #4 opens by
   objecting to ONI turning dug rock into an abstract number; this is what it
   looks like not to.
2. *The physics does the game design if you let it.* My first build target
   was a cell of air above the walkway, and the test failed because sand is
   granular and fell. A wall has to be built on something. Nothing in the
   order code knows that.
3. *Every overlay you draw on a cell eats the pixels a pixel-check was
   using.* The order marker takes the border, the gnome takes the middle,
   the carry pip takes the top-left corner — and each one in turn broke the
   e2e check that had been sampling exactly there. A canvas assertion has to
   name which pixel and why. (The pip also had to be framed in near-black:
   a pale gnome carrying pale sand was the same dot as an empty-handed one,
   which I only saw by looking at a rendered frame.)
4. *A gnome standing beside the cell digs it on the very next step*, so
   "one order outstanding" is a race the page loses. Pausing to mark up and
   then resuming is the fix — and is how a person plays anyway.

Verified: 176 lib tests in release, clippy and rustfmt clean, and all seven
e2e checks, including a new `terrarium_orders.test.mjs` that clicks the real
canvas with a real mouse, watches the hole appear in real pixels against its
undug neighbour in the same frame, and watches the spoil come back down as a
wall — mass residual 5.4e-14 throughout. I looked at rendered frames of a
marked-up jar and of a gnome carrying a load rather than trusting the
numbers. What I did **not** verify: any run with orders past about 2500
steps; whether a colony kept busy with orders starves its garden; the
warm/chill and rub-out tools by eye or in any e2e (they have unit tests
only); what happens when the cell under a standing order changes phase
before anyone reaches it; and I did not re-open `gases.html` or `still.html`
this session. A carried load does not cool while it is carried, which is a
lie of the same family as instantaneous digestion.

Deliberately left undone: **decomposition** is still the open piece of the
biology tier — nothing rots, so carbon only comes back through lungs — and
carbon aside, a colony that can dig now has nowhere to put anything: there
is no stockpile, no drag-to-designate over a region, no priority between
orders, and no building that is a *machine* rather than a block. Also still
carried over: the book-copying knowledge economy, the Gnome Grandmother,
nitrogen, dissolved gases, wash as a real solution, and pressure-dependent
boiling.

**Night 7/7 — 2026-09-12 — Opus 5/high — Rot, and a jar that is still busy at
eighty thousand steps.** I started by *measuring* rather than choosing: night
5 had left an open question — the colony looked like a net oxygen sink, and
nobody had ever run the jar past 12000 steps — so the first thing I did was
run it to 60000. The suffocation never happens; oxygen dips to 0.28 g at
10000 and comes back above where it started. What does happen is worse and
quieter. **By step 30000 the colony had stopped.** Gin spent to 1.2 of 400,
`mass_conjured` frozen to the microgram for the next 30000 steps: four gnomes
alive, embodied, breathing, and doing nothing at all, in a jar where every
invariant still held to 1e-14. That measurement chose the night for me. The
alternatives were the knowledge economy (still the largest untouched piece of
`NORTH_STARS.md` #4) and a soil/humus loop; both would have added to a world
that had quietly stopped being a world.

So: **decomposition**, the piece nights 5 and 6 both named and both skipped,
plus whatever it took to make the long run genuinely run. Two materials and
five rows of the table: a bush sheds `litter`, a frost kills one outright,
litter rots into `fungus` and carbon dioxide, mould eats the litter around it
and spreads through a heap in the dark, and mould with nothing left to eat
spends itself. Nothing in code names any of them. Carbon now gets back into
the air without going through lungs, and the jar's carbon constant covers
five places instead of three. The `Metabolism` machinery could not express
any of it before tonight, for one reason: a living process could only emit
gases, mists, and more of its own host.

Five findings worth keeping:

1. *Conserving mass and conserving atoms come apart again, in a new place.*
   `life::give` now puts something solid into a cell of air — and the air has
   to go somewhere, not nowhere. Worse, **products must be placed gases
   first**: a solid product *consumes* a gas cell and a gas product does not,
   so a rot with one cell of air beside it put its mould there and then had
   nowhere to breathe out. The carbon dioxide it could not place was handed
   back to the host as more host. Six micrograms a step, mass exact to
   1e-14, and the jar was turning its own atmosphere into dead leaves.
2. *A cell that is mostly empty is mostly air — but only if it is loose.* A
   gnome standing in the first grains of a drift of litter was being treated
   as buried alive: it gasped, conjured oxygen it was already standing in,
   and the colony bled 400 Gin and a third of a gram of oxygen into the jar
   in 12000 steps. The fix has a sharp edge: a cell of juniper at a tenth of
   its density is not a gappy bush, it is a *small* one, so "under half full
   is passable" had to be restricted to granular materials. When it wasn't,
   the colony walked straight through the garden into the water bed behind
   it and banished the pool a gram at a time to breathe.
3. *Magic should be the last resort, and it wasn't even the second.* A gnome
   out of air now steps sideways before it pays. That one rule is most of
   why the colony is solvent at 80000 steps where it was broke at 30000.
4. *A summary number can be alarming and useless at the same time.* The
   thinnest air **in the jar** goes to zero around step 20000 and stays
   there — a garden dense enough to seal a cell inside its own canopy
   breathes that cell flat overnight and never refills it. That is correct,
   and it says nothing about whether anyone is suffocating, which is what it
   was being read as. The report now carries both it and the worst gnome's
   remaining breath.
5. *Scale decides what you can see.* The whole jar's biology moves a few
   micrograms a step, so a compost heap built from leaf fall alone would take
   a hundred thousand steps to appear. The scenario is seeded with one
   instead — which is also just what you do when you plant a terrarium — and
   leaf fall is what keeps it topped up rather than what has to create it.

Verified: 182 lib tests in release, clippy and rustfmt clean, all seven e2e
checks, including a new one that counts mould-coloured pixels on the live
canvas (the scenario seeds litter and no mould, so every violet pixel grew)
and watches the heap go down, 0.0599 → 0.0413 g, while the garden puts on
weight. I looked at a rendered frame of the garden corner rather than
trusting the numbers: brown litter among the bushes, three cells of violet
mould, a gnome standing in the leaves. Over 80000 headless steps: residuals
5.5e-14, all four gnomes embodied throughout, the colony still eating in the
last quarter (that is now a lib test), oxygen flat at 0.33 ± 0.02 g, and a
standing litter stock that builds to 0.07 g after the seeded heap is gone.

What I did **not** verify: any non-default grid size; the frost-kill
transition anywhere except its own unit test (nothing in the terrarium gets
near 274 K); `gases.html` or `still.html` by eye; and whether the
`coalesce_loose` generalisation changes anything about sand, which is always
full in every scenario we have. I did not check whether the 30000-step stall
I found in the old code had a single cause — my changes removed it, and I
did not go back to confirm which one.

Deliberately left undone, and the honest ugly bit: **by 80000 steps the
garden has grown over the walkway and the colony lives inside the hedge** —
four gnomes in one cell, bellies empty, idle, Gin trickling between 2 and 30.
Nobody dies and the jar keeps running, but they are stuck, and the cause is
pathing rather than physics: juniper is a static solid, a gnome can climb one
course, and a bush that seeds the cell behind you walls you in. Also
untouched: the book-copying knowledge economy, the Gnome Grandmother, a soil
nutrient that would make rot load-bearing for farming rather than only for
carbon (the obvious next piece, and the reason to build humus), nitrogen,
dissolved gases, wash as a real solution, and pressure-dependent boiling.

_On the seven-night experiment itself, since this is the last of them: it
compounded. Nothing was reset, every night built on the night before, and the
substrate that survived is the whole of it rather than a scaffold. The shape
of the failure is worth naming though, and it is the one the user predicted
in the note above night 5: the pull is always toward what last night left
undone, and four of the seven nights took exactly that. What broke the
pattern both times was **measuring before choosing** — night 6 read the
journal as a whole and moved sideways on purpose; tonight ran the jar four
times longer than anyone had and let the number pick. A run of nights like
this one might be better briefed to spend its first twenty minutes measuring
the state it inherited rather than reading about it._

**Night 8 (bonus round) — 2026-09-12 — Opus 5/high — Gnomes that can find
their way.** Measured before choosing, which is what night 7's closing note
asked for. The first thing I did was run the jar the way night 7 left it,
and the number chose the night: **from step 10000 to step 80000 all four
gnomes stand in one cell**, idle, bellies flat, and by 70000 the colony's
Gin is spent to 1 of 400. Night 7 named the cause in its own last paragraph
— "the cause is pathing rather than physics" — and left it. The alternatives
were the knowledge economy (still the largest untouched piece of
`NORTH_STARS.md` #4), a soil nutrient to make rot load-bearing for farming,
and buildings that are machines rather than blocks. Each of them adds to a
world whose people have stopped moving, which is the same mistake night 7
declined to make.

Built: [`src/path.rs`](src/path.rs), the gnome mobility graph and a
breadth-first flow field over it, recomputed every step rather than planned.
Steering is now a route: goals are filtered by whether a gnome can actually
get to them, orders are claimed by how far away they are *by route* and
surveyed once a step so one nobody can reach is drawn dashed, counted on the
page and eventually retired. A gnome will not walk onto the pool or into
anything near lethal, and it can chimney straight up out of a pit.
Food became a column in the material table (`Material::nutrition`) instead
of two hard-coded materials in the game layer, and "crop" is now *anything
the table grows*, with `MaterialTable::shed_form` saying what a cut one
leaves. A colony shut in by its own garden lifts the hedge up a course and
walks out underneath.

Findings worth keeping, each written up where it bites:

1. *A route is a promise about what the walker will do.* `path::moves` is
   the walker's own move set — fall before walk, one course of climb — so a
   route cannot offer a step the gnome will refuse. Everything else in the
   module is downstream of that one rule.
2. *A gnome could step down anywhere and only climb back diagonally.* Any
   pit one cell wide was therefore a permanent trap, and the colony ate its
   way out of the one a dig order opened. Falling is generous and climbing
   was not; they have to match.
3. *"Mostly empty is passable" applied only to granular materials, and mould
   was not one.* Two milligrams of it across a doorway was a locked door.
   Fungus is granular now — it also slides and piles, and fills holes.
4. *Every last resort needs a clock and a direction, and getting either
   wrong is a lawnmower.* Cutting a crop went through four versions: cut to
   reach food (2.5 g of garden into 1.9 g of litter in 1500 steps); cut only
   when penned, by cell count (four gnomes then paced a five-cell pocket for
   40000 steps with the garden two cells away, because five was one more
   than the threshold); cut toward the way out, after 2000 steps of getting
   nowhere; and finally **do not cut at all if you can lift**.
5. *A cut cell keeps its mass, and that is why cutting did not even work.*
   Half a gram of bush becomes half a gram of leaf litter in a cell that
   holds a fiftieth of that, so the way stays blocked and the gnome cuts the
   next one. Lifting the column is a swap: the bush keeps every gram and
   every joule, the ledger has nothing to say, and the path opens.
6. *Reachability is the missing half of the glass pane.* Before tonight an
   order being walked to and an order walled off from the whole colony were
   the same picture on screen.

Verified: 193 lib tests, clippy and rustfmt clean, and all eight e2e checks
including a new `terrarium_routing.test.mjs` that clicks a cell at the far
end of the jar with a real mouse, watches a gnome route there and dig it,
and reads the dashed marker of an unreachable order against a solid one in
the same frame. I read the act histogram and the per-gnome trace of long
runs rather than trusting the summary — `--trace` exists because of that and
every diagnosis above came out of it — and I looked at rendered frames of
the live jar, with gnomes spread along the walkway rather than stacked, and
of a dashed marker on the far wall. What I did **not** verify: any
non-default grid size; `gases.html` or `still.html` by eye (their e2e checks
pass); and whether the dashed marker is legible to a human at normal zoom —
it is deliberately dim, and at 12 px a cell it is faint.

Left undone, and the honest part. Over 80000 steps the colony now walks
throughout, all four stay embodied with full breath, and the garden ends
*bigger* than it started (2.50 → 2.61 g, 5 → 16 cells, litter 0.06 → 0.16 g,
residuals 6e-14 and 1.5e-12) — but **the colony ends the run broke and
hungry**: Gin 400 → 0, bellies empty in every late sample. The jar's own
ethereal pipe spends 8.3 Gin per 1000 steps for ever, which is more than
four gnomes forage, so the Gin economy is the obvious next piece and it is
an economy question rather than a pathing one. The other known-bad
interaction: **a dig order into the garden's water bed still costs most of
the garden** — gnomes fall into the hole, banish water to breathe (17 g
through the ledger in 6000 steps), and 3 g of the garden mashes into wash
where warm water now touches it; juniper ends at 0.5 g instead of 2.5. I
made it four times less bad tonight and did not finish it. Still untouched:
the book-copying knowledge economy, the Gnome Grandmother, a soil nutrient,
nitrogen, dissolved gases, wash as a real solution, and pressure-dependent
boiling.
