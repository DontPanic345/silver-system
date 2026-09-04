# M0.4 closeout — Mathematical foundations

**Closed out:** 2026-09-05T09:04+12:00

## Targets — met or not

From `cycle-log/tranche-0/m0.4/plan.md` §3 (sharpened from `PLAN.md`):

1. **Grid and vector primitives exist and are unit-tested.** Met. `src/math.rs`
   ships `Vec2` (add/sub/scale/dot) and `GridIndex`, both with their own test
   coverage, confirmed by Round 1/2's Green+Refactor and re-confirmed cold by
   the milestone-scope Refactor.
2. **A test pins the coordinate convention, and a doc comment states it in
   words.** Met. `Vec2` documents math-y-up; `src/lib.rs` documents the
   canvas's y-down; the two doc comments cross-reference each other and state
   the mapping, per the milestone-scope Refactor's independent read
   (`cycle-log/tranche-0/m0.4/refactor-milestone.md`) — not just test-pinned.
3. **The numeric type decision (`f32` vs `f64`) is stated with written
   reasoning.** Met. `f32` chosen; the rationale lives on `Scalar`'s own type
   definition in `src/math.rs`, not buried in a log, confirmed by the same
   cold read.
4. **A fixed-timestep harness exists, is unit-tested, and is actually called
   by running code.** Met. `src/timestep.rs`'s `FixedTimestep` (Round 3);
   Round 4 retrofitted `src/lib.rs`'s `tick_and_draw` to drive `TICK` through
   `advance_tick`/`FixedTimestep::advance` instead of a bare `+1`. Round 4's
   Refactor grepped the whole crate and confirmed no dead parallel mechanism
   remains.
5. **Post-retrofit, wasm build / e2e test / live Pages deploy all still
   work, checked directly.** Met. Round 4's Green rebuilt wasm and reran the
   local Playwright e2e test; Round 4's Refactor independently rebuilt and
   reran both; the live deploy was checked with a headless Playwright run
   against `https://dontpanic345.github.io/silver-system/` itself (not just
   CI status), sampling three real ticks and confirming the colours match.

All five milestone targets met, each independently re-verified by a cold
Refactor pass rather than taken on Green's word alone.

## Rounds run, timing roll-up

- **Round 1** (numeric type + `Vec2`): Red ~20 min, Green ~3 min, Refactor
  ~2 min. Advanced.
- **Round 2** (`GridIndex`, cell-center convention, index↔position
  conversion): Red ~12 min, Green ~1 min, Refactor ~1 min. Advanced.
- **Round 3** (`FixedTimestep` accumulator harness): Red ~17 min, Green
  ~5 min, Refactor ~15 min wall (adversarial spiral-of-death/large-`dt`
  checks). Advanced.
- **Round 4** (retrofit M0.1's hello-world onto the harness): Red 4 min,
  Green 4 min, Refactor 2 min — **interrupted mid-milestone by this
  session's own rate limit between Green and Refactor** (a multi-hour gap
  in wall-clock time is visible in the round log's timestamps; the work
  itself was untouched and resumed cleanly once the limit reset). Advanced.
- **Milestone-scope Refactor** (whole crate, ~12 min): found and fixed one
  cross-round clippy lint no single round's narrow scope had reached
  (`tick % 2 == 0` → `tick.is_multiple_of(2)`, commit `2aedabc`); confirmed
  no correctness issues; made the reasoned call to carry forward, not close,
  the e2e test's one known coverage gap (see below). Verdict: advance —
  milestone genuinely done.
- **Milestone total:** 4 rounds + 1 milestone-scope pass, all comfortably
  inside budget on task-clock time (the Round 4 rate-limit gap was wall-clock
  only, not phase-budget time).

## What was learned that changes the plan going forward

- **Rounds 1–3 of this milestone ran with substantially overlapping
  timestamps** (Round 1 Red 03:15–03:35, Round 2 Red 03:22–03:34, Round 3
  Red 03:14–03:31) — the cold M0.4 orchestrator dispatched them concurrently
  rather than sequentially, despite its own plan (`m0.4/plan.md` §4)
  explicitly noting "sequenced per CLAUDE.md's 'do one thing at a time'."
  Nothing broke — the three rounds touched disjoint files (`math.rs`'s
  `Vec2`, `math.rs`'s `GridIndex`, `timestep.rs`) so no merge conflict or
  cross-round confusion resulted — but it's a real deviation from a stated
  project principle, not just this cycle's convention, and is worth naming
  plainly rather than quietly passing over because the outcome was fine.
  Flagged below as a candidate fix.
- **The rate-limit interruption between Round 4's Green and Refactor caused
  no rework.** The round log, the plan file, and the committed diff were
  enough for a fresh cold agent to resume exactly at the Refactor phase
  with no re-derivation — good evidence the cold-agent-per-milestone /
  file-as-source-of-truth pattern (adopted after M0.2/M0.3) survives a
  mid-milestone failure cleanly, which a forked, context-holding orchestrator
  would not have.
- **A real, bounded verification gap was found and consciously not closed**:
  the Playwright e2e test can't distinguish genuine `FixedTimestep`
  accounting from a disguised per-call `+1` bypass at the wasm-bindgen
  export boundary specifically (the native unit tests already catch this
  everywhere else). The milestone-scope Refactor reasoned this through
  explicitly rather than either ignoring it or over-building
  `wasm-bindgen-test` infrastructure disproportionate to the risk — carried
  forward as an open, named gap rather than resolved.

## Open gaps and flags carried forward

- The wasm-bindgen export-boundary verification gap above — closing it
  would mean standing up `wasm-bindgen-test`; not required now, worth
  revisiting if a future milestone's retrofit touches that boundary again.
- `wasm-bindgen-cli`'s version is still hand-pinned to `Cargo.lock`, not
  automated to track it (carried from M0.2, unrelated to this milestone,
  repeating so the tranche closeout doesn't have to dig for it).
- Staggered/face-value grid support is deferred to M1.4 by design, per
  `m0.4/plan.md` §7 — not a gap, a stated non-goal for this milestone.

## What the cycle itself got wrong — candidate fixes to cycle-* skills

- **A cold milestone-level agent dispatching its own rounds concurrently,
  against its own plan's stated sequencing, is a real slip** — not
  catastrophic here because the rounds happened to touch disjoint files,
  but that's luck, not design. `cycle-milestone`/`cycle-round` should say
  explicitly: rounds run one at a time, wait for a round's phases and
  close-out before dispatching the next round's Red, even when nothing
  technically stops firing them concurrently. Not fixed in this session;
  flagging for the tranche-scope refactor or a future skill-maintenance
  pass.
- The fork→cold-agent switch (recorded in `cycle-contract` §3a, commit
  `b921194`) held up under an actual mid-milestone failure, not just in
  theory — worth stating plainly as validated, not just theorized.

## PLAN.md

M0.4 executed as scoped; no target was cut, reworded, or found unattainable.
No change needed to `PLAN.md`'s M0.4 section itself. Tranche-0 closeout
(next) is where `PLAN.md` gets marked.
