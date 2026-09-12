# Seven nights, plus two — a retrospective

Closes out the run `NIGHTLY.md` opened on 2026-09-07: one unattended Opus
session per window, each starting cold from the repo's committed state, no
resets. Nights 1–7 were the planned sequence; nights 8–9 were bonus rounds
run on 2026-09-12 to use up the remaining weekly budget before its reset,
under the same brief. This document is the closeout `effort-level-experiment/README.md`
is for that experiment — the per-night detail lives in `JOURNAL.md` and stays
there; this is what the run looked like end to end.

## What ran

| Night | Date | Model/effort | Chose | Landed |
|---|---|---|---|---|
| 1/7 | 09-07 | Opus/low | Physics + game layer, no process | `World`, `physics.rs`, `gnome.rs`, `terrarium.rs` — conserving mass/energy movement, phase change, Gin as the one ledgered exception |
| 2/7 | 09-07 | Opus/low | Gas pressure | `src/gas.rs`: real `P = m·R·T`, pressure-gradient transfer, CO₂ in the table |
| 3/7 | 09-08 | Opus/high | Chemistry, brewing | `src/chemistry.rs`, `src/still.rs`: data-driven reactions, Gin from distilling instead of berries |
| 4/7 | 09-10 | Opus/xhigh | Gas mixtures, vapour | `src/vapour.rs`: gas cells hold species mixtures, evaporation below boiling, `Scalar` → `f64` |
| 5/7 | 09-11 | Opus/high | Biology tier | `src/life.rs`, `src/light.rs`: `Metabolism`, oxygen as a real species, day/night sun, carbon cycle |
| 6/7 | 09-11 | Opus/high | Game/interaction layer | `src/order.rs`: dig/build/temper orders, click-to-designate UI, hover inspector |
| 7/7 | 09-12 | Opus/high | Decomposition (measured first: colony had silently stalled by step 30 000) | Rot, mould, litter; fixed a gas/solid product-ordering bug that was quietly eating the jar's atmosphere |
| 8 (bonus) | 09-12 | Opus/high | Pathfinding (measured first: colony frozen in one cell for 70 000 steps) | `src/path.rs`: mobility graph, flow-field routing, reachability on the glass pane |
| 9 (bonus) | 09-12 | Opus/high | Gin economy (measured first: same ending, five unrelated causes, none of them pathing) | `Job::Supply`, mash-based Gin, cut the in-jar still after it deadlocked for a real reason |

Every night after 4 ran at the same model/effort (Opus/high) by this week's
own direction, rather than the effort-level experiment's varying dial — this
run was testing compounding, not effort, and held that variable fixed.

## Did it compound?

Yes, materially: nothing was reset, each night's substrate is what the next
one built on, and the surviving code is the accreted whole — physics → gas →
chemistry → vapour → biology → interaction → decomposition → pathing →
economy — not a scaffold one night threw away. That's the first time in this
project's history a generation has survived past its own night (see
`CLAUDE.md`: "only the bare Rust scaffold has ever survived a generation").

But the shape of the failure mode it was watching for showed up anyway, and
is worth stating plainly because Night 7 predicted it before Night 9
confirmed it twice more: **the pull is always toward whatever last night
left undone.** Four of the nine nights (2, 3, 5, 6) picked the next tier in
`NORTH_STARS.md`'s stated order more or less by default. What broke that
pattern, both times it broke, was *measuring the inherited state before
choosing* rather than reading about it: Night 6 read the whole journal and
moved sideways on purpose; Night 7 ran the jar four times longer than anyone
had and let a stalled colony pick the work; Night 8 and 9 both opened by
reproducing the previous night's long run before deciding anything. The
nights that measured first found real, non-obvious bugs (the gas/solid
ordering bug, the five independent causes behind "the colony goes broke")
that the nights which didn't measure would not have found by reading code.
If this pattern runs again, brief it to spend the first fraction of the
session running the inherited build to its own limits before choosing what
to build next — night 9's own closing note said the same thing.

## What the codebase can do now, end to end

- Conserve mass and energy by construction through movement, phase change,
  chemistry, and living processes, with drift asserted at 1e-14 residual
  over 80 000-step runs, not just short unit tests.
- Model a real atmosphere: pressure, per-species diffusion, bulk flow with
  momentum, vapour pressure and evaporation below boiling, CO₂ that mixes
  rather than layering.
- Run a full carbon cycle: photosynthesis and respiration as exact reverses,
  litter and mould returning carbon to the air without a gnome's lungs.
- Route gnomes by an actual mobility graph rather than naive stepping, with
  reachability surfaced on the same UI that shows the order itself.
- Sustain a Gin economy that doesn't monotonically drain: mashing turns
  garden biomass into Gin without a still, and the colony ends an 80 000-step
  run richer in Gin and garden mass than it started, not broke.
