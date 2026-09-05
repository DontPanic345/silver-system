# Round 2 — Grid index type

**Milestone:** M0.4 — Mathematical foundations (`cycle-log/tranche-0/m0.4/plan.md`)

**Milestone intent:** the small, boring substrate every later tranche reaches
for — vector/grid primitives, a numeric type decision, a fixed-timestep
harness — got right once, here, rather than reinvented per-tranche.

**Round goals:**

1. Introduce a `GridIndex` (or equivalently named) integer `(i, j)` grid-cell
   coordinate type.
2. Decide and document, as a doc comment, the cell-center-vs-corner indexing
   convention: index `(i, j)` denotes a specific cell, and that cell's
   world-space position is its *center* (the default this milestone ships;
   staggered/corner values are explicitly deferred to M1.4, per the milestone
   plan).
3. A conversion function between a `GridIndex` and its `Vec2` world-space
   center position, given a cell size, under the pinned convention (using
   `Vec2` from round 1's `src/math.rs`) — with a fixed grid origin convention
   stated (e.g. index `(0, 0)`'s center sits at a stated offset from world
   origin).
4. Tests pin both the indexing convention (which is stated in the doc
   comment) and the conversion's correctness (e.g. index `(0,0)`'s center
   position under a stated cell size; index spacing between adjacent cells
   equals the cell size; round-tripping a position back to its index where
   applicable).

**Round 1 carried forward:** `src/math.rs` now has `Scalar = f32` and `Vec2`
with `add`/`sub`/`scale`/`dot`, a pinned `+y`-up convention, `UP` constant.
Round 1 recommendation was Advance (Refactor found no correctness issues).
Build on `src/math.rs` as it stands — do not re-litigate round 1's decisions.

**Push on vs. patch back:** pushing on. Round 1 advanced cleanly with no
carried-forward gaps that block round 2.

**Refactor scope/focus:** round scope (the grid module, and how it composes
with `src/math.rs`); focus on whether index-to-position conversion actually
round-trips/composes correctly (does converting an index to a position and
using cell size correctly reflect adjacency), and whether the cell-center
convention is stated plainly enough that a later milestone (M1.4) adding a
staggered variant would know exactly what this one guarantees and doesn't.

## Round 02 — Red — 2026-09-05T03:22:26+12:00 → 2026-09-05T03:34:00+12:00 (~12 min)

**What I did.**

