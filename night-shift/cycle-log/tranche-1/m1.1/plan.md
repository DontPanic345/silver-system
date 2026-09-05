# M1.1 plan — Substrate and harness

**Planned:** 2026-09-05T10:22+12:00

## 0. What was folded forward

Read: `cycle-log/tranche-1/plan.md`, `cycle-log/tranche-0/closeout.md`,
`cycle-log/tranche-0/m0.4/closeout.md`, `src/lib.rs`, `src/math.rs`,
`src/timestep.rs`, `Cargo.toml`, `src/bin/native_viewer.rs`, `www/index.html`,
`tests/e2e/canvas_rectangle.test.mjs`, `tests/native_fallback.rs`.

- `Scalar` (`f32`), `Vec2`, `GridIndex` (`src/math.rs`) and `FixedTimestep`
  (`src/timestep.rs`) exist, are unit-tested. `Scalar`/`FixedTimestep` are
  already exercised by real running code (M0.1's `tick_and_draw`).
  `Vec2`/`GridIndex` are **not** — named explicitly in tranche 0's and
  tranche 1's plans as this milestone's honest gap to close, via a real
  grid, not forced wiring. This milestone's own grid type is that real grid.
- A working wasm-bindgen canvas pipeline already exists end to end:
  `src/lib.rs` (`#[wasm_bindgen]` exports, `paint_rect`, `render_frame` for
  the native-target-independent buffer), `www/index.html` (loads the wasm
  module, drives a `setInterval` loop), `src/bin/native_viewer.rs` (native
  fallback, PNG output via `render_frame`), a Playwright e2e test
  (`tests/e2e/canvas_rectangle.test.mjs`) and a native fallback test
  (`tests/native_fallback.rs`). Both paths share `render_frame` as their one
  source of pixel truth. M1.1's renderer builds on this — a new "paint the
  grid" function alongside `render_frame`/`paint_rect`, reusing the same
  wasm export + native buffer + Playwright/native-test double-check
  pattern — not a new pipeline.
- GitHub Pages deploy is live (`.github/workflows/deploy-pages.yml`), gated
  on `cargo test` passing. No change needed to CI mechanics this milestone;
  new files just need to build cleanly under the existing `wasm32-unknown-
  unknown` + native targets.
- Tranche 0 fixed two process defects, now live in the skills: fork→cold-
  agent for self-dispatch (`cycle-contract` §3a), and strict one-round-at-
  a-time dispatch (`cycle-milestone`/`cycle-round`). Both already read and
  will be honored below — no further folding needed on that front.
- No exit ramps pending from tranche 1 planning; this is the tranche's
  first milestone, nothing to patch back into.

## 1. Intent, restated

The bones: how a cell is represented, how a step happens, how anything gets
measured, and how a human can watch it. Every later milestone in this
tranche (granular solids, liquids, pressure, gases, temperature) adds
*behaviour* to cells this milestone defines the shape of — get the shape
wrong here and every later round pays a retrofit tax. This milestone adds
no real physics itself: the step function this round builds is a generic
engine (advance the grid by one fixed tick, whatever "advance" currently
means), not gravity or pressure — those are M1.2 onward's job. What must
exist by the end of this milestone: a `Grid` (structure-of-arrays, double-
buffered, `GridIndex`-addressed), a `Material` table as data, a `Scenario`
definition consumed by both a headless runner and the renderer, a headless
JSON-measurement path a test can assert on with no human in the loop, and a
minimal renderer that paints the grid so a human *can* watch, if they
choose to.

**Serving the north star.** Nothing in this tranche is checkable — by a
test or by a person — without this milestone. The north star's first half
("a large universe... genuinely full") depends on a representation that can
hold an actually-large grid without being rebuilt later; the second half
("a terrarium people can see") depends on the renderer this milestone
ships, reused unchanged by every later tranche-1 milestone rather than each
milestone inventing its own way to look at its own scenario.

## 2. What this milestone needs beyond PLAN.md's/tranche-1 plan's sketch

Applying the "does this milestone's own intent fail without it" test:

- **Closing `Vec2`/`GridIndex`'s unused-code gap is not optional** — the
  grid this milestone must build literally cannot be `GridIndex`-addressed
  without calling `GridIndex`, and cell-to-cell forces (arriving M1.2+, but
  the type needs to exist now) need `Vec2`. This is the milestone's own
  first requirement, not scope creep.
- **Nothing else is folded in.** The tranche-1 plan's M1.1 sketch (grid +
  material table + scenario harness + headless measurement + test tagging
  + minimal renderer) already matches what "the bones" needs. No reach
  items found worth naming beyond what's deferred below.

**Deferred, not folded in:**

- Any real physics (gravity, pressure, transport) — M1.2 onward's job. This
  milestone's step function is a generic "advance one fixed tick" hook;
  what it does to cell contents is empty/trivial by design this round.
- Overlays, camera, pan/zoom, interaction — tranche 4's job. The renderer
  here stays a flat top-down paint of material colour, nothing more.
- GPU groundwork specifics — M1.7's job, though structure-of-arrays chosen
  here is a decision that also happens to serve that later goal (not built
  *for* it, but not working against it either).