- Survive a genuinely long unattended run — 80 000 steps, four embodied
  gnomes, fed, breathing, moving — which nights 1–6 never actually tried.

## The shape of the bugs, across nights

A few classes of mistake recurred, each a candidate for `PRINCIPLES.md` if
it turns up again on a different substrate:

1. **Conserving mass is not conserving atoms.** Twice (nights 4/still,
   night 7/rot): a ledger balanced to 1e-14 while carbon or CO₂ was quietly
   being turned into the wrong thing, because nothing in the accounting
   distinguishes "some mass" from "the right mass of the right thing."
2. **A derived quantity is not the raw one it's derived from.** Night 3's
   still ran hot because boiling was checked against a fixed one-atmosphere
   point inside a pressurised pot; night 9's gnomes died in a warm room
   because their lethal check read a bare thermometer instead of felt
   temperature scaled by heat capacity. Same shape: a physical rule that
   silently assumed a reference condition the actual cell no longer met.
3. **Order of operations inside one step is a real dependency, not
   bookkeeping.** Night 5's convection spent a `moved` flag air needed and
   silently broke levelling; night 7's product-placement order (solid before
   gas) silently ate the atmosphere. Both were green on every test that
   didn't specifically watch for the interaction.
4. **A green suite is not a working system; a long run is a different test
   than a short one.** The clearest instance: nights 5 and 6 both shipped
   with full test suites and clean e2e checks, and the colony still stalled
   completely by step 30 000 — a failure mode no test under a few thousand
   steps could have shown, and one only found by running the thing for a
   long time and reading the numbers rather than trusting green.

## What's left undone, stack-ranked

From Night 9's own list, which is the most current:

1. The knowledge economy (book-copying) — named as the largest untouched
   piece of `NORTH_STARS.md` #4 in nights 2, 4, 8, and 9 alike, and never
   built. The obvious next target if another round follows this one.
2. A soil nutrient that makes rot load-bearing for farming, not just carbon.
3. The Gnome Grandmother (never named beyond a placeholder).
4. Nitrogen / a real N₂-O₂ atmosphere split, dissolved gases, wash as an
   actual water-ethanol solution, pressure-dependent boiling.
5. The in-jar still: built, ran, hit a real deadlock (a pot that must empty
   to recharge can't coexist with standing water beside it, because liquids
   level), and was cut rather than patched. Two ways out are written up in
   `BREWING.md`; neither has been tried.
6. Long-run watches nobody has done yet: the garden still grows over the
   walkway past 80 000 steps in the current build (mitigated, not solved);
   nobody has run past 80 000 steps at all, or at a non-default grid size.

## Operational notes, for whoever orchestrates the next one

Two mistakes worth naming so they don't repeat:

- **The 5-hour session window is a shared, account-wide rolling budget, not
  a per-agent-run counter.** Launching Night 6 immediately after Night 5
  finished handed it Night 5's leftover usage instead of a fresh window, so
  it opened at 68% already spent, burned the remaining 32% in half an hour,
  and stopped near the account cap far short of a full night's work. This
  is the exact failure the `effort-level-experiment/README.md` already
  documented (`high` got starved the same way behind `low`/`medium`) —
  worth reading before assuming a new agent gets a clean window just because
  the last one finished. From Night 7 onward, each launch first confirmed a
  near-zero session percentage via the `usage-check` skill.
- **An artificial stopping target overrides self-direction, which is the
  thing this whole brief is trying to preserve.** An early instruction to
  the agents to wrap up "around 80% of the session window" was corrected
  mid-run: the brief already asks for a coherent, self-judged stopping
  point, and a fixed percentage second-guesses that. Later launches asked
  only for periodic `usage-check` visibility, with the stopping judgement
  left entirely to the agent.

One accidental interruption is also on record: Night 7 was killed by a
stray keypress and resumed a few minutes later from the same transcript.
The repo was clean at the moment of the kill (no partial commits), so
nothing was lost — a reminder that this brief's commit-as-checkpoint
discipline is what makes an accidental interruption cheap rather than
expensive.

## Budget spent

This week's chase (Nights 5–9, run back to back on 2026-09-11/12):

| Point | Weekly % | Session note |
|---|---:|---|
| Before Night 5 | 61% | — |
| After Night 5 | 67% | full window |
| After Night 6 | 70% | starved to ~30% of a window (see above) |
| After Night 7 | 78% | full window, resumed once after accidental kill |
| After Night 8 | 85% | full window |
| After Night 9 | 95% | full window |

Bonus rounds stopped at 95% weekly, deliberately short of the 21:00 reset,
rather than risk starting a tenth round with only 5% of headroom left —
next week's experiment gets its own full budget rather than inheriting a
run that hit the cap mid-task.
