# Plan

## The north star

Two statements of the same goal. Both are load-bearing; neither replaces the other.

> **A believable small world inside a large universe.**

> **A terrarium people can see on their screens and interact with.**

The first is the standard. Believability is not decoration — it is the whole
product, and it comes from the universe underneath being genuinely full. Only when
the big universe is sufficiently full will the small world be believable. A world
whose rules stop just past what the camera shows will read as a trick, however
pretty the camera is.

The second is the deliverable. It is a thing on a screen that a person opens,
watches, reaches into, and comes back to. Everything below is judged by whether it
moves those two statements closer together — a tranche that advances the physics
but can never be seen has failed, and so has a tranche that looks convincing over
a void.

Every tranche states how it serves the north star. Every planning pass restates it
in its own terms. If a planner can't say how the work in front of it serves the
north star, that is the finding, and it goes back to planning.

---

## How to read this document

The language is Rust. GPU execution is the intended direction of travel.

Tranches run in order: physics, chemistry, biology, the glass pane. Milestones
within a tranche are listed with their intent and targets, but that list is a
**starting position, not a contract** — `cycle-plan` sharpens it at tranche level
before the tranche runs, and again between milestones.

Targets here are the ones knowable now. Expect planning to add more, make these
sharper, and occasionally conclude one was wrong. Rounds are not listed at all;
they are planned a round at a time, because the shape of round N+1 is not knowable
before round N reports.

**The glass pane is tranche 4, but a viewer is needed in tranche 1.** Scenarios
have to be watchable long before the pane is built properly — a minimal renderer
lands in physics and grows. Tranche 4 is where it becomes the product rather than
a debugging tool.

---

# Tranche 1 — Physics

**Intent.** Matter that moves because of forces, not because a rule said so. This
is the substrate every later tranche stands on: if the physics is only
approximately right, chemistry built on it will be convincingly wrong, and biology
built on that will be nonsense with good graphics.

The earlier JavaScript work (`stable-fluids/`) got as far as conservative
transport, temperature and buoyancy, and halted on pressure. That halt is why this
tranche exists in this form: pressure and fluid dynamics are the point, not an
extra.

**Serving the north star.** Everything believable about a terrarium — water
finding a level, air moving because it's warm, a pile of soil sitting still — is
this tranche. It is also where the universe first gets "full": the rules must hold
off-screen and at rest, not just where the eye is drawn.

**Tranche targets.**

- Water in a U-pipe levels to within 1 cell across both arms and stays there for
  5,000 steps.
- Mass and energy conserve to 0.00% drift over 10,000 steps in every closed
  scenario.
- A resting configuration — pool, pile, sealed gas — is bit-stable at rest: no
  jitter, no creep, no checkerboard.
- Every primitive scenario passes empirically with no human looking at it.
- A step of the reference grid fits the performance budget set in milestone 1.

## M1.1 — Substrate and harness

**Intent.** The bones: how a cell is represented, how a step happens, how anything
gets measured. Get this wrong and every later milestone pays for it.

- Grid and material representation — structure-of-arrays, double-buffered, fixed
  timestep.
- A material table that is *data*, not code per material: density, viscosity, heat
  capacity, conductivity, phase, colour, and room to grow.
- The scenario harness: a scenario is a definition, consumed by both the headless
  runner and the viewer. One definition, two consumers.
- Headless empirical measurement — a scenario emits numbers, and a test asserts on
  them without a human in the loop.
- Test tagging and fast paths from the very first round (`cycle-red` owns this).
- A minimal renderer good enough to *watch* a scenario. Debugging blind is a tax on
  every round after this one.

**Targets.** A scenario runs headless and emits JSON measurements; the same
scenario renders; the reference grid steps within a stated per-step budget, with
the number recorded so later milestones can be held to it.

## M1.2 — Granular solids

**Intent.** Things that fall, pile, and then genuinely stop.

