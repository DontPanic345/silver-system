# M0.1 closeout — Toolchain proving ground

**Closed out:** 2026-09-05T03:03+12:00

## Targets — met or not

From `PLAN.md` (M0.1 targets):

1. **The wasm build succeeds.** Met. `cargo build --target wasm32-unknown-unknown`
   (via `scripts/build-wasm.sh`, which also runs `wasm-bindgen-cli`) succeeds
   from a genuinely clean checkout — verified twice independently: once by
   Round 2's Refactor (`cycle-log/tranche-0/m0.1/round-02.md`), once by this
   orchestrator just now (`git log` shows commit `62ba5cf`; build output
   confirms "Finished `release` profile").
2. **The artifact runs in a real browser and draws something [that visibly
   changes over time].** Met. Headless Chromium (Playwright) loads the built
   page and reads real `getImageData` pixel values at three points in time
   (tick 0/1/2), confirmed alternating and back — `tests/e2e/canvas_rectangle.test.mjs`,
   passing as of commit `62ba5cf`. The "visibly changes" half was this
   session's own reach addition (`cycle-log/tranche-0/plan.md` §2), not
   literally in PLAN.md, added because M0.4's fixed-timestep target needs a
   real animated hello-world to exercise, not a static frame.
3. **The exact commands are recorded so CI can repeat them exactly.** Met.
   `README.md` documents the build/serve/test commands; `scripts/build-wasm.sh`
   is the single source of truth CI (M0.2) will call directly rather than
   reconstructing from prose.

Tranche-level target #4 (this session's own addition — "the full loop is
reproducible from a clean checkout with a stated, short list of commands") is
also met by the same evidence above.

## Rounds run, timing roll-up

- **Round 1** — rendering approach decided (canvas 2D / wasm-bindgen / web-sys),
  static rectangle proven end-to-end. Red ~7 min, Green ~1 min, Refactor ~4 min
  (including two deliberate fault-injection attacks on the headless harness).
  Round total well under the 30-minute budget. Advanced.
- **Round 2** — tick-driven animation, constant-duplication fix, README
  commands. Red ~4 min, Green ~1 min, Refactor ~4 min (including a mutation
  that found a real gap — the harness couldn't tell an advancing counter from
  one frozen at 1 — fixed forward in the same phase). Round total well under
  budget. Advanced; this Refactor pass doubled as M0.1's milestone-scope pass.
- **Milestone total:** 2 rounds, roughly 21 minutes of phase time across all
  six phases, well inside budget with room to spare. No exit ramps taken, no
  round needed to cycle.

## What was learned that changes the plan going forward

- **The dev-env memory claiming "no rustc/cargo" was stale.** The toolchain
  installs cleanly via rustup in under a minute; the only real snag was
  `~/.cargo/bin` not being on `PATH` for a fresh non-interactive shell (fixed
  by symlinking into `~/.local/bin`, which already is). Resolved before
  tranche planning even started this session — `dev-env-gaps` memory updated.
- **Canvas 2D via wasm-bindgen/web-sys worked on the first real attempt** —
  no WebGL/WebGPU comparison was forced, since nothing failed. This is a
  genuinely light rendering path; M4's real renderer will need much more
  (overlays, camera, zoom) but the base pipe is proven cheap.
  M0.4 was flagged (by Round 2's Refactor) as likely to throw away the
  `setInterval`/`tick_and_draw` coupling in favor of an accumulator or
  `requestAnimationFrame`-driven fixed-timestep harness with steps decoupled
  from rendering — expected and fine; M0.1's job was proving the pipe, not
  building the final harness.
- **A real correctness class was found and fixed inside this milestone**: a
  "sample twice, assert different" verification pattern can't distinguish a
  genuinely advancing counter from one that's stuck after its first increment.
  Every later milestone's animated/long-run scenario verification (U-pipe
  levelling, resting-pile stability, etc.) should sample at least three points
  or otherwise guard against this, not just two. Worth stating explicitly to
  M1's planner when it designs the scenario harness (M1.1) — this is exactly
  the kind of headless-verification pitfall that's cheap to avoid once named
  and expensive to discover mid-tranche-1.

## Open gaps and flags carried forward

- M0.4 will need to retrofit `www/index.html` / `src/lib.rs`'s hello-world to
  call the real shared fixed-timestep harness instead of the inline tick
  counter built here — recorded in `cycle-log/tranche-0/plan.md` and
  `cycle-log/tranche-0/m0.1/plan.md`; repeating it here so M0.4's planner
  doesn't have to dig for it.
- M0.2 (GitHub Pages deploy) is the next milestone and inherits the open risk
  already flagged in the tranche plan: no `gh` CLI / `GITHUB_TOKEN` in this
  environment, so Pages enablement can only go through committed workflow
  files and whatever `actions/configure-pages` can self-enable — untested
  until M0.2 actually runs.
- Two untracked planner files (`cycle-log/tranche-0/plan.md`,
  `cycle-log/tranche-0/m0.1/plan.md`) have sat untracked through both rounds,
  correctly left alone by every phase since they predate the round's file
  list. Committing them is this orchestrator's job, done as part of this
  closeout commit.

## What the cycle itself got wrong — candidate fixes to cycle-* skills

- Nothing structurally wrong surfaced in M0.1. The pattern of adapting
  cycle-red/green/refactor's "failing test + skeleton" idiom to a
  tooling-proving milestone (skeleton = Cargo project scaffold + host page,
  "test" = a headless Playwright script reading real pixel data) worked
  cleanly both rounds — worth noting as a validated pattern for any future
  tooling-flavored milestone, not a defect to fix.
- One process observation, not a skill defect: this orchestrator was
  reloading the full `cycle-plan`/`cycle-round` skill text via the `Skill`
  tool on every re-invocation within the same session, which is unnecessary
  once the instructions are already in context — corrected mid-tranche after
  the user raised context growth as a concern. Worth a line in
  `cycle-contract` or `cycle-tranche` telling the orchestrator explicitly:
  don't re-invoke a skill already loaded this session, follow it from
  memory. Flagging here rather than editing the skill unilaterally mid-run.

## PLAN.md

No change needed to PLAN.md's M0.1 section — the milestone was executed
essentially as scoped, plus this session's own reach additions (documented in
`cycle-log/tranche-0/plan.md` and `m0.1/plan.md`), which don't require rewriting
PLAN.md itself since they were framed as this-tranche-planning decisions, not
corrections to the source document.
