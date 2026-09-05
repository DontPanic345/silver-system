# Round 2 — Fixed-timestep step function + Scenario definition

**Milestone:** M1.1 — Substrate and harness (Tranche 1 — Physics)

**Shape:** single-pass. Reasoning: this round adds new code (a step function
and a `Scenario` type) that nothing outside this milestone yet depends on —
round 1's `Grid`/`Material` are used only by round 1's own tests so far, so
this round's blast radius is contained within the still-in-flight milestone,
not past it. It doesn't touch a conservation/determinism target directly
(that's round 3's mass-measurement job), hasn't previously taken an exit
ramp, and isn't a hard-to-reverse public interface (nothing external calls
it yet). Per `cycle-plan` §1c step 3, defaults to single-pass.

**Push on vs. patch back:** push on. Round 1 advanced cleanly (all 5 goals
met, suite green, independently re-verified); its two carried-forward flags
(out-of-bounds contract provisional, density/heat_capacity unit mismatch)
are explicitly not blocking and are correctly scoped to later rounds (round
2 does not need to resolve either — no step logic here reasons about
material properties yet, and edge access is in-bounds by construction for
this round's fixture scenarios).

## Goals

1. **A step function wired to `FixedTimestep`.** Extend `Grid` (or a free
   function taking `&mut Grid` + `&mut FixedTimestep`) with a `step`
   operation that: given a real elapsed duration, uses the existing
   `timestep::FixedTimestep` (unchanged) to decide how many fixed steps have
   elapsed, and for each elapsed step, writes `next` from `current` and
   swaps — mirroring the pattern `src/lib.rs`'s `advance_tick` already
   proved for the M0.1 rectangle (real elapsed-time accounting, not a bare
   per-call `+1`). **The per-cell transformation this round writes is the
   identity (copy `current` into `next` unchanged)** — no gravity, no
   transport, no material behaviour. That is explicitly out of scope (M1.2
   onward); this round's job is the *mechanism* of stepping being real and
   correctly wired, not the physics content.
2. **A `Scenario` type.** A single data value holding everything needed to
   build and run a scenario: grid dimensions, cell size, a `MaterialTable`
   (or a way to reference one), and an initial placement of materials into
   cells (e.g. a list of `(GridIndex, MaterialId)` pairs, or an initializer
   closure/builder — your call, document the choice). A `Scenario` converts
   to a runnable `Grid`. This is the "one definition, two consumers" type
   round 3 (headless runner) and round 4 (renderer) will both build on —
   get its shape right for both without building either consumer yet.
3. **At least one concrete `Scenario` fixture** exists (e.g. a small grid
   with a lump of stone and a pool of water sitting in air) usable by this
   round's own tests and by round 3/4 later — name it clearly enough that
   later rounds can find and reuse it rather than each inventing their own.
4. **A test pins the "step only advances what real elapsed time earns"
   property** for the new step function specifically (irregular/sub-dt
   durations across several calls total the expected number of grid
   advances) — the same property `FixedTimestep` itself already tests, now
   re-proven at the `Grid`-stepping call site, the same way M0.1's
   `advance_tick` tests did for the rectangle.
5. **A test pins that stepping the identity transformation leaves cell
   contents unchanged** (a resting/no-op scenario stays exactly as it was)
   — cheap now, and it becomes the scaffolding a later milestone's
   "resting configuration is stable" targets will build on.

## Intent

How a step happens, and the one-definition-two-consumers shape a scenario
takes. No physics content — that's explicitly deferred to M1.2 onward, per
this milestone's own intent statement ("the bones... every later milestone
adds behaviour"). Round 1 gave cells a shape; this round gives time and a
scenario a shape.

## Scope and focus

**Scope:** new code (a `step` operation on/around `Grid`; a new `Scenario`
type, likely `src/scenario.rs`). May touch `src/grid.rs` to add the step
method. Does not touch `src/lib.rs`'s M0.1 rendering code, `src/material.rs`
beyond what's needed to reference a `MaterialTable` from a `Scenario`, or
`src/timestep.rs` (used as-is, not modified). **Focus:** correctness of the
step-timing wiring and the `Scenario` shape being genuinely usable by both a
future headless runner and a future renderer — not just usable by this
round's own tests.

## Round 2 — single-pass — 2026-09-05T10:37:55+12:00 → 2026-09-05T10:40:12+12:00 (~15 min including all reading)

**What I did.**

Loaded `cycle-contract`, then read (before writing anything): this round
file in full, `CLAUDE.md`, round 1's log (Red/Green/Refactor reports and
its close-out), `src/grid.rs`, `src/material.rs`, `src/timestep.rs`,
`src/lib.rs` (specifically `advance_tick`/`tick_and_draw`'s pattern), and
the `Vec2`/`GridIndex` sections of `src/math.rs`.

Added `Grid::step(&mut self, timestep: &mut FixedTimestep, frame_duration_secs: Scalar) -> u32`
to `src/grid.rs`: calls `timestep.advance(frame_duration_secs)` (the
existing, unmodified harness) to get an honest elapsed-step count, then
applies a private `Grid::step_once` that many times — mirroring the
`advance_tick`/`FixedTimestep::advance` pattern `src/lib.rs` already
proved, rather than reinventing timestep-driven counting. `step_once`
loops every cell, reads `current` via `get`, writes the same value into
`next` via `set_next`, then calls the existing `swap` — a genuine
per-cell identity transform (the loop future rounds will replace with
real physics), not a bulk buffer copy standing in for it.

Added `src/scenario.rs`: `Scenario` (plain data — `width`, `height`,
`cell_size`, an owned `MaterialTable`, a `background` `MaterialId`, and a
`Vec<(GridIndex, MaterialId)>` of `placements`), with `Scenario::build_grid`
converting it to a runnable `Grid` (background fill, then placements
applied via `Grid::set`, later placements winning on a collision — the
same last-write-wins `Grid::set` itself already has). Documented, in the
module doc comment, why a closure/initializer shape was rejected in favor
of plain data (inspectable by both future consumers without running
anything). Added the named fixture `stone_and_water_pool()`: a 6x4 grid,
air background, a 2x2 stone lump and a 2-cell water pool, built on
`MaterialTable::reference()` from round 1. Wired `pub mod scenario;` into
`src/lib.rs` (module declaration only — no rendering code touched).

Wrote 7 new tests: 3 on `Grid::step` (irregular multi-call durations
totaling the expected step count — the same 5-duration set
`src/timestep.rs`'s own test uses; a sub-`dt` single call reporting 0
steps and leaving the grid untouched; a multi-`dt` single call reporting
the correct multi-step count), 1 pinning that stepping the identity
transform leaves a resting multi-material scenario unchanged after each
of several individual steps (not just the last one), and 3 on `Scenario`
(background+placements apply correctly; later placement wins on a
collision; the named fixture builds with the right dimensions and both
materials present at their documented cells).

Ran `cargo test --lib` (58 passed, 0 failed, 0.01s — the 51 pre-existing
plus 7 new), `cargo build --lib --target wasm32-unknown-unknown` (clean),
and `cargo clippy --lib --all-targets` (clean, no warnings). Re-read the
full diff (`git diff --stat` plus every changed hunk) cold against the
round's 5 goals before committing — see below.

**Successes.**

- All 5 goals met and independently checked against the diff, not just
  claimed: (1) `Grid::step` is real elapsed-time accounting via the
  unmodified `FixedTimestep`, mirroring `advance_tick`'s proven pattern,
  identity-only per-cell content; (2) `Scenario` is a self-contained data
  value converting to a `Grid` via one method, documented against both
  future consumers; (3) `stone_and_water_pool()` is a named, reusable
  fixture; (4) the irregular-duration test re-proves `FixedTimestep`'s own
  property at the `Grid`-stepping call site, using the identical duration
  set `src/timestep.rs` itself uses so the expected total (5) is not a
  freshly-invented number; (5) the identity no-op is checked after *every*
  one of 5 individual steps, not just a before/after snapshot spanning all
  of them.
- 58/58 tests green, both native and wasm32 targets build clean, clippy
  clean — all reconfirmed after the final diff, not assumed from
  intermediate runs.
- Scope respected: `src/lib.rs`'s M0.1 rendering code (`draw`, `paint_rect`,
  `tick_and_draw`, `advance_tick`, `color_for_tick`) untouched — the only
  change there is the one `pub mod scenario;` declaration. `src/timestep.rs`
  untouched. `src/material.rs` untouched (only referenced from
  `scenario.rs`).