- Sand and powders: falling, piling, angle of repose.
- Static solids: stone, walls, containers that hold.
- The distinction between at-rest and asleep — resting matter should cost nothing
  and move not at all.

**Scenarios.** A central column of sand falls into a pile. A central column of
something else does *not* stay still. A pile at rest stays at rest for 5,000 steps.

**Targets.** Pile forms with a repose angle in a stated range; mass conserved
exactly; a resting pile is unchanged after 5,000 steps.

## M1.3 — Liquids and the free surface

**Intent.** Water that behaves like water at the surface, which is where the eye
looks and where naive solvers fail.

- Liquid transport with a sharp interface against gas.
- A pool at rest that is *still* — the failure that halted the previous attempt.
- Wetting, puddles, thin films, droplets that don't dissolve into fog.

**Scenarios.** A resting pool of water stays still. A column of water falls and
finds a level. Water poured into an irregular vessel fills it from the bottom.

**Targets.** Resting pool unchanged over 5,000 steps; a fallen column levels to
within 1 cell; no mass drift.

## M1.4 — Pressure and incompressible flow

**Intent.** The milestone the last attempt died on. Pressure is what makes fluid
read as fluid rather than as falling pixels, and it's the mechanism half of
chemistry later depends on.

- A pressure solve that survives resting states without a checkerboard null mode —
  staggered or compact, chosen on evidence, not on what's easiest to write.
- Hydrostatic pressure increasing with depth.
- Communicating vessels; flow driven by pressure difference.
- Sealed volumes that resist compression, and the beginnings of over-pressure —
  the hook chemistry's explosions hang on.

**Scenarios.** **The U-pipe**: a U-shaped pipe surrounded by air, one arm filled
with water, the water comes to a level across both arms. A sealed container holds
pressure. A breached container equalises.

**Targets.** U-pipe arms level to within 1 cell and hold for 5,000 steps; pressure
increases linearly with depth within a stated tolerance; a sealed volume's pressure
is stable at rest.

## M1.5 — Gases

**Intent.** Air as a real participant, not empty space. Most of what looks alive in
a terrarium is air moving.

- Gas transport, mixing, and filling available volume.
- Density-driven motion: cool air falls, warm air rises, heavy gas pools low.
- Multiple gas species coexisting — the groundwork chemistry needs for oxygen
  depletion and combustion products.
- Pressure equalisation across a connected region.

**Scenarios.** Cool air falls. A heavy gas released in a room pools at the floor
and stays there. A gas released into vacuum or low pressure fills the space evenly.

**Targets.** Gas count conserved; stratification by density is stable and does not
re-mix on its own; equalisation completes within a stated number of steps.

## M1.6 — Temperature

**Intent.** Heat as a field that moves matter and, later, drives every reaction in
tranche 2.

- Conduction, with conservation — the previous attempt's conduction leaked until it
  was rewritten in flux form; don't repeat that.
- Heat carried by moving matter.
- Buoyancy driven by temperature.
- Heat capacity and conductivity per material, from the material table.
- Hot and cold sources as placeable entities — needed by tranche 4, cheap to build
  here.

**Scenarios.** A hot cell in a bar spreads heat symmetrically and matches an
analytic decay curve. Warm water rises; cool air falls. A closed system's total
energy is unchanged.

**Targets.** Energy drift 0.00% over 10,000 steps; conduction matches the analytic
solution within a stated tolerance; symmetric setups stay symmetric.

## M1.7 — Consolidation and scale

**Intent.** Make the substrate something three more tranches can be built on
without dread.

- The full element set the later tranches need, in the material table.
- Performance against the budget set in M1.1, measured, with the profile recorded.
- Groundwork for GPU execution if the evidence says it's needed — structure-of-
  arrays layouts, no algorithm that depends on global sequential ordering.
- A completeness sweep: every primitive scenario, tagged, passing, kept for
  regression even when it's not in the default suite. Completeness is valued.

