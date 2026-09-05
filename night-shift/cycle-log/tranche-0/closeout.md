# Tranche 0 closeout — Mathematics and tooling foundations

**Closed out:** 2026-09-05T09:11+12:00

## Targets — met or not

From `cycle-log/tranche-0/plan.md` §3:

1. **Something reachable at a public GitHub Pages URL, built by CI.** Met.
   `https://dontpanic345.github.io/silver-system/` — live, built by
   `.github/workflows/deploy-pages.yml` on every push (M0.2), independently
   re-confirmed twice more since (M0.4 round 4, live-URL Playwright check;
   tranche-scope refactor's own CI-gate fix keeps it honest going forward by
   failing the deploy if `cargo test` fails).
2. **If (1) can't work, a documented working alternative exists.** Met, and
   moot: (1) succeeded, so the native binary + Playwright screenshot-capture
   fallback (M0.3) is built, proven end-to-end, and explicitly documented as
   the non-maintained path — not run in parallel with the live one, per the
   tranche's own "don't maintain both" instruction.
3. **Grid/vector primitives exist, unit-tested, exercised by real code.**
   Partially met, honestly. `Scalar` (`f32`) and `FixedTimestep` are both
   unit-tested *and* driven by running code (M0.1's retrofitted hello-world,
   M0.4 round 4). `Vec2`/`GridIndex` are unit-tested but not yet wired into
   anything running — the tranche-scope refactor considered force-wiring
   them into the static rectangle and rejected it, correctly, as repeating
   the exact "paper exercise" mistake the tranche's own reach notes warned
   against. Named plainly in `src/math.rs`'s doc comments as an open gap for
   tranche 1, not silently left implicit.
4. **The full loop is reproducible from a clean checkout, commands
   recorded.** Met. `README.md` documents build/serve/test; `rust-toolchain.toml`
   (added by the tranche-scope refactor) now pins the exact toolchain so a
   clean checkout can't silently drift from what was proven here.

3 of 4 targets fully met; the 4th is honestly partial with the gap named,
not glossed over, and a clear owner (tranche 1) for closing it.

## Milestones run, timing roll-up

- **M0.1 — Toolchain proving ground.** 2 rounds, ~21 min phase time. Canvas
  2D via wasm-bindgen/web-sys proven first try; found and fixed a real
  verification-harness gap (2-sample vs 3-sample state-change detection).
- **M0.2 — Deploy to GitHub Pages.** Run as a single forked continuous pass
  (phase isolation not yet available to a fork) — 358.8s, 132,566 tokens, 74
  tool calls. All 4 targets met on the first attempt; the flagged
  no-`gh`/no-token risk never materialized (Pages was already enabled).
- **M0.3 — The fallback.** Same continuous-pass shape — 214.1s, 97,617
  tokens, 31 tool calls. Both targets met; native+PNG fallback proven but
  deliberately not the maintained path.
- **M0.4 — Mathematical foundations.** 4 rounds + 1 milestone-scope
  Refactor. Round phase-time totals: R1 ~25 min, R2 ~14 min, R3 ~37 min
  (heaviest — the fixed-timestep harness's adversarial dt-spike/spiral-of-
  death checks), R4 ~10 min phase time (interrupted mid-round by this
  session's own rate limit between Green and Refactor — a multi-hour
  wall-clock gap with zero rework needed on resume), milestone-scope
  Refactor ~12 min. All 5 targets met, each independently re-verified cold
  rather than taken on Green's word.
- **Tranche-scope Refactor** (this session, whole-repo sweep): ~5.5 min.
  Added the missing `rust-toolchain.toml` pin, gated CI on `cargo test`
  passing (previously deployed regardless), fixed stale "two samples" doc
  drift in README/e2e test comments left behind by M0.1 round 2's 3-sample
  fix, named the `Vec2`/`GridIndex`-unused gap explicitly, and cleaned up
  clippy noise with rationale. Verdict: tranche genuinely done.