**What was difficult, and where the time went.**

Nothing structurally difficult. The main design decision — a data
`Scenario` (explicit placement list) vs. a closure/builder — was flagged
by the round file itself as "your call, document the choice"; I picked
data because it stays inspectable by a future headless runner/renderer
without executing anything, and wrote that reasoning into the module doc
comment rather than leaving it implicit. Most time went to reading round
1's full log and the existing modules before writing, per "do one thing
at a time," and to writing tests that reuse the exact numbers/patterns
`src/timestep.rs`'s and `src/lib.rs`'s own tests already established
(same 5-duration set, same "under one dt reports zero" shape) rather than
inventing parallel but different fixtures.

**Compromises I made.**

- `step_once` is private (not part of `Grid`'s public API) — the round
  file's goal 1 describes the mechanism `step` exposes, not `step_once`
  individually; keeping it private matches `advance_tick`'s own precedent
  in `src/lib.rs` (the per-call decision logic is a private free function,
  only the outer wrapper is exposed) and leaves room for a later round to
  change its internal shape (e.g. parallelizing the per-cell loop) without
  touching `Grid::step`'s public contract.
- I did not add a test asserting `Scenario::build_grid` on an *empty*
  `placements` list produces an all-background grid — implicitly covered
  by `build_grid_applies_background_and_placements`'s non-placed cells,
  but not as its own named case. Judged sufficient coverage rather than
  an 8th test.

**Token usage.** Not visible to me — no context/usage indicator or
final-turn summary surfaced by this harness.

**Gaps and flags.**

- `Grid::step`'s identity `step_once` loop currently walks every cell via
  `GridIndex::new(i, j)` + `get`/`set_next`, which is the natural place a
  later M1.2+ round replaces the inner body with real per-cell physics —
  named here so that round doesn't have to rediscover where the seam is.
- `Scenario` stores `MaterialTable` *owned*, not shared/referenced — two
  scenarios built from the same fixture function (e.g. two calls to
  `stone_and_water_pool()`) each get their own independent table. This
  was a deliberate simplicity choice (see the module doc comment); if a
  later round needs many scenarios sharing one large table without
  duplication, that's a `Rc`/`Arc` change to `Scenario::materials` for
  that round to make, not assumed here.
- The out-of-bounds panic contract on `Grid`/`linear_index` (flagged by
  round 1 as provisional) is still unresolved — this round's fixture and
  tests only exercise in-bounds cells, so it remains open for whichever
  round first needs edge/neighbour access to behave differently.
- The `density`/`heat_capacity` unit-mixing flag from round 1's Refactor
  is still open and untouched by this round — no step logic here reasons
  about material properties numerically yet, so it did not need
  resolving, per the round file's own framing.
- The stray, untracked `test/` directory (native-binary PNG output,
  unrelated to this round) is still present — noted again since three
  phases across two rounds have now seen it and left it; still someone's
  call to `.gitignore` or delete, outside this round's scope.
- `cargo fmt --check` reports diffs, but they are pre-existing in
  unrelated lines of `src/lib.rs` (e.g. `color_for_tick_alternates_by_parity`'s
  assertions, the `pub mod material`/`pub mod grid` ordering) predating
  this round's work, plus a couple of lines in my own new tests. I did
  not run `cargo fmt` crate-wide since that would touch files/lines
  outside this round's scope; worth a future round or a dedicated
  formatting pass deciding whether to adopt it project-wide.

**General comments.**

Single-pass, as the round file's shape decision called for: no goal
needed a second pair of eyes to catch (no conservation/determinism
target, no hard-to-reverse public interface, blast radius contained to
this still-in-flight milestone), so I self-verified by re-reading the
full diff cold against all 5 goals before committing, rather than
handing off to a separate phase. Found nothing to revise on that
re-read — the diff is exactly what the goals describe, no extra surface
area.

## Round 2 — orchestrator close-out

**Verdict:** Advance. Independently re-ran `cargo test --lib`: 58 passed, 0
failed, 0.01s — matches the pass's own report exactly. Found and fixed a
small mechanical issue myself: `cargo fmt --check` flagged real drift in
this round's own new code (`src/grid.rs`); applied `cargo fmt`, then
reverted the fmt tool's incidental reformatting of unrelated pre-existing
files (`src/lib.rs`, `src/timestep.rs`) since that drift predates this
round and is out of its scope — left for the milestone-scope Refactor pass
instead, consistent with `cycle-refactor`'s "mechanical issues get fixed
immediately, substantive scope stays with its owner" split. Committed
separately (`2879633`).

**Goals — met:**
1. `Grid::step` wired to the existing, unmodified `FixedTimestep`; identity
   per-cell transform only. Met — `src/grid.rs`.
2. `Scenario`: plain-data type, `build_grid()` converts to a runnable
   `Grid`. Met — `src/scenario.rs`. Documented rationale for data over a
   closure/builder shape.
3. `stone_and_water_pool()` fixture exists, named and reusable by rounds 3/4.
   Met.
4. Step-timing property (irregular/sub-dt durations across calls) pinned at
   the `Grid`-stepping call site. Met.
5. Identity-transform no-op pinned after each of several individual steps.
   Met.

**Timing roll-up (Round 2):** single pass, ~15 min including all reading
(per the pass's own report, well inside the 30-minute budget) plus the
orchestrator's own ~5 min independent verification/fmt cleanup. No
overtime, no exit ramp.

**Gaps/flags carried forward (unchanged from the pass's own report, plus
one closed):**
- `step_once`'s per-cell loop is the seam M1.2+ replaces with real physics
  — named for that round.
- `Scenario.materials` is owned, not shared — a future round's call if
  many scenarios need to share one table.
- Out-of-bounds panic contract (round 1's flag) still open.
- density/heat_capacity unit-mixing (round 1's flag) still open, still
  unused by any numeric step logic so still non-blocking.
- Whole-crate `cargo fmt` adoption: now explicitly flagged for the
  milestone-scope Refactor pass to decide, rather than left as an
  ambient, unowned observation.
- The stray untracked `test/` directory: harmless, outside git, not
  pursued further (a Bash removal attempt was blocked by this session's
  own sandboxing; not worth escalating for a build artifact nothing
  references).

Round 2 closed. Proceeding to re-plan round 3 (headless measurement) per
`cycle-milestone` §2.
