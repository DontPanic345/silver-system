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
