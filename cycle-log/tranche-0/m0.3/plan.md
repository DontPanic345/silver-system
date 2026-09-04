# M0.3 plan — The fallback

**Planned:** 2026-09-05T03:09+12:00 (forked from the orchestrator, run as one
continuous pass — see process note below)

## 0. What's been read

`PLAN.md`'s M0.3 section, `cycle-log/tranche-0/plan.md`, M0.1 and M0.2
closeouts. M0.2 succeeded — Pages is live and re-deploys cleanly. So M0.3's
job is not "rescue the project", it's "prove the fallback exists and works,
then say plainly it isn't the path in use."

## 1. Intent, sharpened

Build and exercise the native-binary fallback now, while the stakes are zero,
per PLAN.md's own reasoning — not because Pages failed (it didn't), but
because a later tranche is the wrong time to discover the fallback doesn't
actually work. State plainly, once done, that Pages remains the live path.

## 2. Deviation from the literal target wording

PLAN.md says "headless screenshot capture (Playwright...)". Playwright drives
a *browser*; a native binary has no browser and no window to screenshot, so a
literal Playwright screenshot doesn't apply here. The equivalent-strength
substitute, consistent with the project's headless-verification preference:
the native binary renders frames to real PNG files on disk, and a separate
headless check decodes those PNGs and reads real pixel bytes back — same
"read real output data, don't eyeball a picture" discipline as the Playwright
`getImageData` check, just applied to a file instead of a canvas. Documented
here rather than silently reinterpreting the target.

## 3. Targets (from PLAN.md, milestone-scope)

- A documented, exercised path exists for the user to see output even in the
  worst case (native binary produces real image output, proven by running it
  and reading the bytes back — not just compiling).
- The chosen path (Pages, since it works) is stated plainly in the README;
  the fallback is documented but not the one maintained/watched going
  forward.

## 4. Plan — single pass

No round split: this is a forked agent and cannot spawn the isolated
Red/Green/Refactor subagents `cycle-round` requires (a fork can't spawn
subagents). Same deviation M0.2 already flagged. One continuous pass:

1. Add a pure, cfg-independent `render_frame` function to `src/lib.rs`
   reusing the existing geometry/colour constants and `color_for_tick` — the
   native path must not duplicate the single source of truth the wasm path
   already established.
2. Add `src/bin/native_viewer.rs`: a native binary that renders ticks 0/1/2
   to real PNG files via the `image` crate, using `render_frame`. `image` is
   a target-specific dependency (`cfg(not(target_arch = "wasm32"))`) so it
   never affects the wasm build.
3. Add `tests/native_fallback.rs`: an integration test that runs the compiled
   binary (or calls its rendering path directly), decodes the PNGs it wrote,
   and asserts the rectangle's pixel colour at ticks 0/1/2 matches
   `color_for_tick`'s alternation — three samples, per M0.1's flagged
   verification pitfall (two samples can't tell "advancing" from "stuck").
4. Update `README.md`: state plainly that Pages is the live, maintained path;
   the native fallback is proven and documented but not run/watched day to
   day.
5. Milestone-scope refactor pass over what this milestone added (steps 1-4)
   before closeout.

## 5. Adversarial focus

Same class of bug M0.1 found: verify the fallback's check can actually fail —
temporarily break `color_for_tick`'s alternation (or skip a render) and
confirm the integration test catches it, before trusting it long-term.
