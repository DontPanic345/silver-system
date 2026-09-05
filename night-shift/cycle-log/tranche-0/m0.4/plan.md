# M0.4 plan — Mathematical foundations

**Planned:** 2026-09-05T03:14:06+12:00

## 0. What's been read

`PLAN.md` (tranche 0 and M0.4 sections), `CLAUDE.md`, the tranche plan
(`cycle-log/tranche-0/plan.md`), and all three prior closeouts (M0.1, M0.2,
M0.3). Also read `src/lib.rs`, `www/index.html`, `Cargo.toml` and `README.md`
as they stand today, to know exactly what M0.4 is retrofitting.

Relevant carried-forward flags:

- Tranche plan §2 and M0.1's closeout both flag the same thing twice: M0.1's
  hello-world drives its animation with an inline, admittedly-throwaway tick
  counter (`TICK: Cell<u32>` in `src/lib.rs`, driven by a plain JS
  `setInterval` in `www/index.html`) and explicitly defers the real
  fixed-timestep harness to this milestone. One of this milestone's rounds
  must retrofit it — that's the honest way M0.4's target ("exercised by the
  M0.1 hello-world so it isn't a paper exercise") gets met.
- M0.1's closeout also flags: sample at least three points in time for any
  animated/long-run verification, never two — a two-sample check can't
  distinguish "advancing" from "changed once, then stuck". Applies directly
  to whatever Playwright/unit test verifies the retrofitted harness.
- The tranche plan flags the coordinate-convention doc comment as a named
  reach item: "the test pins it, the doc comment stops someone re-deriving it
  by reading test names." `src/lib.rs` already has one such comment for the
  canvas (origin top-left, y down) — M0.4's vector/grid primitives need their
  own, stated explicitly, and the two need to agree with each other or state
  the mapping between them.
- No open risk specific to M0.4 was flagged by M0.1/M0.2/M0.3 beyond the
  retrofit itself and the wasm-bindgen-cli version pin (unrelated to this
  milestone).

## 1. Intent, sharpened

`PLAN.md`'s intent: the small, boring substrate every later tranche reaches
for — vector/grid primitives, a numeric type decision, a fixed-timestep
harness — got right once, here, rather than reinvented per-tranche. The
tranche plan adds: the coordinate convention must be a written doc comment,
not just pinned by a test, because that was the JS attempt's single most
expensive recurring bug (re-derived under pressure three times).

**How this serves the north star:** every later tranche's physics, chemistry
and biology reads or writes a grid cell and steps time. If the coordinate
convention, the numeric representation, or the timestep discipline is wrong
or informal here, every tranche after this one either silently inherits the
bug or repeats the JS attempt's expensive re-derivation. This milestone is
pure infrastructure with no visible payoff of its own — its service to the
north star is entirely in preventing the tranche-1-through-4 work from being
wrong or badly re-invented underneath the terrarium nobody would otherwise
see the cost of.

## 2. Reach

- **Numeric type decision needs a real written rationale**, not a default.
  Candidates: `f32` (cheaper, GPU-friendly, the likely eventual target per
  CLAUDE.md's "GPU execution is the intended direction"), `f64` (safer
  headroom for long accumulations — relevant given targets like "conserved
  over 10,000/100,000 steps"), fixed-point (deterministic but more code, and
  CLAUDE.md says determinism is architecture-contingent, not a standing
  requirement). Decide `f32` vs `f64` for the world-space/physics scalar type
  now, with the reasoning written down, rather than each future tranche
  picking its own and disagreeing.
- **Grid indexing needs an explicit cell-center vs. corner decision**, not
  just "some grid type exists". Physics quantities (density, temperature,
  pressure) are conventionally cell-centered; staggered solvers (M1.4's
  pressure milestone, flagged in `PLAN.md` as "staggered or compact, chosen
  on evidence") may want face/corner values later. M0.4 should pick and
  document the *default* convention for the primitives it ships now (cell-
  center, integer `(i, j)` index → cell-center world position), and say
  plainly that a staggered variant is a later, additive decision for M1.4 —
  not something M0.4 needs to pre-solve.
- **The fixed-timestep harness should be decoupled from rendering**, per
  M0.1's own Refactor flag ("expected to throw away the `setInterval`/
  `tick_and_draw` coupling in favor of an accumulator... decoupled from
  rendering"). This is exactly the retrofit round's job.
- A `Vec2` type wants the basic arithmetic later tranches will reach for
  immediately (add, sub, scale, dot) — not because a future round can't add
  it, but because a `Vec2` with no arithmetic isn't really a primitive yet
  and the tranche's own target language calls these "grid/vector primitives
  physics needs" (plural capability, not just a named struct).
- The retrofit is explicitly in scope for this milestone (see 0, above) and
  must include re-verifying the wasm build and the GitHub Pages deploy still
  work afterward — the orchestrator's own instruction, and consistent with
  M0.2/M0.3's practice of verifying the live artifact rather than trusting
  CI green.

## 3. Milestone targets

Restating `PLAN.md`'s M0.4 targets, sharpened:

1. Grid and vector primitives exist and are unit-tested.
2. A test pins the coordinate convention explicitly (which axis is up,
   cell-center vs. corner indexing) — **and** a doc comment states the same
   convention in words, so a reader doesn't have to reverse-engineer it from
   test names.
3. The numeric type decision (`f32` vs `f64`, or fixed-point) is stated with
   written reasoning, not defaulted into.
4. A fixed-timestep stepping harness exists, is unit-tested, and is actually
   called by running code — specifically, M0.1's hello-world
   (`src/lib.rs`/`www/index.html`) is retrofitted to drive its animation
   through the harness instead of the ad-hoc tick counter.
5. After the retrofit, the wasm build still succeeds, the Playwright e2e test
   still passes, and the live GitHub Pages deploy
   (`https://dontpanic345.github.io/silver-system/`) still loads and draws
   correctly following a push — checked directly, not assumed from workflow
   status.

## 4. Rounds (starting position, not a contract)

1. **Round 1 — Numeric type and `Vec2`.** Decide and document (doc comment,
   module-level) the scalar numeric type for world-space/physics quantities;
   introduce a `Vec2` with basic arithmetic and an explicit coordinate-
   convention doc comment (which axis is up — math convention, y-up,
   distinct from the canvas's y-down, with the mapping between them stated);
   a test pins the convention (e.g. "up" moves a point in the direction the
   convention says, not the canvas's).
2. **Round 2 — Grid index type.** A `GridIndex`/cell-coordinate type
   (integer `(i, j)`), the cell-center-vs-corner decision made and
   documented, and a conversion function between a grid index and its
   `Vec2` world-space position under the pinned convention. Tests pin both
   the indexing convention and the conversion's correctness (e.g. index
   (0,0)'s center position, index spacing).
3. **Round 3 — Fixed-timestep harness.** An accumulator-style stepping
   harness (fixed `dt`, decoupled from wall-clock/render rate), unit-tested
   directly (no browser involved) — e.g. feeding it varying frame durations
   and asserting it produces the right number of fixed steps.
4. **Round 4 — Retrofit M0.1's hello-world.** Replace `src/lib.rs`'s ad-hoc
   `TICK` counter and `www/index.html`'s bare `setInterval`-driven
   `tick_and_draw` with the real harness from round 3, keeping the same
   externally-observable behaviour (rectangle alternates colour on a fixed
   period) so the existing Playwright e2e test keeps working as the
   regression check. Round includes: rebuild wasm, re-run the e2e test,
   push, and verify the live Pages URL still loads and animates afterward.

Round 4 is a hard dependency on round 3 (needs the harness to retrofit onto)
and soft dependency on rounds 1–2 only in spirit (the harness doesn't need
`Vec2`/`GridIndex` to exist, but by round 4 they should already be in the
same crate so nothing about round 4 forces reordering). Rounds 1 and 2 could
in principle run in parallel; sequenced per CLAUDE.md's "do one thing at a
time".

## 5. Refactor scope/focus per round (starting position)

- Round 1: round scope, focus on convention correctness and whether the
  doc comment and the test actually agree (the exact bug class this exists
  to prevent).
- Round 2: round scope, same focus, plus whether the grid/vector types
  compose cleanly (does converting an index to a position and back round-
  trip correctly).
- Round 3: round scope, focus on the harness's determinism/robustness to
  variable frame durations (spiral-of-death guard, large `dt` spikes) since
  every later tranche depends on this loop being right.
- Round 4: wider scope (the whole crate, since this round touches the
  previously-shipped M0.1 surface), focus on regression — does the retrofit
  actually preserve the animated behaviour the existing e2e test checks, and
  is the ad-hoc tick counter fully gone (not left as dead code).

## 6. Push on vs. patch back

No prior M0.4 rounds exist — pushing on. Starting with Round 1.

## 7. Deferred / flagged forward

- Staggered-grid (face/corner value) support is explicitly deferred to
  M1.4, per its own `PLAN.md` language ("staggered or compact, chosen on
  evidence"). M0.4 ships the cell-center default only, documented as a
  default rather than the only possible layout.
- Fixed-point numeric types are not being built now — `f32` vs `f64` is the
  live decision; fixed-point stays a documented non-choice unless a much
  later milestone's evidence calls for it (determinism is architecture-
  contingent per `CLAUDE.md`, not a standing requirement).
