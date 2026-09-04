# Round 1 — Numeric type and Vec2

**Milestone:** M0.4 — Mathematical foundations (`cycle-log/tranche-0/m0.4/plan.md`)

**Milestone intent:** the small, boring substrate every later tranche reaches
for — vector/grid primitives, a numeric type decision, a fixed-timestep
harness — got right once, here, rather than reinvented per-tranche.

**Round goals:**

1. Decide the scalar numeric type for world-space/physics quantities (`f32`
   vs `f64`, or fixed-point) and write the reasoning down as a doc comment
   where the type is defined — not defaulted into silently.
2. Introduce a `Vec2` type (using that scalar) with the basic arithmetic
   later tranches will reach for immediately: add, sub, scale (scalar
   multiply), dot product.
3. Pin the coordinate convention for this `Vec2` explicitly, as a doc
   comment stating which axis is "up" (math/physics convention: y grows
   upward) — distinct from, and stated in relation to, the canvas convention
   already pinned in `src/lib.rs` (y grows downward) — and back it with a
   test that would fail if the convention were silently flipped.

**Push on vs. patch back:** pushing on — no prior M0.4 rounds exist.

**Refactor scope/focus:** round scope (the new module only); focus on
whether the doc comment and the test actually agree on the convention, and
whether the numeric-type reasoning is genuine (not a rationalization for
whatever was fastest to type).

---

## Round 1 — Red — 2026-09-05T03:15:22+12:00 → 2026-09-05T03:35:00+12:00 (~20 min)

**What I did.**

Read `src/lib.rs` in full to learn the existing canvas coordinate convention
(origin top-left, `+y` down) before writing anything, per the round brief.
Created `src/math.rs` — a new module, wired in via `mod math;` in
`src/lib.rs` (a one-line addition; no other line of `lib.rs` touched) — with:

- `pub type Scalar = f32;`, with a doc comment giving the actual reasoning
  (GPU-native float per `CLAUDE.md`'s stated GPU direction; determinism is
  architecture-contingent and conservation only needs a stated tolerance, so
  `f64`'s extra headroom isn't spending anything the current plan needs;
  fixed-point's determinism trade isn't asked for by any current target;
  `f32` halves memory traffic once the universe is "sufficiently full").
- `pub struct Vec2 { pub x: Scalar, pub y: Scalar }`, `#[derive(Debug, Clone,
  Copy, PartialEq)]`, with a doc comment pinning the convention: `+y` is up
  (math/physics), explicitly stated as the opposite of `src/lib.rs`'s `+y`-down
  canvas convention, with the boundary-crossing obligation ("flip explicitly")
  spelled out.
- Methods `new` (implemented for real — plain field construction, no
  decision, same reasoning `lib.rs` applies to its own getters), `add`,
  `sub`, `scale`, `dot` — all four left as `unimplemented!("<what goes
  here>")` stubs for Green.
- `pub const UP: Vec2 = Vec2 { x: 0.0, y: 1.0 };` — implemented for real (a
  constant declaration, not an algorithm) specifically so the convention has
  something a test can assert on, not just a doc comment.
- Ten tests in `mod tests`: one convention-pin test needing no stub (so it
  is green from Red), one scenario combining the pin with `add`, six
  arithmetic scenarios (two each for add/sub, one for scale, two for dot),
  and two disposable property/unit tests.

Ran `cargo test --lib`, confirmed the 8 new tests fail on the named
`unimplemented!()` panics and the convention-pin test passes unstubbed;
confirmed all 5 pre-existing `tests::*` tests in `lib.rs` are still green.
Ran `cargo build --lib` and `cargo build --bin native_viewer`: both compile
clean apart from expected `dead_code`/`unused` warnings on the new stubs and
`UP` (nothing outside the test module calls them yet — expected until
Green's callers, or a later round, use them for real).

**Successes.**

The convention pin ended up testable, not just documented: `UP.y > 0.0`
would fail if a future change silently redefined "up" as `-y` to match the
canvas instead of math/physics, and `moving_up_by_the_pinned_up_vector_increases_y`
exercises that same pin together with `add`'s real arithmetic once Green
fills it in. `src/lib.rs` needed exactly one line changed (`mod math;`) —
its own hello-world logic and tests are untouched, confirmed by the 5/5
green re-run.

**What was difficult, and where the time went.**

Most of the time went into the numeric-type doc comment — making sure the
reasoning was genuinely about this project's stated direction (GPU,
architecture-contingent determinism, tolerance-based conservation from
`CLAUDE.md`) rather than a generic "f32 is faster" rationalization the
Refactor focus explicitly warns against. Deciding how to make the coordinate
convention *test-backed* rather than doc-only took a second pass: the first
draft only had the `UP.y > 0.0` assertion, which is trivially true by
construction; adding the `add`-based scenario on top of it means the pin
also exercises real arithmetic, not just a static constant, once Green
implements it.

