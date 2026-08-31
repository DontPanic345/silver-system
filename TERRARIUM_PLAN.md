# Terrarium — Roadmap

Turning the current mixture-based falling-sand sim into a terrarium: a sealed
glass jar with its own light cycle, a closed water cycle (evaporation →
condensation → rain), and plants that germinate, grow, photosynthesize, and
decay. The goal is a small system that can run itself indefinitely once
sealed — not just a prettier sandbox.

## Conceptual shift from where the sim is now

The current engine is *reactive*: cells fall, mix, and burn only in response
to what you paint. A terrarium needs to be *alive on its own* — driven by an
internal clock (day/night), with matter cycling through states (liquid water
→ vapor → dew → rain, dead plant → biomass → new plant) with nothing added
or removed once the lid is sealed. That's a real shift, but it builds
directly on what's already there: the composition-per-cell model, the
mass-conservation discipline from the diffusion fix, and the "fire is a flag
on flammable matter" pattern are exactly the right shape to extend.

## Phased plan

### Phase 0 — Jar & light cycle (small, do first)
Everything else needs a light source and a hard boundary.

- A `GLASS` material: solid like stone (blocks movement) but tracked
  separately for rendering (translucent) and later, for condensation.
- A jar/dome shape replacing the open rectangular canvas — walls plus an
  optional sealed lid.
- A global day/night clock driving a `lightLevel` (and later a light
  *direction*, for phototropism) that everything downstream reads.
- Payoff even alone: ambient dimming at "night," a visibly enclosed scene.

### Phase 1 — Closed water cycle (medium — the first real "wow")

- Add a 5th gas channel, `WATER_VAPOR`, alongside N2/O2/CO2/smoke.
- **Evaporation**: exposed liquid water converts a bit into vapor each tick,
  scaled by light level — already rises on its own since the buoyancy rule
  is generic (just tune its density like smoke's).
- **Condensation**: vapor touching the glass (especially at "night," cooler)
  condenses back into liquid on the inner wall — droplets that then slide
  down the glass and pool via the *existing* gravity/density code, no new
  movement logic needed.
- **Rain**: enough condensation on the underside of the lid detaches and
  falls back into the soil.
- Total water (liquid + vapor) becomes a checkable invariant across the
  whole cycle, the same way mass conservation was verified for the mixing
  engine — good regression test.

### Phase 2 — Plants (large — the centerpiece, and the part that doesn't fit the pure-diffusion model)

- Plants can't just be composition that diffuses — a stem needs to hold its
  shape and grow *purposefully*. Plan: a lightweight sparse "organism" layer
  (a list of Plant structs, each owning a set of grid cells tagged
  stem/leaf/root) layered on top of the grid, the same way fire is a flag
  layered on composition today — not a rewrite of the core model.
- A new `SEED` material and a `PLANT_TISSUE` channel distinct from loose
  biomass (so living structure doesn't get diffusion-blended into the soil
  the way dead matter should).
- **Germination**: a seed in soil with adequate moisture sprouts after some
  ticks.
- **Growth**: stem extends toward light (phototropism), roots extend into
  moist soil and draw down water + nutrients from neighboring soil cells —
  direct interaction with the existing composition arrays.
- **Photosynthesis**: lit leaf cells consume CO2 + water, emit O2 into the
  adjacent air, add to the plant's growth energy — this is the payoff of
  already having real CO2/O2 gas channels.
- **Death & decay**: a plant that dries out or burns loses "alive" status,
  browns, and slowly rots back into loose biomass over many ticks — closes
  the nutrient loop so the jar can support a second generation, not just
  one.

### Phase 3 — Temperature/seasons (medium, optional depth)

A scalar temperature field shaped by day/night and evaporative cooling,
feeding back into evaporation rate and plant growth/dormancy. Mostly tuning,
plus one new field and a frost/fog tint.

### Phase 4 — Decomposers (stretch, probably skip unless requested)

Small wandering critters (isopods/worms) that speed up decay. Nice flavor,
not required for the thing to read as a terrarium.

### Phase 5 — Seal & balance pass (small code, real iteration time)

A "seal lid" toggle plus a debug readout of total water/carbon/biomass in
the closed system over time, then empirical tuning of
evaporation/photosynthesis/decay rates so a small planting can run
indefinitely without drying out, drowning, or suffocating on its own
CO2/O2.

## Cross-cutting concerns

- **Performance**: a 5th gas channel + plant tissue channel + an
  organism-update pass adds real per-tick cost. Likely fine at current grid
  sizes, but plant growth ticking every frame is unnecessary — update it
  every few frames instead of shrinking the grid.
- **Invariant discipline**: every new cycle (water, carbon) should be
  mass-conserving by construction, verified the same way the earlier
  diffusion bug was caught — via direct composition readback, not
  eyeballing screenshots.
- **Save/restore**: once building a jar takes real waiting time (growth,
  water cycles), losing it on refresh gets painful — a localStorage
  snapshot is worth pulling forward, probably into Phase 1 or 2 rather than
  leaving it to the end.

## Recommended path

Phase 0 → 1 → 2, in that order. Jar+light and the water cycle are
self-contained, testable checkpoints that already feel like a terrarium
before a single plant exists, and Phase 2 (plants) is where the real
complexity and design risk lives — worth having the water cycle solid
underneath it first.
