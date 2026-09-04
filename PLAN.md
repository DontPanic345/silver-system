# Plan

A small world inside a much larger universe. Four tranches, in order. Each is
broken into milestones by `/cycle-plan` when it is reached — not up front.

The language is Rust. GPU execution is the intended direction of travel.

---

## Tranche 1 — Physics

**Intent.** A grid where matter moves because of forces, not because a rule said
so. Pressure and fluid dynamics are the point of this tranche; the earlier
JavaScript work (`stable-fluids/`) reached temperature and buoyancy and halted on
pressure, so this tranche re-treads that ground on a solver that can carry it.

Must cover:

- pressure
- fluid dynamics
- temperature
- the element set the later tranches will need

**Primitive scenarios.** One or a few per element and behaviour, each with its own
empirical pass/fail check and no human in the loop:

- a central column of sand falls into a pile
- a central column of something else does *not* stay still
- a resting pool of water stays still
- a column of water falls and finds a level
- cool air falls
- **the U-pipe**: a U-shaped pipe surrounded by air, one arm filled with water —
  the water comes to a level across both arms

Some of these need not run in the general suite; they exist for regression and for
completeness. Completeness is valued. Keep them.

---

## Tranche 2 — Chemistry

**Intent.** Matter that changes what it *is*, not just where it is.

- state changes
- burning wood
- iron and carbon → steel; rust by more than one route
- dissolved gases and moisture: gas in solids, moisture in solids, water in soil,
  air in soil, air dissolved in rubber escaping a balloon
  *(open: how much of this is really physics and belongs in tranche 1)*
- explosions — particles fly naturally and land naturally
- the full water cycle

---

## Tranche 3 — Biology

**Intent.** Things that live off the cycles the first two tranches built.

- fungus, humus, soil, bacteria, slime mould, and the like
- the carbon cycle and the nitrogen cycle

---

## Tranche 4 — The glass pane

**Intent.** The pane through which the player sees the small world.

In practice some of this is needed **early** — scenarios have to be watchable long
before this tranche is reached. What lands here is the fleshed-out version.

- interaction with a sandbox world and with a terrarium world
- the terrarium has little use for coal, explosions and boiling; the sandbox needs
  all of it
- full human interaction: placing entities (a hot entity, a cold entity), placing
  light sources

---

## Open design questions

**Fully-mixed gas regions.** Would modelling a contiguous gas region as one entity
tracking component fractions — rather than a field per component — buy the solver
anything? Same question for a gas dissolved in a body of water, assumed mixed
throughout.

Current read: the win is collapsing the elliptic pressure solve into a region-level
equalisation rule. The costs are that connected-component labelling is a global
graph pass that maps badly to the GPU, and that uniform composition transports
composition at infinite speed — which deletes plumes, stratification, diffusion
fronts and local oxygen depletion, i.e. most of what tranche 2 is trying to grow.
Revisit as an optional optimisation for quiescent enclosed volumes, not as a
substrate.
