# Round 1 — Grid and material representation

**Milestone:** M1.1 — Substrate and harness (Tranche 1 — Physics)

**Shape:** risky — full Red/Green/Refactor. This round establishes the shared
grid/material primitive every later round in this milestone, and every later
milestone in tranche 1, builds on directly (`cycle-plan`'s "blast radius
reaches past this round" criterion). See `cycle-log/tranche-1/m1.1/plan.md`
§4 Round 1 for the full reasoning.

## Goals

1. A `Grid` type: structure-of-arrays, double-buffered, addressed by
   `math::GridIndex` (existing, tested, currently unused by running code —
   this round is its first real caller). Fixed dimensions at construction.
   "Double-buffered" means the grid can produce a next-state buffer distinct
   from its current-state buffer for a step to write into, then swap —
   the exact step logic is NOT this round's job (that's round 2); this
   round's job is the buffer shape existing and swappable.
2. A `Material` struct that is *data*: density, viscosity, heat_capacity,
   conductivity, phase (an enum: at minimum Solid/Liquid/Gas), colour (for
   the later renderer), and evident room to grow (a struct, not a bag of
   special-cased constants). A small material table (e.g. a `Vec<Material>`
   or array indexed by a `MaterialId` newtype) holding at least 2-3 distinct
   materials (e.g. "empty/air", "water", "stone") as data, not per-material
   code paths.
3. Each grid cell holds (at minimum) a `MaterialId`, addressable via
   `GridIndex`. `Vec2`/`GridIndex` genuinely called by this real code (not
   forced into unrelated code) — closes the tranche-0-flagged gap.
4. A test pins the coordinate convention as applied to this new grid type
   specifically (not just re-asserting `math.rs`'s existing convention
   tests) — e.g. that indexing the grid at a `GridIndex` and indexing its
   underlying storage agree, and that the grid's width/height axes match
   `GridIndex`'s `i`/`j` in the way callers will expect.
5. Test tagging and a fast path established from this round onward: a way to
   run this round's (and later rounds') tests without pulling in anything
   slow (e.g. Playwright/e2e), documented with the exact commands.

## Intent

The bones: how a cell is represented. Every later milestone in tranche 1
(granular solids, liquids, pressure, gases, temperature) adds *behaviour* to
cells this round defines the shape of. No physics content this round — a
step function is explicitly round 2's job, not this one. Get the shape
(SoA, double-buffered, GridIndex-addressed, material-as-data) right now
because retrofitting it later is the expensive mistake tranche 0's own
Vec2/GridIndex gap already previewed at smaller scale.

## Scope and focus

**Scope:** new code only (`src/grid.rs` and/or `src/material.rs`, whatever
module layout Red chooses; `src/math.rs`'s existing `Vec2`/`GridIndex` may
lose their `#[allow(dead_code)]` once real code calls them, but their
implementations are not otherwise touched). **Focus:** the new grid/material
representation and its own test coverage; do not touch `src/lib.rs`'s M0.1
rendering code this round.

## Round 1 — Red — 2026-09-05T10:24:16+12:00 → 2026-09-05T10:31:00+12:00 (~7 min)

**What I did.**

Read `cycle-contract`, `cycle-red`, `CLAUDE.md`, `src/math.rs`,
`src/timestep.rs`, `src/lib.rs`, `Cargo.toml`, and this milestone's plan
(`cycle-log/tranche-1/m1.1/plan.md`) for context before writing anything.

Created two new modules:

- **`src/material.rs`** — `Phase` (enum: `Solid`/`Liquid`/`Gas`), `Material`
  (data struct: `density`, `viscosity`, `heat_capacity`, `conductivity`,
  `phase`, `colour`), `MaterialId` (newtype `u16` index), `MaterialTable`
  (owns `Vec<Material>`, looked up by `MaterialId`).
- **`src/grid.rs`** — `Grid`: fixed-size, structure-of-arrays,
  double-buffered (`current`/`next` parallel `Vec<MaterialId>`),
  `GridIndex`-addressed. `Grid::linear_index` is the new row-major (`i`
  fastest-varying) storage convention this round pins. `Grid::cell_center`
  delegates to `GridIndex::center`, giving `Vec2`/`GridIndex` their first
  genuine caller (see below).

Wired both `pub mod`s into `src/lib.rs` (module declarations only — no
change to any M0.1 rendering code: `draw`, `paint_rect`, `tick_and_draw`,
`advance_tick`, `color_for_tick` are all untouched).

Removed the `#[allow(dead_code)]` attributes from `src/math.rs`'s `Vec2`
struct/impl and `GridIndex` struct/impl (left `UP`'s attribute in place —
`UP` itself is still exercised only by its own test, unlike `Vec2`/
`GridIndex` which `Grid` now genuinely calls), and updated the module doc
comment to record the change. Confirmed both `cargo build --lib` and
`cargo build --lib --target wasm32-unknown-unknown` compile clean (one
expected `dead_code` warning on `Grid::next`, which is unread until Green's
stub bodies exist — not a real defect).

Established the fast-path test convention (goal 5) in `README.md`: `cargo
test --lib` (fast — in-crate unit/scenario tests only) vs. `cargo test`
(medium — adds the native-binary subprocess integration test) vs. the
Playwright e2e command (slow, never part of `cargo test`), plus a note for
future `#[ignore]` use if a genuinely slow `#[test]` shows up later.

**Successes.**

