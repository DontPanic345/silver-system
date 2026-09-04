# Water & Phase Change — Acceptance Criteria

**Status: awaiting final sign-off.** Produced by `/tdd-acceptance`. Once you confirm,
this is the frozen input to `/tdd-cycle`.

Decisions locked in this revision: (1) one shared incompressible velocity field with
phase fractions advected through it — buoyancy for vapour, a settling velocity for
liquid, an optional divergence source at boiling cells; (2) tolerance percentages
accepted as starting points, a phase may propose better; (3) the dye demo is
**removed**, not preserved; (4) grid-size AC left as "under 16ms at the shipped size,
state the size"; (5) no additional scope (no separate pressure field, no dissolved
gases, no freezing, no multiple liquids, no game layer).

Also revised: the scenario layer is pulled forward to before the phase-change
physics, so every later round has a way to be seen; and any criterion that would
have needed a human to rate the visuals is restated as **"a scenario exists that
would show this behaviour, and these measurable proxies hold"**.

---

## Intent statement

Turn the stable-fluids demo into a **water simulation with phase change**. A closed
box holds air, liquid water and water vapour, each with a temperature. Heat the
water and it boils; the vapour rises, cools against something cold, condenses, and
rains back down. The point is a *closed water cycle driven by physics* rather than
scripted events — mass and energy go round the loop and are conserved to a good
approximation.

The user opens the GitHub Pages site, picks a named scenario from a list, and
watches it play — the same scenarios the automated suite asserts against — with
play/pause/step/reset and a live readout of the conserved quantities. Plus a sandbox
for painting water and dropping heat/cold sources freely.

The capstone is one scenario: a pool of water, a heat source beneath it, a cold
source at the ceiling, and a visible boil → rise → condense → rain loop.

### Illustrative vs load-bearing

- **"Conserves mass and energy to a good approximation"** — the behaviour is
  "nothing appears or vanishes as the cycle runs, and drift does not grow without
  bound". The percentages below are starting points; a phase that finds one wrong
  for the scheme reports it and proposes a number, rather than loosening it silently
  or contorting the code to hit it.
- **Specific temperatures and the grid size are illustrative.** What matters is that
  the relationships hold — boiling above a threshold, condensation below one — and
  the numbers are legible on screen.
- **"Both fluids simulated as fluids"** means liquid and gas both move by the solved
  flow field, not by ad-hoc falling rules. It does **not** mean two separate
  momentum fields — decision (1) above.
- **Qualitative behaviours** — "looks like rain", "the pool has a flat surface", "the
  cycle sustains" — are validated by *a scenario that exhibits them, playable on the
  page*, together with the strongest numeric proxies that are not brittle. They are
  **not** validated by asserting exact interior state, nor by a human scoring the
  visuals. Where the proxies cannot be met without distorting the physics, the phase
  **halts** and reports (a `npm run shot` PNG path is fine as evidence).

### Test-suite shape

The user asked for a practical, non-brittle suite. Prefer assertions on
**conservation laws, monotonic trends, orderings, thresholds, and centre-of-mass
direction** over exact interior values. Pinning a specific cell's temperature to
several decimals is the failure mode to avoid. Every scenario earns its assertions;
do not re-assert the same invariant across five scenarios.

---

## Acceptance criteria

Ordered so each round builds on the last. Rounds 1–3 are groundwork with no
user-visible change. Rounds 4–5 bring up the scenario layer. Rounds 6–9 are the
physics, ending at the capstone.

### Round 1 — Conservative transport (groundwork)

1. In a closed domain with no sources or sinks, the total of an advected scalar
   changes by **less than 0.5% over 500 steps**, for any velocity field the solver
   produces. *(Current advection loses ~15% over 200 steps; every conservation
   claim downstream depends on fixing this first.)*
2. Advecting a scalar never produces a value outside the range present in the field
   before that step — no new maxima, no negative amounts.

### Round 2 — Temperature field (groundwork)

3. Each cell carries a temperature that is advected by the flow and diffused by
   conduction.
4. In a closed domain with no heat sources, total thermal energy drifts **less than
   1% over 500 steps**.
5. A hot region beside a cold region equalises monotonically: the hottest cell never
   gets hotter, the coldest never gets colder, and the two converge.

