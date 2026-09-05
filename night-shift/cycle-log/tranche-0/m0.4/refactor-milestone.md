# M0.4 — Milestone-scope Refactor

**Milestone:** M0.4 — Mathematical foundations (`cycle-log/tranche-0/m0.4/plan.md`)

**Scope:** the whole milestone — everything built across rounds 1-4
(`src/math.rs`'s `Scalar`/`Vec2`/`GridIndex`, `src/timestep.rs`'s
`FixedTimestep`, and the round-4 retrofit of `src/lib.rs`), per
`cycle-milestone` step 3.

**Focus:** general code quality and gap sweep across the whole milestone —
is the coordinate-convention doc comment actually clear (not just
test-pinned), is the numeric-type rationale written down somewhere
findable, are there loose ends across rounds 1-4 no single round's Refactor
caught, and a judgement call on the e2e-vs-native-test coverage gap round
4's Refactor flagged.

## Round — Refactor — 2026-09-05T09:00:15+12:00 → 2026-09-05T09:12:00+12:00 (~12 min)

**What I did.**

Read `cycle-contract` and `cycle-refactor`, then `cycle-log/tranche-0/m0.4/plan.md`
and all four round logs (round-01 through round-04, every Red/Green/Refactor
report) in full before reading any code, to see what each round already
flagged and checked. Then read `src/math.rs`, `src/timestep.rs`, `src/lib.rs`
and `www/index.html` cold. Ran `cargo test --lib`, `cargo build --lib`,
`cargo build --bin native_viewer`, `cargo clippy --lib --tests`.

**Successes.**

- **Coordinate-convention doc comments — genuinely clear, not just
  test-pinned.** `src/math.rs`'s `Vec2` doc comment states the math/physics
  `+y`-up convention, explicitly contrasts it with `src/lib.rs`'s `+y`-down
  canvas convention, and states the boundary obligation ("flip explicitly").
  `src/lib.rs`'s `RECT_X`/`RECT_Y` doc comment does the same in the other
  direction. `GridIndex`'s doc comment adds the cell-center-vs-corner
  decision with the exact formula, states the world-origin convention, and
  separately labels what's a *default* (cell-center) vs. what's *not
  claimed* (the only possible layout, explicitly naming M1.4 as where a
  staggered variant would go). All three doc comments cross-reference each
  other by name. A reader landing on any one of the three files would not
  need to re-derive the convention or guess at M1.4's boundary — this target
  is met with real margin, not just satisfied minimally.
- **Numeric-type rationale — written down, findable, genuine.** `src/math.rs`'s
  `Scalar` doc comment gives four specific, project-grounded reasons (GPU
  direction, architecture-contingent determinism, tolerance-based
  conservation, memory traffic) and states what would overturn the decision
  (a measured `f32` drift past a stated tolerance) rather than treating it as
  final. This was independently checked by round 1's own Refactor against
  the "genuine reasoning vs. rationalization" bar; re-reading it cold here, I
  agree with that verdict. It lives exactly where a future reader would look
  (on the `Scalar` type definition itself), not in a log or a comment
  elsewhere.
