# M0.1 plan — Toolchain proving ground

**Planned:** 2026-09-05T02:33:07+12:00 (immediately following tranche-0 plan, same
session)

## 0. What's been read

No prior rounds for this milestone. Read `cycle-log/tranche-0/plan.md` (this
session's own tranche plan) and PLAN.md's M0.1 section.

Groundwork already done during tranche planning, ahead of Round 1, so Round 1
doesn't have to spend its 30 minutes rediscovering it:
- `rustup target add wasm32-unknown-unknown` succeeds on this machine.
- `crates.io`'s sparse index (`index.crates.io`) is reachable — `cargo` can
  fetch dependencies. (A plain `curl` to `crates.io`'s HTML front-end 403s,
  presumably bot-blocked; irrelevant to `cargo`, which never hits that path.)
- No `wasm-bindgen-cli` installed yet, no `wasm-pack`/`trunk` present — Round 1
  picks one.

## 1. Intent, sharpened

Confirm the whole chain from `cargo build` to a browser tab, on this actual
machine, before trusting it with real work. This directly serves the tranche's
purpose: it's the first empirical answer to "can Rust reach a screen here",
which both north-star statements depend on.

## 2. Reach

- PLAN.md's "a coloured rectangle is enough" is the floor, not the target.
  Make it a rectangle that visibly changes once per fixed tick (colour or
  position), so the artifact is proof of a *running program*, not a single
  paint call — a static image would pass a weaker version of this milestone's
  target but wouldn't prove the loop.
- **Ordering dependency, flagged now rather than discovered in Round 1:** the
  tranche-level reach note said M0.1's hello-world should call M0.4's
  fixed-timestep harness so M0.4's target is honestly exercised. But M0.4 is
  sequenced *after* M0.1–M0.3 in the tranche plan. Resolution: M0.1 gets its
  own small inline tick counter (not M0.4's shared harness — that doesn't
  exist yet) to drive the visible change. When M0.4 runs, one of its rounds
  retrofits the hello-world to call the *real* shared harness instead of the
  inline counter, which is what actually earns M0.4's target. Recording this
  here so M0.4's planner doesn't miss that retrofit step.
- Record exact commands as they're discovered (build, serve, and — once
  wasm-bindgen-cli or trunk is chosen — the bindgen step) into the README as
  they're proven, not reconstructed from memory afterward.

## 3. Targets (from PLAN.md, milestone-scope)

- The wasm build succeeds (`cargo build --target wasm32-unknown-unknown`, or
  equivalent through the chosen tool).
- The artifact runs in a real browser and draws something that visibly changes
  over time.
- The exact commands are recorded (README) so CI (M0.2) can repeat them
  exactly.

Measurement approach: headless verification via Playwright (already available,
matches the project's stated visual-verification preference) — read pixel
data from the canvas at two points in time and assert they differ, rather than
a screenshot handed to a human. A PNG goes to the user via SendUserFile only if
useful alongside the numbers, never as the only proof.

## 4. Rounds — starting position

- **Round 1.** Choose the rendering approach (canvas 2D via `wasm-bindgen`
  + `web-sys` vs. a WebGL/WebGPU crate), on the standard PLAN.md sets: what
  actually builds and runs here, not paper suitability. Get a static coloured
  rectangle on a canvas, served locally, verified headless. This is the
  highest-uncertainty round in the milestone — if canvas 2D turns out to have
  a snag, better to find it here before Round 2 builds the tick loop on top.
  *Likely direction going in (not frozen): `wasm-bindgen`/`web-sys` canvas 2D
  is the simplest possible path and needs no GPU context negotiation, so it's
  the natural first thing to try — but Round 1 owns the actual decision.*
- **Round 2.** Add the inline tick counter (see Reach, above) so the rectangle
  changes once per fixed step; verify headless that two samples in time
  differ; record the exact build/serve commands in the README.

Two rounds is a starting position — if Round 1 finds the chosen approach
doesn't hold up, expect to insert a groundwork round rather than force Round 2
onto a shaky base.

## 5. Round 1 — decisions

**Push on vs. patch back:** nothing to patch back; this is the milestone's
first round. Push on.

**Goals.**
1. Pick and justify a rendering approach, based on what builds and runs here.
2. A wasm32 build of a minimal crate succeeds.
3. That build, served locally, draws a coloured rectangle to a canvas in a
   real browser (verified headless via Playwright — read canvas pixel data,
   don't eyeball a screenshot).
4. The exact commands used (build, bindgen if applicable, serve) are written
   down in the round log, ready for the README once Round 2 finalises them.

**Refactor scope/focus:** round scope (this is the first round of a new
milestone — nothing to fold into yet). Focus: is the chosen approach actually
sound to build Round 2 on, or did Green paper over a snag that will bite next
round? Adversarial angle: check that the headless verification is actually
reading real canvas output and not a stale/cached page — the failure mode
that would quietly make every later round's "verified headless" claim
worthless.

**Adversarial focus:** the headless-verification harness itself, since every
later round in this whole tranche will depend on it being trustworthy.