- **Tranche total:** 4 milestones (2 forked, 2 cold-agent-orchestrated with
  full round structure) + 1 tranche-scope pass. No exit ramps taken anywhere
  in the tranche; every round and milestone advanced on its own or the
  orchestrator's first independent check.

## What was learned that changes the plan going forward

- **Forking for milestone/tranche self-dispatch was the wrong tool**,
  discovered concretely twice (M0.2, M0.3): a forked agent can't spawn its
  own subagents, so it can't give Red/Green/Refactor true isolation — it
  collapses a milestone into one continuous pass, quietly discarding the
  cold-Refactor-catches-Green's-blind-spot property the whole phase design
  exists for. Fixed going forward via `cycle-contract` §3a (commit
  `b921194`): dispatch milestone/tranche self-isolation as a fresh cold
  `Agent` call instead. Validated under real conditions in M0.4, which
  survived a mid-round rate-limit failure with zero rework by resuming from
  the committed round log — a forked, context-holding orchestrator would not
  have offered that recovery property.
- **A cold milestone orchestrator dispatched three rounds concurrently**
  (M0.4 rounds 1-3), against both CLAUDE.md's "do one thing at a time" and
  its own round plan's stated sequencing. Nothing broke this time — the
  rounds happened to touch disjoint files — but it was unenforced, not
  designed-for. Fixed going forward: `cycle-milestone` and `cycle-round` now
  state explicitly that rounds are dispatched strictly one at a time,
  waiting for a round's full close-out before the next round's Red starts
  (tranche-scope refactor, commit `99066d2`).
- **The "sample at least 3 points in time" lesson from M0.1 needs active
  maintenance, not just a one-time fix** — the tranche-scope refactor found
  the README and e2e test's own header comment had drifted back to
  describing "two points in time" after round 2's fix moved the actual code
  to three samples. A fix landing in code without its explaining prose
  keeping pace is a real, recurring failure mode worth naming for future
  tranches, not just this one instance.

## Open gaps and flags carried forward

- **`Vec2`/`GridIndex` are tested but not exercised by running code** —
  named explicitly in `src/math.rs`; the honest close is tranche 1's first
  real grid, not a forced wiring into M0.1's static rectangle.
- **`wasm-bindgen-cli`'s version is hand-pinned to `Cargo.lock`**, not
  automated to track it (carried since M0.2, never became a problem, still
  unautomated).
- **A narrow wasm-bindgen export-boundary verification gap** (M0.4): the
  Playwright e2e test can't distinguish genuine `FixedTimestep` accounting
  from a disguised per-call bypass specifically at that boundary; native
  unit tests catch it everywhere else. Consciously deferred, not resolved —
  standing up `wasm-bindgen-test` would be new scope disproportionate to the
  risk today.

## What the cycle itself got wrong — candidate fixes to cycle-* skills

- **Fork vs. cold-agent for self-dispatch** — real defect, found and fixed
  mid-tranche (`cycle-contract` §3a, commit `b921194`). Worth flagging to
  whoever maintains the cycle-* skills as validated under an actual failure,
  not just theorized.
- **Unenforced sequential round dispatch** — real defect, found and fixed
  this pass (`cycle-milestone`/`cycle-round`, commit `99066d2`).
- **Doc/comment drift after a code fix** — no skill mechanism currently
  catches this class of gap; a milestone- or tranche-scope Refactor pass is
  where it gets caught today (as it did here), which worked, but there's no
  standing check between those wide sweeps. Not fixing this now — flagging
  as a pattern to watch, not a concrete skill edit, since one clean catch
  isn't yet evidence of a systemic gap.

## PLAN.md

Marking tranche 0 closed in `PLAN.md` next, in the same commit as this file.
No milestone or target required rewording — the tranche executed
essentially as scoped, plus this session's own reach additions (recorded
throughout `cycle-log/tranche-0/`), which don't require rewriting `PLAN.md`
itself.

## Stopping point

Per explicit instruction, **not** chaining forward into tranche 1 (Physics)
automatically. Tranche 0 is handed back for review before tranche 1 starts.