**Targets.** All primitive scenarios green; per-step cost within budget at the
reference grid; the default test suite runs inside a stated time.

---

# Tranche 2 — Chemistry

**Intent.** Matter that changes what it *is*, not just where it is. This is where
the world stops being a physics demo and starts having a history — things burn,
rust, dissolve, evaporate and come back as something else.

**Serving the north star.** The cycles are the believability. A terrarium is
convincing precisely because the water that evaporates comes back as rain, and the
leaf that falls becomes soil. This tranche builds the machinery those loops run on.

**Tranche targets.**

- A closed water cycle runs for 100,000 steps with no mass drift and no state it
  can't recover from.
- Every reaction conserves mass and energy, individually and in aggregate.
- Combustion self-extinguishes correctly when starved of fuel or oxygen — it never
  runs away and never burns from nothing.

## M2.1 — Phase change

**Intent.** The single most-visible chemistry: solid, liquid, gas, and the latent
heat between them.

- Melting, freezing, boiling, condensation, sublimation.
- Latent heat that is actually paid for — a boiling plateau, not an instant
  teleport between states.
- Pressure-dependent boiling point, if the pressure work in M1.4 supports it.

**Targets.** Boiling holds a temperature plateau while latent heat is absorbed;
mass conserved across every transition; ice→water→steam→water→ice returns to the
starting mass exactly.

## M2.2 — The reaction framework

**Intent.** Reactions as *data*, the way materials are data. One mechanism, many
reactions — otherwise every reaction is bespoke code and the world stops growing.

- Reactants, products, activation conditions (temperature, pressure, contact,
  catalyst), rate, energy released or absorbed.
- Reactions that respect conservation by construction, not by convention.
- Reversibility where it's real.

**Targets.** A reaction defined purely in data runs correctly with no new code;
mass and energy conserve across a long run with many reactions firing.

## M2.3 — Combustion

**Intent.** Fire, done properly — the archetypal emergent behaviour and the one
players test first.

- Wood burns: fuel consumed, oxygen consumed, heat released, products emitted.
- Smoke, soot, ash, char.
- Fire spreads through contiguous fuel and dies without fuel or oxygen.
- Ignition temperature; smouldering vs flaming.
- Local oxygen depletion — a sealed box smothers its own fire.

**Scenarios.** A wooden structure lit at one corner burns through and leaves ash. A
fire in a sealed box goes out, and the box's oxygen is measurably gone. A fire with
no fuel path doesn't jump the gap.

**Targets.** Sealed-box fire self-extinguishes with oxygen accounted for; burn
conserves mass into products; no ignition without an ignition source.

## M2.4 — Metallurgy

**Intent.** The long-timescale, multi-path chemistry — the same inputs reaching
different products by different routes. This is where the universe gets *deep*
rather than merely wide.

- Iron, carbon, and steel: more than one route to the alloy, with carbon content
  mattering.
- Smelting: ore, heat, reduction, slag.
- Rust by more than one route — oxygen and water; accelerated by salt or acid.
- Temperature-dependent material properties: hot metal, quenching, hardness.

**Targets.** Two distinct routes produce steel with the carbon content each route
implies; rust forms only where oxygen and water are both present; rate responds to
conditions rather than to a timer.

## M2.5 — Solutions, moisture and permeation

**Intent.** The quiet chemistry that makes soil, weather and living things
possible — matter dissolved *inside* other matter.

- Dissolved gases in liquids, with saturation limits and outgassing on heating.
- Moisture in solids: water in soil, water in wood, damp stone.
- Air in soil — the pore space biology will need.
- Permeation: air dissolved in rubber, escaping a balloon slowly.
- Diffusion driven by concentration gradients, at rates that differ per medium.