**Compromises I made.**

`Vec2` exposes only methods (`add`, `sub`, `scale`, `dot`), not
`std::ops::Add`/`Sub`/`Mul` operator overloads. The round's goals list the
four operations by name, not by calling convention, and adding operator
trait impls on top would be additional shape decisions past what this round
asked for — left as a call for a later round if ergonomics demand it.
`new`'s body and `UP`'s value are implemented for real rather than stubbed;
I judged both to be plain data/construction with no decision to make, on the
same basis `lib.rs` already uses for its own getters — flagging this
explicitly in case Refactor reads it differently.

**Gaps and flags.**

- `Vec2` is not yet used by anything in `lib.rs` or the native binary — that
  is out of this round's scope (goal 2/3 only ask for the type to exist),
  but a later M0.4 round (the fixed-timestep harness, or a grid primitive)
  is presumably where it gets a first real caller.
- `dead_code`/`unused` warnings on `add`/`sub`/`scale`/`dot`/`UP` in a
  non-test build are expected right now (only the test module calls them)
  and should resolve naturally once Green's implementations exist and/or a
  later round adds a real caller — flagging so Refactor doesn't mistake it
  for an oversight.
- I did not add operator overloads (see Compromises) — flagging in case the
  milestone plan expected them implicitly under "the basic arithmetic later
  tranches will reach for immediately."

**General comments.**

Did not restructure the round's goal slice — the plan's three goals map
directly onto one module with one coherent test file, small enough for one
round and already sequenced correctly (type decision, then the type built on
it, then the convention pinned on top of that). No exit ramp needed.

**Test file:** `src/math.rs` (`#[cfg(test)] mod tests`, same file as the
skeleton — matches this crate's existing convention of colocated tests, per
`src/lib.rs`).

**Currently-failing tests (8), each a named-stub panic, not a compile error
or typo:**

| Test | Reason |
|---|---|
| `math::tests::add_combines_two_displacements_into_one` | `add` is `unimplemented!()` |
| `math::tests::sub_gives_the_displacement_that_add_can_undo` | `sub` is `unimplemented!()` |
| `math::tests::scale_stretches_both_components_uniformly` | `scale` is `unimplemented!()` |
| `math::tests::scale_by_zero_collapses_to_origin` | `scale` is `unimplemented!()` |
| `math::tests::dot_is_zero_for_perpendicular_vectors` | `dot` is `unimplemented!()` |
| `math::tests::dot_is_positive_for_aligned_vectors` | `dot` is `unimplemented!()` |
| `math::tests::dot_is_commutative` | `dot` is `unimplemented!()` |
| `math::tests::moving_up_by_the_pinned_up_vector_increases_y` | uses `add`, `unimplemented!()` |

**Currently passing (already green, no stub needed):**
`math::tests::up_convention_pins_math_physics_y_up_not_canvas_y_down`.

**Skeleton Green must fill in (exact signatures, all in `src/math.rs`):**

- `pub type Scalar = f32;` — decided, not a stub.
- `pub struct Vec2 { pub x: Scalar, pub y: Scalar }` — decided (derives
  `Debug, Clone, Copy, PartialEq`), not a stub.
- `impl Vec2 { pub fn new(x: Scalar, y: Scalar) -> Self }` — implemented for
  real, not a stub.
- `impl Vec2 { pub fn add(self, other: Vec2) -> Vec2 }` — stub
  (`unimplemented!("component-wise addition of self and other")`).
- `impl Vec2 { pub fn sub(self, other: Vec2) -> Vec2 }` — stub
  (`unimplemented!("component-wise subtraction of other from self")`).
- `impl Vec2 { pub fn scale(self, s: Scalar) -> Vec2 }` — stub
  (`unimplemented!("component-wise multiplication of self by s")`).
- `impl Vec2 { pub fn dot(self, other: Vec2) -> Scalar }` — stub
  (`unimplemented!("dot product of self and other")`).
- `pub const UP: Vec2 = Vec2 { x: 0.0, y: 1.0 };` — decided, not a stub.

