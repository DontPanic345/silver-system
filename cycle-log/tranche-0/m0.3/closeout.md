# M0.3 closeout — The fallback

**Closed out:** 2026-09-05T03:34+12:00

**Process note:** run by a single forked agent, one continuous pass rather
than isolated Red/Green/Refactor phases via `cycle-round` — the same
deviation M0.2's fork flagged (a forked agent can't itself spawn
subagents). Repeating it here rather than re-discovering it as new; see
M0.2's closeout for the same note.

## Targets — met or not

From `PLAN.md` (M0.3):

1. **A documented, exercised path exists for the user to see output even in
   the worst case.** Met. `src/bin/native_viewer.rs` renders ticks 0-2 to
   real PNG files via `viewer::render_frame` (shares geometry/colour
   constants with the wasm path — one source of truth, not a re-derivation).
   Exercised, not just compiled: `tests/native_fallback.rs` runs the actual
   compiled binary as a subprocess, decodes the PNGs it writes, and asserts
   the rectangle's real pixel bytes match `color_for_tick`'s alternation at
   three sample ticks — `cargo test` output confirms `1 passed; 0 failed`
   (commit `307e5d5`).
   **Deviation from literal wording, stated and reasoned in
   `cycle-log/tranche-0/m0.3/plan.md` §2:** PLAN.md said "Playwright
   screenshot capture" — Playwright drives browsers, and a native binary has
   no browser/window to screenshot. Substituted the equivalent-strength
   check: read real pixel bytes back from the file the binary wrote, same
   "don't take output on faith" discipline as the Playwright canvas check.
2. **The chosen path is stated plainly, not left ambiguous.** Met.
   `README.md` (commit `5ec0a12`) says outright that GitHub Pages/wasm is the
   live, maintained path; the native fallback is documented and proven but
   not watched day to day — per the tranche's "don't maintain both" rule.

## Adversarial check

Injected a fault directly into `color_for_tick` (forced it to always return
the even-tick colour, freezing the alternation) and re-ran
`tests/native_fallback.rs` — it failed correctly, catching the frozen colour
at tick 1 (`left: (200, 60, 60) right: (60, 120, 200)`). Reverted; suite
green again. Confirms the fallback's check can actually fail, not just pass
by construction — the same class of check M0.1's round 2 Refactor ran on the
wasm path.

## Rounds run, timing

One continuous pass, no round/phase split (see process note). Wall-clock
roughly 20 minutes: plan, implement (`render_frame`, native binary,
integration test, target-specific `image` dependency), fix the wasm build
(`build-wasm.sh` needed `--lib` once a native-only bin existed in the same
package — caught immediately by re-running the wasm build, not left for
someone else to discover), adversarial mutation, README update, two commits.

## What was learned

- **A native binary sharing a library crate with a wasm target needs
  `cargo build --lib` in the wasm build script**, not a bare
  `cargo build --target wasm32-unknown-unknown` — the latter tries to build
  every target in the package, including a native-only binary that can't
  compile for wasm32 even with a correctly target-gated dependency. Caught
  immediately by re-running `scripts/build-wasm.sh` after adding the binary,
  before it could reach CI. Worth remembering for M0.4 and beyond if any
  future crate in this repo grows more than one binary/library target.
- **Playwright's headless-verification pattern generalizes past browsers**:
  "run the real artifact, read real output bytes back, don't eyeball a
  picture" applies just as well to a PNG file on disk as to a canvas —
  the fallback's test does the file-reading equivalent of `getImageData`.
- Reused the M0.1 round 2 lesson directly rather than re-deriving it: three
  sample ticks, not two, for the same reason (a two-sample check can't tell
  "advancing" from "changed once, then stuck").

## Open gaps and flags carried forward

- None new. M0.2's carried-forward gap (wasm-bindgen-cli version hand-pinned,
  not automated to track `Cargo.lock`) is unaffected by this milestone and
  still open.
- Same process gap M0.2 flagged: `cycle-contract`/`cycle-tranche` should warn
  a self-forking orchestrator upfront that a fork cannot dispatch further
  subagents, so milestone-level forking and round-level phase isolation are
  mutually exclusive — this is now two milestones in a row hitting the same
  constraint, which is stronger evidence it belongs in the skill text, not
  just a per-milestone footnote.

## What the cycle itself got wrong

- Nothing new beyond the repeated process note above. The adversarial-mutation
  habit (inject a fault, confirm the test catches it, revert) established in
  M0.1 round 2 transferred cleanly to a completely different verification
  mechanism (file bytes instead of canvas pixels) — validates it as a general
  practice, not something specific to the browser path.

## PLAN.md

No change needed — M0.3 executed as scoped. Both tranche targets it serves
(PLAN.md's #2, and the tranche plan's own target #4 about a reproducible,
documented loop) are met.