**Open question this milestone must answer.** Whether to model a contiguous
region's composition as a single fully-mixed entity, or as per-cell fields. Current
read: per-cell fields, because uniform composition transports composition at
infinite speed and deletes plumes, stratification and local depletion — the very
phenomena this tranche exists to grow. Revisit only as an optimisation for
quiescent enclosed volumes. **This is a planning decision for M2.5, made on
evidence, and recorded.**

**Targets.** A gas dissolved in water outgasses on heating, with mass conserved; a
balloon loses pressure over a long run at a rate set by the material; soil holds
and releases moisture rather than shedding it instantly.

## M2.6 — Explosions and ballistics

**Intent.** Violent, fast release — and, more importantly, debris that behaves
afterwards.

- Rapid pressure release coupling into the M1.4 pressure field.
- Particles that fly naturally and *land* naturally, rejoining the grid as ordinary
  matter rather than despawning.
- Fragmentation, shockfront, and the aftermath: a hole, scattered debris, heat.

**Targets.** Mass conserved through an explosion and its debris; thrown material
lands and comes to rest; no particle leaks out of the world or vanishes.

## M2.7 — The water cycle, closed

**Intent.** The capstone: every mechanism in tranches 1 and 2 running at once, in a
loop, indefinitely. This is the first moment the world is *alive* in the sense the
north star means.

- Evaporation from surfaces, driven by temperature and humidity.
- Vapour transport and mixing in the air.
- Condensation on cool surfaces; cloud, dew, fog.
- Precipitation, runoff, pooling, and back to evaporation.

**Targets.** 100,000 steps with 0.00% water drift; the loop runs continuously
without a human resetting anything; the cycle responds to a temperature change
rather than running at a fixed rate.

---

# Tranche 3 — Biology

**Intent.** Things that live off the cycles the first two tranches built —
consuming, growing, dying, and returning their matter to the world. Biology here is
not creatures with behaviour scripts; it is chemistry that reproduces.

**Serving the north star.** This is where the terrarium becomes a terrarium rather
than a weather box. It is also the strongest test that the universe underneath is
genuinely full: life only stays alive if the cycles beneath it actually close.

**Tranche targets.**

- A sealed terrarium sustains a living system for a very long run — the target
  number set at planning, and measured — without intervention.
- Carbon and nitrogen conserve exactly across all biological activity.
- Population levels respond to conditions and recover from disturbance, rather than
  sitting at a fixed value or dying out on a knife-edge.

## M3.1 — Soil

**Intent.** Not a material — a small ecosystem in its own right, and the substrate
everything else in this tranche needs.

- Soil structure: mineral grains, pore space, moisture, air.
- Humus as a distinct material with its own properties.
- Nutrient content held, transported and depleted.
- Compaction, drainage, waterlogging.

**Targets.** Soil holds water against gravity to a stated capacity and drains the
excess; nutrients move with water rather than teleporting.

## M3.2 — Decomposition and the carbon cycle

**Intent.** The return path. Without it, matter accumulates and the world runs down.

- Bacteria as a population responding to moisture, temperature, oxygen and food.
- Fungus and slime mould: growth along gradients, spread, and death.
- Dead matter → humus → nutrients, with carbon accounted for at every step.
- Aerobic and anaerobic pathways producing different products.

**Targets.** Carbon conserved through a full decomposition chain; decomposition
rate responds to temperature and moisture; anaerobic conditions produce
measurably different products.

## M3.3 — The nitrogen cycle

**Intent.** The second great loop, and the one that makes soil fertility a real
quantity rather than a number that only goes down.

- Fixation, nitrification, uptake, decay, denitrification.
- Nitrogen held in soil, in living matter, and in the air.

**Targets.** Nitrogen conserved across the whole loop; depleted soil recovers over
time through fixation; over-fertilised soil shows the consequence.

## M3.4 — Growth

**Intent.** Something that grows toward what it needs. This is the milestone that
makes the terrarium read as alive at a glance.

- Plants, moss, or their equivalent: roots into moisture and nutrients, growth
  toward light.