Read `cycle-contract`, `src/math.rs` (round 1's `Scalar`/`Vec2`), `src/lib.rs`,
the M0.4 milestone plan, and this round's log header. Confirmed the pre-baked
goals still hold as framed — no restructuring needed; this round is a clean,
self-contained addition on top of round 1's `Vec2`, with no hidden ordering
problem (the grid type needs nothing from a later round, and later rounds
don't need the grid type to exist first except in spirit).

Added to `src/math.rs` (same module as `Vec2`, not a new sibling module — the
milestone intent groups "vector/grid primitives" together and `GridIndex`'s
only job right now is converting into a `Vec2`, so keeping them in one file
avoids a premature module boundary):

- `GridIndex { pub i: i32, pub j: i32 }` — the integer cell-coordinate type
  (goal 1). `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]` — `Eq`/`Hash`
  added beyond `Vec2`'s derives because integer grid indices are the natural
  key for a future `HashMap<GridIndex, _>` grid storage, at no cost now.
- A doc comment on `GridIndex` stating the cell-center-vs-corner decision in
  words (goal 2): index `(i, j)` denotes a cell, `center` returns that cell's
  *center*, the world origin is cell `(0,0)`'s lower-value corner (not its
  center), `j` increases in `+y` matching `Vec2`'s pinned y-up convention,
  and staggered/corner variants are explicitly named as deferred to M1.4
  rather than something this milestone needs to pre-solve.
- `GridIndex::new(i, j)` — plain construction, implemented directly (same
  reasoning round 1 applied to `Vec2::new`).
- `GridIndex::center(self, cell_size: Scalar) -> Vec2` — the conversion
  function (goal 3), stubbed with a `todo!()` naming exactly what Green
  should return: `Vec2::new((i + 0.5) * cell_size, (j + 0.5) * cell_size)`.

Added 5 new tests to the existing `#[cfg(test)] mod tests` block in
`src/math.rs` (goal 4). Ran `cargo test --lib` and confirmed all 5 fail on
the named `todo!()` panic (not a compile error, not a typo) and all 13
pre-existing tests stay green (18 total, 13 passed / 5 failed).

**Successes.**

The cell-center-vs-corner convention pin came out cleanly as two
complementary tests: one asserts what the convention *is* (`(0,0)`'s center
is `(0.5, 0.5)` at cell size 1) and one asserts what it's *not*
(`assert_ne!` against the world origin), so a future accidental switch to a
corner convention fails loudly rather than just quietly returning a
different-but-still-plausible-looking `Vec2`. The `j`-axis spacing test also
doubles as a cross-cutting convention pin: it reuses round 1's `+y`-up
convention (via `Vec2::sub` and an explicit `step.y > 0.0` assertion) so a
future change that broke either `Vec2`'s convention or `GridIndex`'s
composition with it would fail here too, not just in round 1's own tests.

**What was difficult, and where the time went.**

No real difficulty. The main judgment call was scope: the round's goal 3
wording ("a conversion function between a `GridIndex` and its `Vec2`... ")
could be read as asking for a bidirectional conversion (grid→world and
world→grid), and the round header's own goal 4 mentions "round-tripping a
position back to its index where applicable" as an example. I chose to ship
only the forward direction (`GridIndex::center`) this round: the goal is
phrased in the singular ("a conversion function"), the milestone plan's
round-2 description in `cycle-log/tranche-0/m0.4/plan.md` only names index→
position, and "where applicable" reads as optional rather than mandatory.
Flagged below for Refactor/planning to confirm or correct.

**Compromises I made.**

None on the required scope. I did not add a world-position→`GridIndex`
reverse conversion (see above) — scope judgment call, not a shortcut under
time pressure. `cargo test --lib` reports one pre-existing warning
(`unused variable: cell_size` on the stub) — expected and harmless for a Red
skeleton; Green's implementation will consume it and the warning will
disappear on its own.

**Gaps and flags.**

- Flag for Refactor/planning: confirm whether a reverse (world position →
  `GridIndex`) conversion belongs in this round's scope or a later one. I
  read the goal as index→position only (see above) and did not build it.
  If it's wanted, it's a small addition (e.g. `GridIndex::containing(pos:
  Vec2, cell_size: Scalar) -> GridIndex` using `floor(x / cell_size)`) but
  it's new logic, not plumbing, so it should go through Red→Green→Refactor
  like anything else, not be added ad hoc.
- The test suite has no tagging/fast-subset infrastructure yet (no
  `#[ignore]`, no feature-gated slow tests) — not needed yet, the whole
  suite runs in ~0.00s (18 tests), so I did not introduce one. Flagging so
  a future round that adds a genuinely slow test (e.g. a long-run
  conservation check) knows there's no precedent to follow yet and will
  need to establish one.
- `src/lib.rs`'s own hello-world logic/tests were left untouched, per my
  instructions.

**General comments.**

Test file: `src/math.rs` (tests live in its `#[cfg(test)] mod tests` block,
same file as the implementation — matches round 1's layout, no new test
file introduced).

Currently-failing tests (5, all failing on the same named `todo!()` stub,
which is the right reason — a panic naming exactly what's missing, not a
compile error):

- `math::tests::grid_zero_zero_center_is_offset_half_a_cell_from_world_origin`
  — pins the cell-center-not-corner convention itself.
- `math::tests::adjacent_indices_along_i_are_exactly_one_cell_size_apart_in_x`
  — pins index spacing along `i`/`x`.
- `math::tests::adjacent_indices_along_j_are_exactly_one_cell_size_apart_in_plus_y`
  — pins index spacing along `j`/`y`, and that `j`'s `+y` follows `Vec2`'s
  y-up convention.
- `math::tests::center_scales_linearly_with_cell_size_away_from_the_origin_cell`
  — disposable: a second cell size, away from the origin cell.
- `math::tests::center_handles_negative_indices_consistently`
  — disposable: no special-casing around zero.

Skeleton Green must fill:

- `src/math.rs`: `pub struct GridIndex { pub i: i32, pub j: i32 }` (fully
  implemented already, not a stub — plain data).
- `src/math.rs`: `impl GridIndex { pub fn new(i: i32, j: i32) -> Self }`
  (fully implemented already, not a stub — plain construction).
- `src/math.rs`: `impl GridIndex { pub fn center(self, cell_size: Scalar)
  -> Vec2 }` — **the one real stub**, currently `todo!()`. Expected body:
  `Vec2::new((self.i as Scalar + 0.5) * cell_size, (self.j as Scalar + 0.5)
  * cell_size)`.

Commands:

- This round's/module's tests: `cargo test --lib math::`
- Full suite (native): `cargo test --lib` (18 tests total, includes M0.1's
  `src/lib.rs` tests — all of those and round 1's `math` tests remain
  green: 13 passed, 5 failed, all 5 the new stubs above).
- Full suite including the browser e2e check (unaffected by this round,
  not re-run here since nothing wasm/rendering-facing changed):
  `tests/e2e/canvas_rectangle.test.mjs` via whatever Playwright command
  M0.1/M0.2's rounds established.

Did not restructure the slice — the pre-baked goals mapped cleanly onto one
round's worth of work with no hidden ordering problem.

Durable scenarios (restate the round's goals, should survive later
implementation changes): `grid_zero_zero_center_is_offset_half_a_cell_from_world_origin`,
`adjacent_indices_along_i_are_exactly_one_cell_size_apart_in_x`,
`adjacent_indices_along_j_are_exactly_one_cell_size_apart_in_plus_y`.

Disposable unit tests (may be deleted if the implementation's shape
changes): `center_scales_linearly_with_cell_size_away_from_the_origin_cell`,
`center_handles_negative_indices_consistently`.

Confirmed the rest of the suite is green: all 13 pre-existing tests (9 in
`math::tests` from round 1, 4 in `tests` from M0.1/M0.2/M0.3) pass unchanged.

## Round 02 — Green — 2026-09-05T03:25:01+12:00 → 2026-09-05T03:25:18+12:00 (~1 min)

**What I did.**

Read `cycle-contract`, Red's skeleton and doc comment on `GridIndex::center`,
and Red's report above. The doc comment, the round-header goals, and Red's
own stated expected body all agreed exactly, so there was nothing to
reconcile — filled the one real stub as named.

**File touched:** `src/math.rs` only.

Replaced the `todo!()` body of `GridIndex::center` with:

```rust
pub fn center(self, cell_size: Scalar) -> Vec2 {
    Vec2::new(
        (self.i as Scalar + 0.5) * cell_size,
        (self.j as Scalar + 0.5) * cell_size,
    )
}
```

This is exactly the arithmetic named in the stub's `todo!()` message and in
the `GridIndex` doc comment (cell-center convention, `(i+0.5, j+0.5) *
cell_size`) — no reinterpretation needed.

**Commands run and results.**

- `cargo test --lib math::` → `test result: ok. 14 passed; 0 failed; 0
  ignored; 0 measured; 4 filtered out`. All 5 of Red's new tests
  (`grid_zero_zero_center_is_offset_half_a_cell_from_world_origin`,
  `adjacent_indices_along_i_are_exactly_one_cell_size_apart_in_x`,
  `adjacent_indices_along_j_are_exactly_one_cell_size_apart_in_plus_y`,
  `center_scales_linearly_with_cell_size_away_from_the_origin_cell`,
  `center_handles_negative_indices_consistently`) pass, plus round 1's 9.
- `cargo test --lib` (full suite) → `test result: ok. 18 passed; 0 failed; 0
  ignored; 0 measured; 0 filtered out`. All 13 pre-existing tests (9
  `math::tests` from round 1, 4 `tests` from M0.1/M0.2/M0.3) plus the 5 new
  ones are green — 18/18, none skipped.
- `cargo build --lib` → 0 errors. 6 pre-existing `dead_code` warnings remain
  (`Scalar`, `Vec2`, `UP`, `GridIndex`, and their methods "never
  constructed/used") — expected and unchanged from round 1: this module
  isn't wired into `src/lib.rs`'s rendering path yet, per the module's own
  top-of-file doc comment. Not introduced or worsened by this round; nothing
  to fix here.

**Signature changes from Red's skeleton:** none. `center`'s signature,
parameter order, and return type are exactly as Red left them.

**Numbers changed:** none. No threshold, tolerance, or constant was touched
or proposed.

**Self-diff.** Read the diff: it is a single 6-line body replacing a
6-line `todo!()` call, nothing else changed in the file. No dead compute, no
stray doc-comment drift (the doc comment already described this exact
arithmetic and needed no edit), no leftover experiment code.

**Left for Refactor:** the reverse (world position → `GridIndex`) conversion
question Red flagged — still unresolved, still out of this round's built
scope, not something Green should decide unilaterally. Also left for
Refactor: whether `GridIndex` belongs in `src/math.rs` alongside `Vec2`
long-term or should move to its own module once more grid-shaped code
arrives (Red's call to keep it together for now stands unchallenged by me).

**Recommendation:** all 4 round goals met — `GridIndex` type introduced,
cell-center convention decided and documented, conversion function
implemented, and both durable and disposable tests pin convention and
correctness, all green. No blockers found. Ready for Refactor.

## Round 02 — Refactor — 2026-09-05T03:26:09+12:00 → 2026-09-05T03:27:20+12:00 (~1 min)

**What I did.**

Read `cycle-contract`, then read `src/math.rs` cold (without reading Red's or
Green's reasoning first, as instructed) before forming a view. Ran
`cargo test --lib` myself: 18/18 green, matching Green's report exactly. Then
read Red's and Green's log entries above to compare against my own read.

Adversarial pass on the stated focus:

- **Adjacency/composition.** Worked the arithmetic by hand:
  `center(i, j) = ((i+0.5)*cell_size, (j+0.5)*cell_size)`. Incrementing `i` by
  one moves `x` by exactly `(i+1.5 - i-0.5)*cell_size = cell_size`, `y`
  unchanged; incrementing `j` by one moves `y` by exactly `cell_size` in
  `+y`, `x` unchanged — no cross term, no sign inversion. `j` maps straight
  to `y` with no flip, so it correctly inherits round 1's `+y`-up convention
  rather than the canvas's `+y`-down one. This matches the two adjacency
  tests (`adjacent_indices_along_i_...`, `adjacent_indices_along_j_...`)
  exactly, and I don't find a case where they'd pass on a wrong composition
  — both assert the exact displacement vector *and* the `j` test separately
  asserts `step.y > 0.0`, not just `step.x == 0.0`, so a `-y` regression
  would be caught. No correctness issue found here.
- **Convention documentation.** The doc comment on `GridIndex` states: what
  `(i, j)` denotes, that `center` returns the cell's center (with the exact
  formula spelled out, not just prose), where the world origin sits relative
  to cell `(0,0)` (its lower-value corner, explicitly said never to be
  returned by `center`), that `j` follows `+y`-up (with the canvas contrast
  restated), and — separately, under its own subheading — that cell-center
  is *this milestone's default*, not the only possible layout, naming
  exactly what M1.4 would add (staggered/face/corner values) and stating
  plainly that `GridIndex`/`center` do not need to anticipate it. I judge
  this sufficiently plain for a later milestone: it distinguishes "what this
  guarantees" (the formula, the corner-vs-center fact) from "what it doesn't
  claim" (the only layout) in separate, clearly-labeled sections.
- **Edge cases.** Negative indices: the formula has no special-casing around
  zero (confirmed by the disposable test and by inspection — plain
  multiplication, no `abs`/`floor` branch), so negative cells behave
  consistently; correct. Zero or negative `cell_size`: not guarded anywhere
  in the module (`Vec2`/`Scalar` don't validate their inputs either, so this
  matches the surrounding style) — worked both by hand: `cell_size == 0.0`
  collapses every cell's center to the world origin (degenerate but no
  panic, no NaN); `cell_size < 0.0` flips the sign of the offset consistently
  (still no panic). Judged this worth a one-line doc note rather than a
  runtime check or a new test, since a physically-meaningless but
  arithmetically-consistent result for a nonsensical input matches this
  module's existing philosophy of not validating inputs (`Vec2::scale(0.0)`
  is exercised as a real scenario, not guarded against, for the same reason).
- **File organization.** `src/math.rs` now holds two primitives (`Vec2`,
  `GridIndex`) plus their tests in one file/module. At ~380 lines (roughly
  half tests), I judge this still reads cleanly and matches the milestone
  intent's own framing ("vector/grid primitives" as one group) — no split
  needed yet. Considered splitting the `tests` module into `vec2_tests`/
  `grid_tests` submodules for readability; declined — the existing `// ---`
  section comments already partition it clearly and a submodule split would
  be pure churn with no behavior or clarity gain proportional to the file's
  current size.
- **Test suite health.** 18 tests, 0.00s runtime — no budget concern, nothing
  to collapse or tag. Confirmed no test is tautological (each round-2 test
  asserts a specific numeric result or inequality tied to the stated
  convention, not just "doesn't panic").
- **Correctness findings:** none. The shipped arithmetic is right; I found
  a documentation-drift issue (below), not a computation bug.

**Successes.**

Found one real doc-comment drift bug: `GridIndex::center`'s doc comment
still read "so it is left as a stub for Green" after Green had already
replaced the `todo!()` with the real implementation — exactly the "a comment
describing both the old scheme and the new one is worse than no comment"
trap the skill calls out. Fixed it to state `center`'s actual precondition
(`cell_size` assumed positive) instead of narrating a now-false process
fact. Also found the module's top-of-file doc comment listed only `Scalar`
and `Vec2`, silently out of date the moment `GridIndex` was added — fixed to
mention all three.

**What was difficult, and where the time went.**

Nothing difficult. Most of the ~1 minute went to hand-verifying the
adjacency arithmetic and the zero/negative-`cell_size` cases before deciding
neither needed a runtime guard, and to reading the milestone plan
(`cycle-log/tranche-0/m0.4/plan.md`) to settle the reverse-conversion
question below with real context rather than guessing.

**Compromises I made.**

None. Both fixes are in scope and complete.

**Gaps and flags.**

- **Resolved the open question Red/Green flagged: does a reverse
  (world position → `GridIndex`) conversion belong in this round?**
  Decision: **no, declined for this round.** Reasoning: (1) the round-header
  goal as I was given it says "a conversion function" singular, and the
  milestone plan's own round-2 description
  (`cycle-log/tranche-0/m0.4/plan.md` §4) lists only "a conversion function
  between a grid index and its `Vec2` world-space position" as the round-2
  deliverable — index→position, not the reverse; (2) nothing in round 3
  (fixed-timestep harness) or round 4 (retrofit) as scoped in that same plan
  consumes a reverse conversion, so there's no downstream pressure forcing
  it now; (3) a reverse mapping is new decision-worthy logic, not plumbing —
  it has to pick a rule for `floor` vs `round`, and for which cell owns a
  boundary that lands exactly on a multiple of `cell_size`, and for negative
  positions — exactly the kind of thing that should go through its own
  Red→Green→Refactor pass with its own tests pinning the boundary rule, per
  cycle-contract's "you are not a source of new features" limit on this
  phase and CLAUDE.md's "do one thing at a time". I did not add it.
  I do note the milestone plan's own §5 (Refactor scope per round) says
  round 2's focus is partly "does converting an index to a position and
  back round-trip correctly" — with no reverse function existing, there is
  currently nothing to round-trip. Flagging for the planner: either that
  phrase anticipated a reverse conversion that never got scoped into round
  2's actual goal list, or it was loose language about the forward direction
  only. Worth a future round (or M1.4, if the staggered-grid work needs it
  first) if and when something downstream actually needs
  position→`GridIndex`.
- Dead-code warnings (6, on `Scalar`/`Vec2`/`UP`/`GridIndex`/methods) are
  unchanged from Green's report — expected, this module isn't wired into
  `src/lib.rs` yet, not a new issue.
- Test/file organization is fine as-is for now; flagged above only as a
  "considered, declined" note for whoever next touches this file with more
  content to add.

**General comments.**

Change list:

1. `src/math.rs` top-of-file module doc comment: mention `GridIndex`
   alongside `Scalar`/`Vec2` — it existed but wasn't named. Doc-drift fix.
2. `src/math.rs` `GridIndex::center` doc comment: replace the stale
   "left as a stub for Green" sentence (Green already filled it) with a
   stated precondition (`cell_size` assumed positive) — doc-drift fix plus
   a small real gap (the precondition was previously unstated).

Both are doc-only; no test, signature, or arithmetic changed.
`cargo test --lib`: 18 passed, 0 failed, 0 ignored — same as Green left it,
runtime 0.00s (unchanged, no budget concern). `cargo build --lib`: 0 errors,
6 pre-existing dead-code warnings (unchanged from Green's report).

**Verdict: Advance.** All 4 round goals are met to a sufficient standard:
`GridIndex` exists with a clean integer `(i, j)` representation; the
cell-center-vs-corner convention is decided and documented plainly enough
(with an explicit, separately-labeled statement of what's *not* claimed, for
M1.4's benefit); the forward conversion function is implemented and its
arithmetic is verified correct by hand, not just by the tests it happens to
pass; and the tests pin both the convention (including a negative assertion
against the corner interpretation) and the conversion's correctness
(adjacency along both axes, scaling, negative indices). The one real defect
found — the stale doc comment — was a documentation issue, not a
computation bug, and is fixed. The reverse-conversion question is resolved
with reasoning (declined for this round, flagged forward) rather than left
open. No blockers to advancing to round 3.

**With another 30 minutes** I would: (a) write a small property-style probe
sweeping a range of `(i, j, cell_size)` triples (including large-magnitude
`i`/`j` near `i32` bounds) to check for `f32` precision loss in the
`as Scalar` cast becoming visible at extreme indices — not expected to
matter at this milestone's scale, but cheap to confirm; (b) sketch what a
`GridIndex::containing(pos, cell_size)` reverse conversion's test list would
look like (boundary-cell ownership, negative positions) as a head start for
whichever future round picks it up, without implementing it.