- All 12 new tests fail for the right reason: a named `todo!()` panic from
  an explicit stub, not a compile error, typo, or missing import. Confirmed
  by running `cargo test --lib` and reading every failure's panic message.
- The existing 38 tests (`math`, `timestep`, `lib`) are still green,
  unchanged in behaviour — only `math.rs`'s dead_code attributes and doc
  comment changed, not its logic.
- Both native and `wasm32-unknown-unknown` targets build cleanly with the
  new modules in the tree — confirmed directly, not assumed.
- Closed the `Vec2`/`GridIndex` gap non-artificially: `Grid` is
  `GridIndex`-addressed throughout (every `get`/`set`/`get_next`/`set_next`/
  `linear_index` takes a `GridIndex`), and `Grid::cell_center` is a genuine,
  natural use of `GridIndex::center` (world position of a cell — exactly
  what a later force/rendering round would want), not code forced in to
  tick a box.

**What was difficult, and where the time went.**

Nothing structurally difficult. Most of the time went into two design
decisions I wanted to get right rather than revisit later:

1. Where to draw the "plain plumbing (implement directly) vs. real decision
   (stub for Green)" line — landed on: `Grid::width`/`height`/`cells` and
   `Grid::cell_center` (pure delegation) implemented directly, matching
   `math.rs`'s own precedent for `Vec2::new`/`GridIndex::new`; everything
   that touches `linear_index`'s convention, buffer allocation, or the
   `MaterialId`-to-table-slot mapping left as a stub, since those are this
   round's actual content.
2. How to pin the grid/storage-agreement convention (goal 4) without
   over-specifying it — settled on exposing `Grid::cells()` (raw current
   buffer) and `Grid::linear_index()` (the mapping) as two independently
   stubbed pieces, then asserting they agree with `get()` — so Green commits
   to one row-major, `i`-fastest convention and the test catches any future
   drift from it, the same pattern `math.rs` itself uses for `GridIndex`.

**Compromises I made.**

- `MaterialTable::get`'s id-to-slot mapping is left fully open (stubbed) —
  I did not pin "direct indexing" as the required implementation, just that
  `get` must return what `new` was given at that position. This gives Green
  latitude (e.g. it could reasonably choose a `HashMap` instead of direct
  indexing) at the cost of not pre-deciding a possibly-obvious answer. Judged
  the right trade: the id-to-slot mapping is exactly the kind of "real
  decision" cycle-red says to leave for Green, even when the obvious answer
  is simple.
- I did not write a scenario test asserting `Grid::new`'s *next* buffer is
  independently readable/writable before any `set`/`swap` call (e.g.
  `get_next` on a freshly constructed grid returns `fill`) — covered
  implicitly by `next_buffer_write_is_isolated_from_current_until_swap` and
  `swap_exchanges_both_buffers_not_just_one_direction`, but not as its own
  named scenario. Judged sufficient coverage rather than a gap worth a
  13th test.

**Token usage.** Not visible to me — no context/usage indicator or
final-turn summary surfaced by this harness.

**Gaps and flags.**

- `Grid`'s out-of-bounds contract (`linear_index`/`get` panic on an
  out-of-range `GridIndex`) is pinned by one disposable `#[should_panic]`
  test, not a scenario — this round's goals didn't ask for bounds-handling
  as a first-class concern (round 2's step function is where wraparound vs.
  neighbour-access-at-the-edge questions will actually matter), so I judged
  a disposable smoke test sufficient rather than a durable scenario.
  Green/Refactor should treat the *panic* behaviour itself as provisional —
  round 2 or a later round may want clamping, wrapping, or an `Option`
  return instead once real step logic needs to reason about edges.
- I chose `usize` for `Grid::width`/`height`/`linear_index`'s return type
  (storage-space) versus `i32` for `GridIndex::i`/`j` (index-space,
  supports negative indices per `math.rs`'s own convention). This is a
  deliberate, load-bearing choice — a `Grid` only has non-negative valid
  storage positions even though `GridIndex` itself can express negative
  coordinates — but it means `linear_index` must reject negative `i`/`j` as
  out-of-bounds (covered by the existing bounds contract, not a new test
  written specifically for the negative case). Green should keep that in
  mind when implementing `linear_index`'s bounds check.
- `MaterialTable::reference()`'s exact material choices (which three-plus
  materials, their exact numeric properties) are entirely Green's to
  decide — the tests pin count (`>= 3`) and phase variety (contains
  `Solid` and `Liquid`) but nothing about "empty/air" specifically having
  zero density, etc. If the milestone's later rounds (round 3's headless
  mass measurement, in particular) need a specific reference-table shape
  (e.g. "empty" must have exactly zero density for a mass-conservation
  check to be meaningful), that's worth the round-3 planner naming
  explicitly rather than assuming Green's round-1 choice happens to fit.
- `README.md`'s new "Test tagging / fast path" section documents
  `cargo test --lib` as the fast command going forward; I did not add any
  `#[ignore]`-tagged tests this round because none of this round's tests are
  slow enough to warrant it (all 50 tests, `--lib`, run in ~0.01s per the
  test harness's own reported time) — the convention is written down ready
  for when a later round needs it, not exercised yet.

**General comments.**

Cargo (`~/.local/bin/cargo`, 1.98.1) was available and worked without
issue — the dev-env memory note claiming no rustc/cargo is stale as of this
session; worth updating that memory file at some point outside this round's
scope.

Restructuring: none. The five goals as handed to me mapped cleanly onto one
Red pass with no reordering or splitting needed.