- Growth as consumption — matter drawn from soil, water and air, and returned on
  death.
- Light as a resource, which means light needs to exist: a placeable source, and
  occlusion. Tranche 4 needs this too.
- Death, fall, decay — closing the loop back into M3.2.

**Targets.** Growth consumes exactly the matter it gains; a plant deprived of
water, light or nutrients dies rather than stalling forever; dead matter re-enters
the soil.

## M3.5 — A closed terrarium

**Intent.** The whole thing, sealed, running on its own. The proof.

- A sealed jar containing soil, water, air, and life, reaching a dynamic
  equilibrium.
- Day/night or seasonal driving, if the evidence says the system needs a driver.
- Recovery from disturbance: knock it, and watch it come back.

**Targets.** The sealed system survives a very long run unattended; all conserved
quantities hold exactly; after a disturbance the system returns toward equilibrium
rather than collapsing or running away.

---

# Tranche 4 — The glass pane

**Intent.** The pane through which a person sees the small world, and reaches into
it. Everything before this is invisible without it; this tranche is where the work
becomes the deliverable.

A viewer exists from tranche 1 — this tranche is where it stops being a debugging
tool and becomes the product.

**Serving the north star.** This is the second north-star statement, directly: a
terrarium people can see on their screens and interact with. It is also the final
test of the first statement — a person poking at the world is the most creative
adversarial tester there is, and every place the universe isn't full enough will
be found by someone reaching for something we didn't anticipate.

**Tranche targets.**

- A person who has never seen it can open it, understand what they're looking at,
  and change something within a stated number of seconds.
- Interactive frame rate at the reference world size, measured.
- Every scenario in tranches 1–3 is loadable and watchable from the pane.

## M4.1 — Seeing

**Intent.** Look at the world and understand it.

- Materials rendered legibly; the world readable at a glance.
- Overlays for the invisible: temperature, pressure, moisture, gas composition,
  nutrients. Half of believability is being able to *check* the world is real.
- Camera, zoom, pan.
- Light and shadow, once M3.4 needs light to exist anyway.

## M4.2 — Time

**Intent.** Control over the world's clock — as much a debugging instrument as a
feature.

- Play, pause, single-step, fast-forward.
- Reset, and reset-to-a-saved-state.
- Long-run mode: leave it running and come back to it.

## M4.3 — Reaching in

**Intent.** Interaction. The point at which it stops being a film and becomes a
place.

- Placing and removing matter; brushes and sizes.
- Placeable entities: the hot entity, the cold entity, light sources, sources and
  sinks.
- Igniting, breaching, flooding, seeding.
- Inspecting a cell: what is this, how hot, how wet, what's dissolved in it.

## M4.4 — The two worlds

**Intent.** The same universe, two framings, because they want opposite things.

- **The terrarium**: sealed, gentle, alive, slow. No coal, no explosions, no
  boiling. Something to keep and return to.
- **The sandbox**: everything unlocked — fire, steel, explosions, weather. Somewhere
  to break things and find out what the universe does.
- A scenario browser: every primitive scenario from every tranche, watchable.

## M4.5 — Believability pass

**Intent.** The last mile, judged against the north star and nothing else.

- Presentation: the pane itself, the framing, the feel.
- Performance under real interaction.
- An adversarial pass by the hardest available critic: a person given no
  instructions and left alone with it, and every place they reach for something
  that isn't there recorded as a finding.

**Targets.** Stated at planning, against the north star — this is the milestone
where "believable" has to be turned into something measurable, and doing that
honestly is part of the work.

---

## Beyond the plan

Four closed tranches is not the end of the project. It is the first point at which
the whole planned scope exists to look at — and the right move then is a planning
pass with the north star in hand, deciding whether the next thing is a new tranche,
a return pass deepening an earlier one now that the later ones have revealed what
it should have supported, or something none of this anticipated.

See `cycle-tranche`, step 5.