- `dissolves_in`/`permeable` material-table hooks — tranche 2's job
  (M2.5). The material table's "room to grow" requirement means the struct
  shouldn't make adding those fields later structurally painful, but no
  such field is added this milestone.

## 3. Milestone targets (measurable)

Restated from `PLAN.md`/tranche-1 plan, sharpened:

1. **A scenario runs headless and emits JSON measurements.** At least one
   scenario definition exists; running it headless (no browser, no human)
   produces a JSON document with at least: total mass per material, cell
   counts per material, and the tick count reached. A test asserts on
   specific values in that JSON, not just "it parsed."
2. **The same scenario renders.** The same `Scenario` value (one
   definition) is consumed by a renderer that paints a recognizable frame
   — reusing the existing wasm-bindgen canvas path and the native-binary +
   Playwright/native-test fallback pattern already proven in tranche 0.
   "Renders" is checked headlessly the same way M0.1 was: read back real
   pixel data (via the existing Playwright e2e pattern) or the native PNG
   buffer, not a human looking at a screenshot.
3. **The reference grid steps within a stated per-step budget.** A grid
   size is chosen and recorded (with reasoning), a step of it is timed on
   this dev machine, and the number is written down in this milestone's
   closeout so M1.7 can hold later milestones to it.
4. **`Vec2`/`GridIndex` are exercised by real running code**, not just
   their own unit tests — checked directly (grep + a passing test that
   fails if the grid stops using them).
5. **Test tagging and a fast path exist** from the first round: at minimum
   a way to run "fast" tests (unit + the new grid/scenario tests) separate
   from anything slow (the Playwright e2e path, a long-running headless
   scenario), so later milestones don't pay e2e-test latency on every
   `cargo test`.

## 4. Rounds (starting position — expect revision)

Ordered so each is buildable on the last; only round 1 is judged risky
ahead of time.

### Round 1 — Grid and material representation (risky: full Red/Green/Refactor)
Structure-of-arrays, double-buffered `Grid` addressed by `GridIndex`; a
`Material` struct (density, viscosity, heat_capacity, conductivity, phase,
colour, room to grow) and a small material table as data (not per-material
code — e.g. a `Vec<Material>` or similar indexed by a `MaterialId`). Closes
the `Vec2`/`GridIndex` gap. Test tagging/fast-path convention established
here (Red's job, per the milestone's target 5) so every later round
inherits it rather than retrofitting.

**Why risky:** this is the shared primitive every later round in this
milestone, and every later milestone in the tranche, builds on directly —
exactly `cycle-plan`'s "blast radius reaches past this round" criterion.
Getting the grid's shape wrong here is the expensive mistake the whole
milestone exists to avoid.

### Round 2 — Fixed-timestep step function + Scenario definition (single-pass, tentative)
Wire `FixedTimestep` (already exists, already tested) to a `Grid::step`
(or free function) that advances the grid by whatever the current step
means (a no-op/identity pass is acceptable and honest at this point — real
per-material behaviour is M1.2+). Define `Scenario`: an initial grid state
plus whatever parameters a run needs, as one data value both a headless
runner and a renderer can consume. No real physics content; this round is
about the *shape* of stepping and scenario definition, matching the
milestone's own intent statement.

### Round 3 — Headless measurement (single-pass, tentative)
A headless runner that takes a `Scenario`, steps it some number of times,
and emits JSON measurements (mass per material, cell counts, tick reached).
A test asserts on specific values, no human judgement in the assertion
path. This is milestone target 1.

### Round 4 — Minimal renderer (single-pass, tentative)
Extend the existing wasm-bindgen canvas pipeline (and native fallback) to
paint a `Scenario`'s grid — flat top-down, material colour per cell,
nothing fancier. Reuses `render_frame`'s pattern (pure function, callable
from wasm export and native binary alike) and the existing Playwright e2e
+ native-fallback-test double-check. This is milestone target 2.

### Round 5 — Reference grid size and performance budget (single-pass, tentative)
Pick and record the reference grid size (with reasoning — large enough to
be a meaningful stand-in for "the large universe", small enough to time
sensibly on this dev machine), time a step of it, record the per-step
budget number and how it was measured. This is milestone target 3, and the
first fixing of tranche target 6's number.

**Ordering dependency to watch:** round 3 (headless measurement) and round
4 (renderer) both depend on round 2's `Scenario` type existing first, and
round 5's timing measurement depends on round 2's step function existing
to time. Round 1 is the hard prerequisite for everything after it.

## 5. Report

**Push on vs. patch back.** Push on — tranche 1 closed no exit ramps
before this milestone started, and the tranche plan's own note ("M1.1
building an actual grid *is* the honest close [of the Vec2/GridIndex
gap]") is exactly what round 1 does; no patch-back needed.

**Deferred, and where recorded.** Real physics (§2 above, to M1.2+);
overlays/camera/interaction (§2, to tranche 4); GPU-groundwork specifics
(§2, to M1.7); dissolved/permeable material fields (§2, to M2.5) — all
named in this file's §2 so the relevant future planner finds them without
re-deriving.

Round list above is a starting position; re-planned after each round per
`cycle-milestone` §2.
