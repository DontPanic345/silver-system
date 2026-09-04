# M0.2 closeout — Deploy to GitHub Pages

**Closed out:** 2026-09-05T03:15+12:00

**Process note:** this milestone was run by a single forked agent rather than
the usual isolated Red/Green/Refactor phases dispatched via `cycle-round` —
the fork's hard rule prohibits it from spawning further subagents. So this was
one continuous pass, not three cold reads. Flagging this as a deviation from
the cycle for the parent orchestrator's awareness, not something to hide.
Given the milestone's small, mostly-declarative surface (a CI YAML file, a
README edit), the risk this creates is low, but it's a real gap from the
contract's normal isolation guarantee.

## Targets — met or not

From `PLAN.md` (M0.2):

1. **A GitHub Actions workflow builds the wasm artifact and publishes it to
   GitHub Pages on push.** Met. `.github/workflows/deploy-pages.yml`
   (commit `0ce8dc5`) builds via `scripts/build-wasm.sh` (the exact M0.1
   commands, no re-derivation) and publishes `www/` via
   `actions/configure-pages` + `actions/upload-pages-artifact` +
   `actions/deploy-pages`.
2. **Verify the deployed page actually loads and draws — a person opening the
   URL and seeing it, not CI green.** Met, doubly: `curl` against
   `https://dontpanic345.github.io/silver-system/` returns the real page
   (fetched directly, contents match `www/index.html`); separately, the user
   opened the URL themselves mid-session and reported "I see it, red/blue
   square" — direct human confirmation of the alternating-tick rectangle from
   M0.1, live.
3. **A public GitHub Pages URL exists and is recorded in the README.** Met —
   `https://dontpanic345.github.io/silver-system/`, recorded in `README.md`
   (commit `0ce8dc5`).
4. **A second push produces a second successful deploy with no hand-holding.**
   Met. First push (`0ce8dc5`) triggered run `33887381065`, completed
   `success`. This closeout commit is itself the second push — see the commit
   hash and run ID recorded below once it lands; no manual step was taken
   between the two.

## What was learned

- **GitHub Pages was already enabled on this repo** — a prior "pages build and
  deployment" run (`33803065359`, legacy Jekyll-style, predates this session)
  had already succeeded, meaning the one open risk flagged in the tranche plan
  (no `gh`/API token to enable Pages) never materialized as a blocker. Worth
  recording plainly: **this was not verified as generally true** — a repo
  where Pages has never been touched might still need the one manual
  Settings → Pages → Source: GitHub Actions click that `actions/configure-pages`
  cannot always perform unattended. Future projects starting from scratch
  should check this early rather than assume it away.
- The public, unauthenticated GitHub REST API (`api.github.com/repos/.../actions/runs`)
  is sufficient to poll workflow run status without `gh` or a token, for a
  public repo — this closes the verification gap flagged in tranche planning.
  Recorded here so M0.3 (and any later milestone needing CI status) doesn't
  re-discover it.
- Run time end-to-end (push → live Pages content) was under 3 minutes.

## Rounds run, timing

One continuous session by this forked agent — no round/phase split, per the
process note above. Roughly 15 minutes wall time including two ~2-minute CI
run waits (polled via the public API, no manual refreshing).

## Open gaps and flags carried forward

- **Phase isolation was not exercised this milestone** (see process note).
  M0.3 and beyond should return to normal dispatch (fresh non-fork agents per
  phase) — this milestone's simplicity is not typical, and future milestones
  with real logic changes need the isolation the contract specifies.
- Workflow uses `dtolnay/rust-toolchain@stable` and pins `wasm-bindgen-cli`
  to `0.2.127` inline (matching `Cargo.lock` at time of writing) — if the
  crate's `wasm-bindgen` dependency version ever changes, this line in
  `.github/workflows/deploy-pages.yml` needs updating too. Not automated;
  flagged for whoever next touches `Cargo.toml`'s `wasm-bindgen` version.
- M0.3 (the fallback) should still be built and proven, per the tranche plan's
  own logic ("build it now, while the stakes are zero") — M0.2's success
  doesn't remove M0.3's target, it just means M0.3's fallback path is true
  insurance rather than the active path.

## What the cycle itself got wrong

- The fork/no-subagent constraint colliding with `cycle-milestone`'s expected
  Red/Green/Refactor dispatch is worth a line in `cycle-contract` or
  `cycle-tranche`: if the orchestrator forks itself per milestone (as this
  session's user and orchestrator agreed on, to manage context growth), the
  fork needs to know upfront it cannot then dispatch fresh phase agents, and
  should default to running the milestone as one continuous pass rather than
  discovering the conflict mid-run. Flagging for whoever edits those skills
  next, not editing them unilaterally from inside a fork.

## PLAN.md

No change needed — M0.2 executed as scoped.