### Round 3 — Buoyancy (groundwork)

6. A warm blob released in an otherwise uniform domain has its centre of mass move
   up; a cold blob's moves down.
7. With uniform temperature and density everywhere, no bulk motion develops from
   buoyancy alone.

### Round 4 — Scenarios as shared data

8. A scenario is **declarative data** in one module: initial field state, entity
   placements, run duration, and its list of assertions.
9. That same module is consumed by **both** the headless suite and the web page —
   adding a scenario requires no change to either consumer.
10. The headless suite runs every scenario's declared assertions.
11. At least two scenarios exist that exercise rounds 2–3 (e.g. "warm plume rises",
    "hot region meets cold region").

### Round 5 — Scenario player on GitHub Pages

12. The published page lists the available scenarios by name and description.
13. Selecting a scenario loads it from its initial state and plays it, with
    **play/pause, reset, and single-step** controls.
14. The page shows live readouts of the conserved quantities relevant to the loaded
    scenario (total energy now; total water once that channel exists), so drift is
    visible while the scenario plays.
15. A **sandbox mode** lets the user paint field state and reset to empty. (Placing
    entities joins the sandbox in round 8.)
16. The renderer draws whatever channels the sim currently has; adding a channel in
    a later round changes the renderer, not the page structure or the scenario
    format.

### Round 6 — Water, vapour, air and phase change

17. Cells carry fractions of **liquid water**, **water vapour** and **air**, summing
    to the cell's capacity; air is tracked explicitly so vapour displaces it rather
    than appearing from nowhere.
18. Total water substance (liquid + vapour) in a closed domain is conserved to
    within **1%** across any amount of boiling and condensation.
19. Liquid above the boiling threshold converts to vapour, and that conversion
    **removes latent heat** — the cell ends cooler than the same cell in an
    otherwise identical run with phase change disabled.
20. Vapour meeting the condensation condition converts to liquid and **releases
    latent heat** — the cell ends warmer than in the phase-change-disabled run.
21. Total energy (sensible + latent) in a closed domain with no sources drifts
    **less than 2%** across a full boil-then-condense round trip.
22. Air alone is conserved: its total is unchanged by any phase change.
23. A scenario exists that boils a body of water; measurably, its total vapour
    fraction rises while heat is applied.

### Round 7 — Liquid falls, gas rises

24. A scenario exists that would show liquid water falling through gas and collecting
    below; measurably, the liquid's centre of mass moves **downward** over the run
    and total water is conserved.
25. A scenario exists that would show vapour rising through air; measurably, the
    vapour's centre of mass moves **upward** with no explicit "rise" instruction —
    it falls out of the buoyancy force alone.
26. A liquid body at rest with no heat applied is **stable**: total liquid constant
    within 0.5%, centre of mass stationary within one cell, and peak speed stays
    bounded over 500 steps — no creep, no runaway, no growing oscillation.

### Round 8 — Placeable entities

27. A **heat source** placed in the domain raises the temperature of nearby cells at
    a bounded rate; a **cold source** lowers it. Neither creates nor destroys water
    or air.
28. Sources can be placed and removed at arbitrary grid positions, and the **net
    energy** each adds or removes is reported, so energy accounting stays checkable
    (closed-domain drift from AC 21; with sources, drift is measured against the net
    energy the sources injected).
29. The sandbox lets the user place and remove heat and cold sources, and scenarios
    can declare them (per AC 8).

### Round 9 — Capstone

30. A scenario **"boil and rain"** exists and is playable from the page: a closed box
    of air over a water pool, a heat source beneath the pool, a cold source at the
    ceiling.
31. Over the capstone run, all of the following measurable proxies hold:
    - total vapour rises while boiling is occurring;
    - vapour reaches the upper region of the box;
    - liquid re-appears in the upper region and the liquid total there **rises then
      falls** (condensation, then fall-back);
    - total water (liquid + vapour) is conserved within **1%**.
32. Over the capstone run the pool **does not fully empty** and **does not exceed its
    initial volume** beyond tolerance, and **at least one complete vapour transit**
    (rise then return to the pool region) occurs within the scenario's duration.

### Cross-cutting — must hold at every round

33. **Determinism:** the same scenario run twice for the same number of steps
    produces bit-identical state.
