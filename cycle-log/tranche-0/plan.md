# Tranche 0 plan — Mathematics and tooling foundations

**Planned:** 2026-09-05T02:33:07+12:00

## 0. What's been read

No prior rounds exist for tranche 0 — this is the first planning pass in the
cycle. Read `PLAN.md` (tranche 0 section in full) and `CLAUDE.md`. Two shelved
experiments exist for reference only (`terrarium/`, `stable-fluids/`) — not to
be extended, but worth knowing they exist as prior art on what broke (the
pressure-solve checkerboard mode, informally-derived grid conventions).

Before planning, I resolved what the dev-env memory recorded as a blocking gap:
**rustc/cargo were believed absent from this machine.** They are not — the
toolchain installs cleanly via rustup in under a minute (`rustc`/`cargo`
1.98.1), and the only real snag was that `~/.cargo/bin` isn't on `PATH` for a
fresh non-interactive shell (no `BASH_ENV`, so per-call shells don't source
`.bashrc`/`.profile`). Fixed by symlinking `~/.cargo/bin/*` into `~/.local/bin`,
which is already on the default `PATH`. Memory updated (`dev-env-gaps`). Only
the `x86_64-unknown-linux-gnu` target is installed so far — `wasm32-unknown-unknown`
is M0.1's first job, not done yet.

Also checked before planning: `git remote -v` shows `origin` is
`https://github.com/DontPanic345/silver-system.git` — a real GitHub remote
exists, so M0.2's GitHub Pages target is reachable in principle. No `gh` CLI
and no `GITHUB_TOKEN`/`GH_TOKEN` in this environment, so nothing here can call
the GitHub API directly; Pages setup has to go through committed workflow
files and whatever `actions/configure-pages` can self-enable at CI time. This
is a genuine open risk for M0.2, flagged below rather than assumed away.

## 1. Intent, sharpened

`PLAN.md`'s intent stands: de-risk the tooling before any physics is built on
it, and lay down the small mathematical substrate every later tranche reaches
for. Rust-to-web was flagged as the biggest unknown; today's check narrows
that unknown from "is Rust even usable here" (resolved: yes) to "does
Rust-to-wasm-to-browser work, and does GitHub Actions publish it without a
human clicking anything" (still open — that's the real content of M0.1–M0.3).

**How this serves the north star:** both statements of the north star are
unreachable if nobody can watch the result. This tranche's entire job is
proving the "terrarium people can see on their screens" half is buildable at
all, cheaply, and proving it now — before three more tranches of physics,
chemistry and biology get built on an assumption that might not hold.

## 2. Reach — what PLAN.md doesn't say but this tranche should cover

- PLAN.md's M0.1 says "a coloured rectangle is enough" — but the fixed-timestep
  harness (M0.4) needs to actually drive *something* visible for its target
  ("exercised by the M0.1 hello-world so it isn't a paper exercise") to be
  honest. So M0.1's hello-world should end up as: a rectangle whose position or
  colour changes once per fixed step, not a single static draw call. Small
  reach, but it's the difference between M0.4's target being real or nominal.
  Recorded here so M0.1/M0.4 planning doesn't lose it.
- A repo-root `README.md` already exists (checked: 1.3KB, mentions nothing
  about a live URL yet) — M0.2 and M0.3 both write to it. Plan those rounds to
  edit it in place, not create a second doc.
- Toolchain versions actually used (`rustc 1.98.1`, `cargo 1.98.1`, the wasm
  target once added) should be pinned somewhere checked-in (a `rust-toolchain.toml`
  is the idiomatic place) so CI and this machine can't silently drift apart —
  not asked for explicitly, but exactly the kind of tooling risk this tranche
  exists to remove.
- M0.4's vector/grid primitives should include the two things the JS attempt
  paid for informally: an explicit written statement of the coordinate
  convention (which axis is "up", cell-center vs. corner indexing) as a doc
  comment, not just a test — the test pins it, the doc comment stops someone
  re-deriving it by reading test names.

## 3. Tranche targets

Restating `PLAN.md`'s three targets, unchanged — they're already concrete and
correctly ordered as a fallback chain:

1. Something reachable at a public GitHub Pages URL, built by CI from this
   repo's Rust source — not a demo, anything visible counts.
2. If (1) genuinely can't be made to work, a working alternative (native
   binary + Playwright screenshot capture, per the project's existing
   visual-verification preference) exists and is documented, decided before
   physics work starts.
3. Grid/vector primitives physics needs exist, are unit-tested, and are
   exercised by real code (the M0.1 hello-world's step loop), not sitting
   unused.

Adding one measurable target of my own, since "cheaply" is part of the stated
intent and is otherwise unmeasured:

4. The full loop — `cargo build` through a browser tab showing something — is
   reproducible from a clean checkout with a stated, short list of commands
   (recorded in the README), so a future tranche never has to re-discover how
   this works.

## 4. Milestones

Kept in `PLAN.md`'s order — each is a hard dependency of the next:

- **M0.1 — Toolchain proving ground.** Confirm wasm32 build + a minimal
  rendering approach + something drawn in a real browser, served locally.
  Produces the first thing a human can watch (locally).
- **M0.2 — Deploy to GitHub Pages.** Same artifact, reachable at a public URL
  via CI with no manual step, if M0.1's approach allows it.
- **M0.3 — The fallback.** Build and prove the native+Playwright path
  regardless of whether M0.2 succeeds, per the tranche's own stated logic
  ("build ... now, while the stakes are zero") — state plainly which path is
  actually in use, and don't maintain both once one is proven.
- **M0.4 — Mathematical foundations.** Vector/grid primitives, numeric type
  decision, fixed-timestep harness — depends on nothing above it technically,
  but is ordered last because M0.1's hello-world needs to actually call it
  (see Reach, above) for M0.4's target to be real. M0.4 could in principle run
  in parallel with M0.2/M0.3; I'm sequencing it last anyway to keep this
  tranche's rule of "do one thing at a time" honest — revisit only if a later
  planning pass shows a real cost to the ordering.

## 5. Push on vs. patch back

Nothing to patch back — no prior rounds exist. Pushing on: start M0.1.

## 6. Deferred / flagged forward

- **GitHub Pages enablement without `gh`/API access** is the single biggest
  open risk in this tranche. `actions/configure-pages` can usually self-enable
  Pages for a public repo given the right workflow permissions, but that's
  untested in this environment. If M0.2 finds it can't self-enable, the exit
  ramp is: tell the user the one manual click needed (Settings → Pages →
  Source: GitHub Actions), not treat it as an M0.2 failure — this is an
  external-service constraint, not a tooling defect in our control.
- Whether `canvas`+`wasm-bindgen`/`web-sys` or a WebGL/WebGPU crate is the
  right M0.1 choice is explicitly left to M0.1's round-level planning, per
  PLAN.md's instruction to choose on what actually builds and runs here.
