# Round 4 — Minimal renderer

**Milestone:** M1.1 — Substrate and harness (Tranche 1 — Physics)

**Shape:** single-pass. Reasoning: this round is additive to the existing,
already-proven wasm-bindgen + native-fallback pipeline (new exports, a new
grid-painting function reusing `render_frame`'s established pattern) rather
than a rewrite of it. It does touch `src/lib.rs` for the first time this
milestone, but the existing M0.1 rectangle path (`draw`, `paint_rect`,
`tick_and_draw`, `advance_tick`, `color_for_tick`) is not itself modified,
only added alongside — so the blast radius is contained to new surface, not
a change to a proven interface. No conservation/determinism target, no
prior exit ramp on this goal. Per `cycle-plan` §1c step 3, defaults to
single-pass — but flagged below with an explicit must-not-break condition
given what's already resting on the existing pipeline.

**Push on vs. patch back:** push on. Rounds 1-3 advanced cleanly; nothing
outstanding blocks a renderer round. Carried-forward flags (unit mismatch,
out-of-bounds contract, fmt drift, stray `test/` dir) are all irrelevant to
painting a grid's current material colours to a buffer.

**Must not break:** the existing M0.1 Playwright e2e test
(`tests/e2e/canvas_rectangle.test.mjs`) and native fallback test
(`tests/native_fallback.rs`) must both still pass unmodified after this
round — they prove the *existing* pipeline still works; this round adds a
second, parallel capability, it does not replace the first one. If keeping
both green turns out to conflict with this round's goals, stop and flag it
rather than editing those tests to make room.

## Goals

1. **A pure grid-to-pixels function**, mirroring `render_frame`'s existing
   shape (`src/lib.rs`): given a `&Grid` (or a built `Scenario`) and pixel
   dimensions, returns a flat RGB8 buffer with each grid cell's material
   colour filling its corresponding pixel region — no DOM, no wasm-bindgen
   dependency, callable from both a native binary and (via a thin wrapper)
   wasm. Use `Material::colour` (round 1) as the per-cell source colour.
2. **A wasm export** that paints a `Scenario`'s current grid state to a
   named canvas element, reusing the existing `web_sys::CanvasRenderingContext2d`
   pattern `paint_rect` already established (get canvas by id, get 2d
   context, draw). This is the "watchable" half of milestone target 2.
3. **A native-fallback path**: extend `src/bin/native_viewer.rs` (or add a
   sibling binary — your call) to write a PNG of a scenario's grid using
   goal 1's pure function, the same `image` crate + pattern already proven
   for the M0.1 rectangle.
4. **A headless empirical check that it actually rendered**, not a human
   looking at a picture: extend the existing Playwright pattern (read real
   canvas pixel data at specific coordinates, per
   `tests/e2e/canvas_rectangle.test.mjs`'s established approach) or the
   native PNG-decode pattern (per `tests/native_fallback.rs`) to assert
   specific pixels match specific materials' `colour` values for
   `stone_and_water_pool()` (round 2's fixture) or another scenario of your
   choosing. State clearly whether you used the wasm/Playwright path, the
   native/PNG path, or both, and why.
5. **One definition, two consumers, demonstrated concretely**: the same
   `Scenario` value that round 3's `run_headless` measures is the one this
   round's renderer paints — not two different scenario shapes that happen
   to look similar. Show this directly (e.g. a shared fixture used by both
   a measurement test and a rendering test).

## Intent

A minimal renderer good enough to *watch* a scenario. Debugging blind is a
tax on every round after this one — this is explicitly the "first milestone
a human can watch anything from," per the tranche-1 plan. Stays cheap and
minimal on purpose: flat top-down material colour, nothing fancier (no
overlays, camera, or interaction — tranche 4's job).

## Scope and focus

**Scope:** primarily `src/lib.rs` (new additive exports/functions only —
existing M0.1 functions untouched), possibly a new small module if that
keeps `lib.rs` from growing unwieldy (your call), `src/bin/native_viewer.rs`
(extend, don't replace its existing M0.1 behaviour), and a new or extended
e2e/native test. Does not touch `src/grid.rs`, `src/material.rs`,
`src/scenario.rs`, `src/measure.rs`'s existing logic (read-only use of their
public APIs is expected and fine). **Focus:** genuinely reusing the existing
proven pipeline pattern (pure buffer function + thin wasm wrapper + thin
native wrapper + headless pixel-reading test) rather than inventing a
parallel one, and the one-definition-two-consumers property being real, not
superficial.