- **Cross-round loose ends swept — found none of substance.** Checked
  specifically for gaps no single round's Refactor could see because it only
  looked at its own diff:
  - `GridIndex` sits in `src/math.rs` alongside `Vec2` rather than its own
    module (round 2's placement call). At the file's current size (~380
    lines, roughly half tests, three primitives total) this still reads
    cleanly; no split needed yet. Consistent with round 2's own
    "considered, declined" note.
  - The reverse (world-position → `GridIndex`) conversion round 2 declined
    to build is still absent, and still not consumed by anything — no
    downstream pressure has appeared across rounds 3-4 to force it. Correctly
    left open per round 2's own reasoning; nothing to close here.
  - `Vec2`/`GridIndex`/most of `FixedTimestep`'s API (`dt`, `max_steps_per_call`,
    `accumulator`, `step_with`) remain unconsumed by any non-test code —
    every round from 1 onward flagged this as expected ("no caller until a
    later tranche/round"), and it is still true and still expected at
    milestone close: M0.4's own scope never included giving these a second
    caller beyond the round-4 retrofit, which only needed `FixedTimestep::advance`.
    Confirmed via `cargo clippy --lib`: exactly the same class of
    dead-code warnings every round already predicted, nothing new.
  - Checked whether `math::Scalar`/`timestep::Scalar` re-exports create any
    ambiguity now that both `src/math.rs` and `src/lib.rs` use `Scalar` —
    `src/lib.rs` imports it via `use math::Scalar;`, `src/timestep.rs` via
    `use crate::math::Scalar;`; one type, two import paths to the same
    definition, no duplication, no drift. No issue.
  - Checked `DT_SECONDS`'s derivation (`TICK_INTERVAL_MS as Scalar / 1000.0`)
    against `FixedTimestep`'s documented unit assumption ("seconds by
    convention") — consistent, and pinned by
    `dt_seconds_matches_tick_interval_ms_converted_to_seconds`. No drift.
  - Found and fixed one real, if minor, cross-round loose end: `cargo
    clippy --lib` still flagged `tick % 2 == 0` in `color_for_tick`
    (`src/lib.rs`) as `.is_multiple_of(2)` — first noted by round 1's
    Refactor, re-noted as out-of-scope by rounds 2 and 3's Refactors, and
    left unfixed through round 4 (which touched this exact file but not this
    exact line). Milestone scope is explicitly "the whole milestone", and
    this line lives in the file round 4's retrofit already widened scope to
    — fixed it (see Change list).
- **Suite and build health.** `cargo test --lib`: 35/35 green, 0.01s. `cargo
  build --lib` and `cargo build --bin native_viewer`: both compile clean,
  only the expected/predicted dead-code warnings. `cargo clippy --lib
  --tests`: the two `assertions_on_constants` warnings on
  `up_convention_pins_math_physics_y_up_not_canvas_y_down` and
  `rectangle_fits_within_canvas` are inherent to how those tests are
  written (asserting a property of a `const` value, by design, as the
  convention "tripwire" round 1 and M0.1 built) — not a defect, and
  reworking them into `const { assert!(...) }` blocks would make them less
  readable as tests without changing what they guard. Left as-is.

**What was difficult, and where the time went.**

Nothing was difficult. Most of the ~12 minutes went into reading all four
round logs in full before touching code (so this pass would actually catch
something no single round's narrower scope could), and into deliberately
re-deriving the coordinate convention and numeric-type reasoning from the
doc comments alone, as a fresh reader would, rather than trusting the round
reports' own verdicts on them.

**Compromises I made.**

None. The one change made (the clippy lint) is complete and low-risk.

**The e2e/native-test coverage gap — carried forward, not closed.**

Round 4's Refactor found, via a deliberate mutation, that
`tests/e2e/canvas_rectangle.test.mjs` cannot distinguish a genuine
`FixedTimestep`-driven `advance_tick` from a disguised per-call `+1` bypass
that still calls `FixedTimestep::advance` but discards its result — under
ordinary `setInterval` timing, both produce the same externally-observable
animation. I re-read that finding, re-derived it by hand (it's real: the
e2e test only polls `window.__tickCount`/pixel colour over wall-clock time,
never the reported step-count-per-call), and considered whether to close it
this pass.

**Decision: carry forward as an explicit gap, not worth closing now.**
Reasoning:

- The bug class this gap actually leaves uncovered is narrower than "the
  wasm/JS boundary is untested" — it is specifically "a bug that manifests
  *only* through the `#[wasm_bindgen]` export boundary itself, and not in
  `advance_tick`'s own logic". `advance_tick` is already directly, natively
  unit-tested with the exact scenarios that matter (sub-`dt` durations,
  multi-`dt` durations, cross-call accumulation) — round 4's own tests catch
  the disguised-bypass mutation immediately and correctly, as its adversarial
  pass demonstrated. What's left uncovered is a much thinner slice: `tick_and_draw`
  forwarding its `frame_duration_secs` argument to `advance_tick` incorrectly
  at the wasm-bindgen boundary itself (e.g. a stale generated binding, an
  argument silently dropped or defaulted).
- That remaining slice is mechanically generated, one line of real logic
  (`tick_and_draw`'s body is `advance_tick(frame_duration_secs)` then
  repaint), and was independently manually verified twice already — by
  Green (confirmed the generated `viewer.js` glue exports the new two-arg
  signature) and by Refactor (confirmed the same, plus that the live,
  deployed page's served JS glue carries the two-arg signature too). This is
  a much stronger check than an absent test suggests at first read.
- Closing it for real would mean standing up `wasm-bindgen-test` (or
  equivalent) infrastructure to run assertions inside an actual wasm/browser
  target rather than natively — genuinely new test infrastructure, not a
  quick addition, for a bug class this thin and already twice manually
  checked. That is disproportionate to the milestone's own framing ("small,
  boring substrate") and to CLAUDE.md's "do one thing at a time" — it would
  be new scope invented by this pass, not a fix to something this pass's
  focus asked for.
- If a future tranche's retrofit ever adds real decision logic *inside* a
  `#[wasm_bindgen]`-exported function itself (rather than in a plain Rust
  function it thinly wraps, which is this crate's consistent pattern so
  far), that is the point at which this gap's cost/benefit changes and
  `wasm-bindgen-test` infrastructure would be worth standing up. Not now.

This is a judgement call, not a target — recording the reasoning here so a
future planner doesn't have to re-derive it, and so it isn't silently
dropped for being "already flagged three times."

**Gaps and flags (carried forward, not new).**

- The e2e/native-test coverage gap above — explicit, reasoned, not closed
  this pass.
- `GridIndex` world-position → index reverse conversion — still absent,
  still no downstream consumer, correctly deferred (round 2's own call,
  re-confirmed here).
- `Vec2`/`GridIndex`/most of `FixedTimestep`'s surface remains uncalled by
  non-test code outside this milestone's own tests — expected, will resolve
  as later tranches consume this substrate, not a defect of M0.4.
- Operator-overload traits (`Add`/`Sub`/`Mul`) for `Vec2` — still not added
  (round 1's call), still nothing downstream asking for them.

**General comments.**

Adversarial pass tried and found nothing beyond the one lint fix: re-derived
the coordinate convention and numeric-type reasoning from scratch against
the doc comments (both held up); checked for import/re-export drift on
`Scalar` across the three files (none); checked `DT_SECONDS`'s unit
consistency against `FixedTimestep`'s documented assumption (consistent,
pinned by a test); re-ran the full native suite and both build targets
myself rather than trusting the round reports' numbers (all matched). This
milestone's four rounds were each carefully refactored in isolation
already — a milestone-scope pass mostly confirms that held up in aggregate,
which it did, plus catches the one thing repeated narrow scopes let slide
through three rounds running (the clippy lint).

**Change list.**

1. `src/lib.rs`: `tick % 2 == 0` → `tick.is_multiple_of(2)` in
   `color_for_tick` — clears a clippy lint first flagged in round 1's
   Refactor and re-flagged as out-of-scope by rounds 2 and 3; in scope now
   under this pass's whole-milestone reach. No behaviour change (verified:
   `cargo test --lib` 35/35 before and after).

**Correctness findings.** None. No shipped arithmetic, convention, or logic
was found wrong anywhere in the milestone's four rounds' worth of code.

**Suite runtime.** `cargo test --lib`: 35 tests, 0.01s. No concern, nothing
to collapse.

**The verdict: Advance — M0.4 is genuinely done.**

All five of the milestone's targets (restated in round 4's own Refactor,
re-checked here) are met: grid/vector primitives exist and are unit-tested;
the coordinate convention is pinned by both tests and doc comments that
genuinely agree and cross-reference each other; the numeric-type decision
has real, findable, project-grounded reasoning; the fixed-timestep harness
exists, is unit-tested, and is now actually driving real (non-test) code —
both locally and, per round 4's live-deploy check, in production; and the
retrofit was verified end-to-end including the live GitHub Pages URL. This
milestone-scope pass found no correctness issues anywhere across the four
rounds, fixed one small cross-round-flagged lint that no single round's
narrower scope reached, and made a reasoned, recorded decision to carry the
e2e-coverage gap forward rather than build new test infrastructure
disproportionate to the risk it closes. Nothing here should block M0.4's
closeout.

**What I would have done with another 30 minutes.** Sketch, without
building, what M0.5 (or whichever milestone first gives `Vec2`/`GridIndex`
a second real caller) would need from this substrate's API shape, so the
first round that finally consumes `Vec2`/`GridIndex`/`FixedTimestep::step_with`
for real isn't guessing — e.g. does a grid-storage type want
`GridIndex: Hash` (already derived) to key a `HashMap`, or a dense
`Vec`-backed array needing a linear-index conversion `GridIndex` doesn't
yet have. Left undone since it's next milestone's planning work, not this
pass's.

**Files touched.** `src/lib.rs` (one line). `cycle-log/tranche-0/m0.4/refactor-milestone.md` for this report.