**Commands:**

- This round's tests only: `cargo test --lib math::`
- Full suite: `cargo test --lib` (native/lib unit tests); the crate also has
  `tests/e2e/canvas_rectangle.test.mjs` (Playwright, unaffected by this
  round — not run here since nothing this round touches the canvas/JS path).

**Did I restructure the slice?** No. The plan's three goals (numeric type,
`Vec2` arithmetic, convention pin) mapped directly onto one module and one
round without needing to split, reorder, or merge anything.

**Durable scenarios (guard the round's goals/intent, keep across
implementation changes):** `up_convention_pins_math_physics_y_up_not_canvas_y_down`,
`moving_up_by_the_pinned_up_vector_increases_y`,
`add_combines_two_displacements_into_one`,
`sub_gives_the_displacement_that_add_can_undo`,
`scale_stretches_both_components_uniformly`,
`dot_is_zero_for_perpendicular_vectors`,
`dot_is_positive_for_aligned_vectors`.

**Disposable unit tests (may be deleted if the implementation's shape
changes):** `scale_by_zero_collapses_to_origin`, `dot_is_commutative`.

**Confirmation the rest of the suite is green:** yes — `cargo test --lib`
shows all 5 pre-existing `tests::*` tests in `src/lib.rs`
(`rectangle_fits_within_canvas`, `getters_expose_the_same_constants_js_reads`,
`render_frame_paints_rect_in_tick_colour`,
`color_for_tick_alternates_by_parity`, plus the crate's implicit doctest
count of zero) passing, unaffected by this round's addition.

---

## Round 1 — Green — 2026-09-05T03:19:01+12:00 → 2026-09-05T03:22:00+12:00 (~3 min)

**What I did.**

Filled in the four stub method bodies in `src/math.rs` — `add`, `sub`,
`scale`, `dot` — with straightforward component-wise arithmetic, exactly as
their doc comments and the intent describe. No other line of the file
changed: `Scalar`, `Vec2`'s struct/derives, `new`, `UP`, and all doc comments
were left exactly as Red wrote them.

- `add`: `Vec2 { x: self.x + other.x, y: self.y + other.y }`
- `sub`: `Vec2 { x: self.x - other.x, y: self.y - other.y }`
- `scale`: `Vec2 { x: self.x * s, y: self.y * s }`
- `dot`: `self.x * other.x + self.y * other.y`

None of Red's four signatures needed changing — took them as given.

**Commands run and results.**

- `cargo test --lib math::` → `test result: ok. 9 passed; 0 failed; 0
  ignored; 0 measured; 4 filtered out`. All 8 previously-failing tests now
  pass, plus the one that was already green
  (`up_convention_pins_math_physics_y_up_not_canvas_y_down`).
- `cargo test --lib` (full suite) → `test result: ok. 13 passed; 0 failed; 0
  ignored; 0 measured; 0 filtered out` — the 9 `math::tests::*` plus 4
  `tests::*` from `src/lib.rs` (`rectangle_fits_within_canvas`,
  `getters_expose_the_same_constants_js_reads`,
  `render_frame_paints_rect_in_tick_colour`,
  `color_for_tick_alternates_by_parity`). Note: Red's report said "5
  pre-existing `tests::*` tests" in `lib.rs`; the actual count, confirmed by
  `grep -n '#\[test\]' src/lib.rs` and by this run, is 4. Flagging the
  discrepancy for the record — not a regression, just a miscount in the Red
  report, and `src/lib.rs` itself was not touched by this phase.

**Files touched.** `src/math.rs` only (the four method bodies).

**Signature changes from Red's skeleton.** None.

**Numbers proposed.** None — no thresholds or constants needed changing;
this was pure arithmetic filling in already-decided signatures.

**What I noticed but left for Refactor.** Red's own flagged compromises
stand as Red left them (no operator-overload traits, `Vec2` not yet used by
any caller outside its own tests, expected `dead_code` warnings should now
be gone since the methods are exercised by tests) — none of that is in
Green's lane to change. Self-diff (`git diff -- src/math.rs`) confirms the
change is exactly the four method bodies: no dead compute, no stray doc
drift, no leftover experiment code.

**Self-diff.** Read the full diff before finishing (`git diff -- src/math.rs`):
four hunks, one per method, each replacing a `let _ = ...; unimplemented!(...)`
stub with the method's real body. Nothing else changed.

**Committed.** `git add src/math.rs` followed by one commit for this phase's
work (see git log).

