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

## Round 1 — Green — 2026-09-05T10:31:06+12:00 → 2026-09-05T10:31:47+12:00 (~1 min)

**What I did.**

Read `cycle-contract`, `cycle-green`, `CLAUDE.md`, and this round log in full
(including Red's report) before touching anything. Read `src/grid.rs`,
`src/material.rs`, and `src/math.rs` to understand the stubs and the
conventions they must honour.

Filled in all eight `todo!()` stubs Red left, in place, with no signature
changes:

- `src/grid.rs`:
  - `Grid::new` — allocates `current`/`next` as `vec![fill; width * height]`.
  - `Grid::linear_index` — row-major, `i` fastest-varying
    (`j * width + i`), with two `assert!`s rejecting negative or
    out-of-range `i`/`j` before the cast to `usize` (Red's report flagged
    this negative-`i`/`j` case explicitly as something to keep in mind).
  - `Grid::get`/`set`/`get_next`/`set_next` — each a one-line read/write
    through `linear_index` into `current`/`next` as documented.
  - `Grid::swap` — `std::mem::swap(&mut self.current, &mut self.next)`.
- `src/material.rs`:
  - `MaterialTable::get` — `&self.materials[id.0 as usize]`, i.e. direct
    indexing by the `MaterialId`'s raw value (the simplest of the options
    Red's report deliberately left open).
  - `MaterialTable::reference` — built three materials as data: "empty/air"
    (density 0.0, viscosity 0.0, heat_capacity 1.0, conductivity 0.0,
    `Phase::Gas`, black), "water" (density 1.0, viscosity 0.5, heat_capacity
    4.186 — water's real specific heat in J/(g·K), a real physical constant
    rather than an arbitrary placeholder, conductivity 0.6, `Phase::Liquid`,
    blue), "stone" (density 2.5, viscosity 0.0, heat_capacity 0.8,
    conductivity 2.0, `Phase::Solid`, grey). Chose zero density for
    empty/air specifically, per Red's report flagging that a future
    mass-conservation check (round 3) may need exactly that.

Ran `cargo test --lib`: all 50 tests pass (the 12 previously-`todo!()`
tests plus the 38 pre-existing ones, unchanged). Ran `cargo build --lib
--target wasm32-unknown-unknown`: compiles clean, no errors, no warnings.
Also ran plain `cargo build --lib` and `cargo clippy --lib`: both clean,
no warnings.

Read my own diff (`git diff -- src/grid.rs src/material.rs`) start to
finish before committing — every hunk is exactly a stub body filled in,
nothing else touched, no leftover experiments or dead code.

**Successes.**

- All 12 target tests pass; the 38 pre-existing tests are unaffected.
- Both native (`cargo build --lib`) and `wasm32-unknown-unknown` targets
  build clean with no warnings.
- No signature changes were needed — Red's skeleton was implementable
  exactly as given.
- `reference()`'s empty/air material has density `0.0` exactly, addressing
  Red's flagged concern about round 3's mass-conservation check without
  round 3 having to renegotiate this round's choice.

**What was difficult, and where the time went.**

Nothing difficult. Every stub was either pure plumbing (buffer alloc,
one-line indexed read/write, a swap) or a bounded data choice
(`MaterialTable::reference`'s concrete numbers, `get`'s indexing strategy)
that Red's report had already scoped tightly. Almost all the time went to
reading the round log, the two new modules, and `math.rs` before writing
anything, per "do one thing at a time."

**Compromises I made.**

None. `MaterialTable::get` uses the simplest valid mapping (direct
indexing) rather than a `HashMap` or similar — Red's report explicitly
left this open and direct indexing is the natural, ungimmicked choice
for a `Vec`-backed table addressed by a small integer id; no
data-structure change is implied for later rounds unless a real need
(e.g. sparse/non-contiguous ids) shows up.

**Token usage.** Not visible to me — no context/usage indicator or
final-turn summary surfaced by this harness.

**Gaps and flags.**

- `linear_index`'s out-of-bounds behaviour is still a panic (via
  `assert!`), matching the round log's note that this is provisional —
  Green kept it as-is since goal 4 and the round's scope did not ask for
  bounds-handling to change; round 2 or later is where clamping/wrapping/
  `Option` might get decided, per Red's own flag.