34. **Performance:** a simulation step stays under **16ms** at the shipped grid size,
    and the shipped grid size is stated in the code and the report. (128–160²
    expected once temperature and three phase channels are in.)
35. `npm test` runs the whole suite headless with no browser required; the scenario
    page is verifiable via the existing `npm run shot` screenshot path.

---

## Interpretation calls made

- **Rounds 1–3 are groundwork I added,** not in the user's prompt. Round 1 exists
  because the current advection measurably loses ~15% of an advected scalar over 200
  steps — no conservation AC is meaningful until that is fixed.
- **The scenario layer (rounds 4–5) is pulled ahead of the phase-change physics.**
  The player then exists before the rounds whose output most needs looking at, and
  each later round ships a scenario that demonstrates it.
- **Scenarios are the tests.** One declarative definition, two consumers (AC 8–10).
  This is the strongest structural call in the document — it is what stops the demos
  and the tests being written twice.
- **Air is an explicit channel** (AC 17, 22), not "whatever isn't water", so vapour
  displaces something real and air conservation is checkable.
- **Human-judgement ACs restated as scenario + proxies** (AC 24, 25, 31, 32) at the
  user's instruction. The automated gate is the numeric proxy; the "would show this"
  half is satisfied by the scenario being playable on the page.
- **Terrarium as reference, not template.** Its water cycle is rule-based off a
  day/night clock with no real temperature field and no latent heat. This sim goes
  further, so the terrarium informs the *mixture-per-cell* representation and the
  closed-system-invariant habit, nothing more.

---

## Process instructions for `/tdd-cycle`

Not acceptance criteria — how this cycle should run.

### Debug report from every phase

Append to every phase prompt (Red, Green, Refactor), verbatim:

> **Additionally, end your report with a short `## Cycle debug` section.** This is
> the first run of this TDD skill set and we are collecting data to improve it. Keep
> it to a handful of bullets and be blunt — it is more useful when unflattering.
> Cover whatever applies:
> - What took a disproportionately long time, and why.
> - Snags: things unclear in your prompt, information you needed but weren't given
>   or had to go find yourself.
> - Missing or awkward tooling; anything you wanted and didn't have.
> - Mistakes you made and had to back out of.
> - Anything about the AC list or intent statement that was hard to work from.
> - Anything you were unsure was in your remit.
>
> If nothing notable happened, say so in one line rather than padding.

The orchestrator collects these and, at close-out, compiles them into a single
**"how to improve the TDD cycle"** summary for the user — grouped by theme, not a
transcript.

### Nightshift operating mode

Runs unattended. Therefore:

- **Halting for a genuine reason is preferred over plowing on.** A clean stop with a
  clear explanation is a good outcome; an implementation that satisfies an AC's
  letter while missing its intent is not.
- Judgement calls **within** the intent: make the best call, record it in the
  report, keep going. Don't halt on small stuff.
- Decisions that change scope, contradict the intent statement, or need information
  not in this document: **halt and surface**.
- Rounds 6–9 involve tuning. If a scenario's numeric proxies cannot be met without
  distorting the physics, **halt and report** with a `npm run shot` PNG path as
  evidence — do not force the assertion or weaken it to pass.
- Tooling / environment failure: **halt immediately**, no workarounds.
- Work directly on `main` and commit as you go, per the repo's convention. Commit
  message attribution trailer is set by the session.

### Environment notes

- Dependency-free browser sim, no build step. `npm test` → `node test/fluid-probe.js`
  (rename/extend as the suite grows). `npm run shot` renders the page headless and
  writes a PNG — **do not read screenshots into context**, they are for the user.
- Solver: `js/fluid.js` (DOM-free ES module). Driver: `js/main.js`. The dye channel
  and its palette are being removed in round 6 or earlier — no need to preserve them.
- Node (linux) and npm work. Playwright is installed globally but only reachable via
  CommonJS `require` (`NODE_PATH=/usr/local/lib/node_modules`, or `createRequire` —
  see `scripts/shot.js`).
- GitHub Pages serves the repo root; `.nojekyll` is present. The shelved terrarium
  sim is under `terrarium/` for reference only.

---

## Next step

Hand this file to `/tdd-cycle`. `/tdd-acceptance` stops here — it does not write
tests, code, or start the cycle.
