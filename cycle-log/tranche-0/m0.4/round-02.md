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