- `reference()`'s exact numeric values (beyond empty/air's density, which
  I deliberately zeroed) are my choice and not pinned by any test —
  water's `heat_capacity` uses water's real specific heat (4.186) as a
  plausibility anchor since nothing in the suite required a specific
  number; if a later round needs different reference values for a
  specific measurement to come out cleanly, that's a renegotiation for
  that round's planner, not implied by this choice.
- I did not touch `src/lib.rs`, `src/math.rs`'s logic, or `README.md` —
  out of this round's scope and untouched by my diff.
- The stray `test/` directory (three `tick-*.png` files) present in
  `git status` before and after my changes is not something I created —
  left it uncommitted/untouched, out of scope for this phase.

**General comments.**

Straightforward round: Red's skeleton and report left no ambiguity that
mattered, so Green was close to mechanical. Nothing to flag for Refactor
beyond what's already noted above and in Red's own report (the
out-of-bounds contract's provisional status, and `reference()`'s
concrete values being open to a future round's needs).

## Round 1 — Refactor — 2026-09-05T10:33:06+12:00 → 2026-09-05T10:34:53+12:00 (~10 min, including read time before the first clock check)

**What I did.**

Read `cycle-contract` and `cycle-refactor`, then read this round's full log
(Red's and Green's reports) before touching any code. Read `src/grid.rs`,
`src/material.rs`, `src/math.rs`, `src/lib.rs`, and `src/timestep.rs` cold,
end to end — not just the diff — per scope. Skimmed `CLAUDE.md`,
`cycle-log/tranche-1/plan.md`, and `cycle-log/tranche-1/m1.1/plan.md` for
the milestone's fuller intent. Also skimmed `src/bin/native_viewer.rs` and
`tests/*` while sweeping the wider crate for mechanical issues; found
nothing to fix there.

Worked the four focus angles I was given:

1. **Repeated swap.** `Grid::swap` is `std::mem::swap(&mut self.current,
   &mut self.next)` — a real exchange of ownership, correct under
   arbitrary repetition by construction (`mem::swap` is its own inverse;
   there is no partial-state or aliasing risk with two disjoint `Vec`
   fields). The existing tests covered one swap and one swap-then-reverse;
   nothing exercised more than two swaps in sequence, which is exactly
   what a real step loop will do. Added a durable scenario test,
   `swap_behaves_correctly_across_many_repeated_swaps_in_sequence`, that
   drives seven ticks (write-to-next, swap, assert current) and confirms
   an untouched cell stays untouched throughout. It passes, confirming
   there is no latent bug here — but the coverage gap was real, so I
   closed it rather than just reporting "found nothing."
2. **`linear_index` vs. `GridIndex`'s own convention.** `GridIndex` pins
   `i`→world `x`, `j`→world `y` (`+y` up), cell-center indexing (see
   `math.rs`). `Grid::linear_index` computes `j * width + i` — row-major,
   `i` fastest-varying — which is exactly what the two dedicated tests
   (`stepping_i_by_one_moves_exactly_one_position_in_storage`,
   `stepping_j_by_one_moves_exactly_one_row_stride_in_storage`) pin, and
   what I independently re-derived by hand from `linear_index`'s source.
   No inconsistency: adjacent `i` are adjacent in storage, adjacent `j`
   are a full row apart, matching what a later round's neighbour-access
   code (`get(GridIndex::new(i+1, j))` etc.) will expect. The one thing
   *not* pinned by this round, correctly so (it's a rendering concern, out
   of scope until M1.1's later renderer round): storage order says nothing
   about which way is "up" on screen, since `GridIndex`'s `+y`-up and the
   canvas's `+y`-down (per `lib.rs`) disagree. I added a module-doc note
   flagging this explicitly so a future renderer round doesn't rediscover
   it as a flipped-image bug — this is the same flip `math.rs`'s own
   `Vec2` doc comment already requires generically, just named at this
   module's own instance of it.
3. **`MaterialTable::reference()`'s data.** Checked for internal
   consistency and round-3 mass-conservation plausibility. Density
   ordering is sane (air 0.0 < water 1.0 < stone 2.5) and air's density
   being exactly zero is deliberate and load-bearing for a later
   mass-conservation check, as both Red and Green's reports already flag.
   One real finding: `heat_capacity` uses water's actual physical specific
   heat (4.186 J/(g·K)) while `density` is an explicitly unpinned,
   effectively water-normalised unit (per `Material`'s own doc comment on
   `density`) — mixing a real physical constant with an arbitrary-unit
   quantity in the same struct. Not a bug today (nothing multiplies them
   together yet), but a trap for a later round that computes real energy
   (`heat_capacity * mass`) and assumes the units compose. Added a doc
   note on `reference()` naming this explicitly so that round pins a real
   unit system for `density` deliberately, rather than inheriting a false
   assumption that these numbers are already dimensionally consistent.
4. **Vec2/GridIndex gap, closed honestly?** Yes. `Grid`'s entire public
   surface (`get`/`set`/`get_next`/`set_next`/`linear_index`) is
   `GridIndex`-addressed, and `Grid::cell_center` is a genuine, natural
   delegation to `GridIndex::center` (a cell's world position — exactly
   what a later force/rendering round needs), not code invented to tick a
   box. This matches `CLAUDE.md`'s and the tranche-0/tranche-1 plans'
   framing of the gap and how it should close. No further action needed
   here.

General code-quality sweep: `cargo clippy --lib --all-targets` is clean
(no warnings) both before and after my changes; no dead imports, no stray
`dbg!`/`todo!`, no stale doc comments found beyond the units note above.
Doc comments throughout both new modules are accurate against the code as
written. `src/bin/native_viewer.rs` and `tests/native_fallback.rs` are
unrelated to this round and untouched, no issues noticed in them either.

**Successes.**

- Found and closed a genuine coverage gap (repeated swaps) with a durable
  test, not a throwaway probe — it stays in the suite.
- Found and flagged a genuine (if not yet exercised) unit-consistency trap
  in `MaterialTable::reference()` before any downstream round could build
  on the false assumption that `density` and `heat_capacity` share a unit
  system.
- Confirmed, by hand re-deriving `linear_index` and cross-checking against
  `GridIndex`'s own doc comment, that the storage convention this round
  pins is genuinely correct — not just test-covered, independently
  verified.
- All 51 tests pass (50 existing + 1 new), `cargo clippy --lib
  --all-targets` clean, `cargo build --lib --target
  wasm32-unknown-unknown` clean — all reconfirmed after my edits, not just
  before.
- Working tree left clean apart from the pre-existing, unrelated `test/`
  directory (three stray `tick-*.png` files, present before this round
  and explicitly out of scope per Green's report — left untouched).

**What was difficult, and where the time went.**

Nothing structurally difficult. Most of the time went into reading —
Red's and Green's full reports, then all five touched-or-adjacent source
files cold — before writing anything, per "do one thing at a time" and
the phase's own "read it cold" instruction. The four focus questions were
each answerable directly from the code and existing tests; the actual
changes (one new test, two doc notes) were quick once the reading was
done.

**Compromises I made.**

None of substance. I chose to add a doc note rather than restructure
`Material`/`MaterialTable` to pin a real unit system for `density` myself
— that is a genuine design decision (what units, what does 1.0 density
mean physically) that belongs to whichever round first needs it to be
concrete (plausibly M1.1 round 3's mass measurement, per Red's own flag),
not something to improvise here under a refactor phase's mandate not to
add new features.

**Token usage.** Not visible to me — no context/usage indicator or
final-turn summary surfaced by this harness.

**Gaps and flags.**

- The unit-system question for `density` (what does `1.0` mean physically,
  and does it compose with `heat_capacity`'s real J/(g·K) units) is still
  open. Worth the round-3 planner naming explicitly, since that's the
  round Red's and Green's own reports already point to as the first real
  consumer of mass/energy quantities.
- The renderer-round y-flip note I added to `grid.rs` is documentation
  only — no behaviour to test yet, since no renderer exists. Whoever
  writes M1.1's renderer round should read it before assuming a naive
  `cells()`-to-image-rows walk is correct.
- `linear_index`'s out-of-bounds behaviour remains a panic, unchanged —
  Red and Green both already flagged this as provisional pending round
  2's step-function needs (wraparound/clamping/`Option`); I found no new
  reason to revisit it this round and didn't.
- The stray `test/` directory (three `tick-*.png` files) is still present,
  untracked, unrelated to this round — flagged again since two phases in a
  row have now noticed it and left it; worth someone deciding whether to
  `.gitignore` it or delete it, outside any single phase's scope.

**General comments.**

Adversarial pass summary: I specifically tried (a) driving `swap` many
times in a row looking for state corruption or buffer-identity drift —
found none, but the coverage gap was real and is now closed; (b) hand
re-deriving `linear_index` against `GridIndex`'s documented convention
looking for an inverted axis or an off-by-one at a boundary — found none;
(c) checking `MaterialTable::reference()`'s numbers against each other and
against what round 3's mass-conservation check would need — found the
unit-mixing issue above; (d) re-reading `math.rs`'s and `grid.rs`'s doc
comments against the actual code looking for a doc/behaviour mismatch —
found none beyond the units gap already named. All four were genuine
adversarial attempts, not just re-reading Green's own report back.

Suite runtime: 0.01s for `cargo test --lib` (51 tests) — well within
budget, nothing to collapse or flag.

**The verdict: Advance.**

Reasoning: all five of the round's stated goals are met and verified,
not just claimed —

1. `Grid` is fixed-size, SoA (one `Vec<MaterialId>` field so far, by
   design), double-buffered, `GridIndex`-addressed, and now verified
   swappable under repetition, not just once.
2. `Material` is a growable data struct (density, viscosity,
   heat_capacity, conductivity, phase, colour), and `MaterialTable::
   reference()` holds three genuinely distinct materials as data — no
   per-material code paths anywhere in either module.
3. Every grid cell holds a `MaterialId`, addressed via `GridIndex`
   throughout `Grid`'s public API — genuinely, not artificially (see
   focus 4 above).
4. `grid_index_lookup_and_raw_storage_lookup_agree` and the two
   axis-stepping tests pin the grid/storage convention specifically for
   this new type, independent of `math.rs`'s own convention tests.
5. The fast-path test convention (`cargo test --lib`) is documented in
   `README.md` and the suite runs in 0.01s — genuinely fast, not just
   labelled so.

The one substantive finding (the `density`/`heat_capacity` unit mismatch)
is not a defect in what this round promised — no goal asked for a pinned
unit system — so it does not block advancing; it is correctly scoped as
an open question for whichever later round first needs it resolved, now
named in the code so it can't be missed. The suite is green, both targets
build clean, and the working tree is clean. This round's goals have been
met to a sufficient standard.

**What I would have done with another 30 minutes.** Sketched what a
concrete unit system for `density` (and, transitively, mass) would look
like against round 3's likely mass-conservation check, to hand the round-3
planner a head start rather than just a flagged question — but that edges
into round 3's own content and was judged out of this round's scope to
pre-empt.
