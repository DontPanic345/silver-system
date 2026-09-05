# silver-system

Building a small world that is part of a much larger universe — emergent physical
behaviour from simple interacting rules (fluids, materials, temperature, pressure,
reactions), in the spirit of Oxygen Not Included and Noita.

The language is Rust. The Rust code below was scaffolded under the
`night-shift` cycle experiment (now shelved, see below) and now carries a real
gravity/density physics step (`src/grid.rs`) built directly on that substrate
— see [`JOURNAL.md`](JOURNAL.md) for the dated, narrative thread connecting
every pivot this repo has made (including `night-shift`'s stated goal and why
it ended), [`night-shift/CLOSEOUT.md`](night-shift/CLOSEOUT.md) for its full
retrospective, [`NORTH_STARS.md`](NORTH_STARS.md) for the aspirational
statements this and every experiment has served, and
[`PRINCIPLES.md`](PRINCIPLES.md) for the aphorisms distilled along the way.

## The physics: gravity + density, generic over materials

`src/grid.rs`'s per-cell step is a single, generic movement rule driven
entirely by `Material::density`/`Phase` data (`src/material.rs`) — never a
per-material `if` chain: a denser cell may swap into a neighbour's position
if that neighbour is strictly less dense and not `Phase::Solid`, checked in
priority order (straight down, then diagonal-down, then — `Phase::Liquid`
only — sideways, gated to settle rather than slosh forever between two open
columns). `Phase::Granular` (e.g. sand) falls and piles; `Phase::Liquid`
(water) additionally flows to level itself; `Phase::Solid` (stone) never
moves and blocks anything from moving into it. Every move is a swap of two
cells' contents, never a creation or deletion, so per-material cell counts
are exactly conserved by construction, for any number of steps — see
`src/grid.rs`'s and `src/measure.rs`'s own conservation tests.

Watch it live: build the wasm module (below) and open `www/physics.html` —
`scenario::physics_demo()`, a sealed stone container with a flat resting
water pool and several sand grains suspended in the open air above, stepped
forward by `step_and_paint_physics_demo` on the page's own real-time timer
loop. Grains fall, then sink through the water (since sand is denser, displacing
the water upward as they go), and the pool re-levels around the
disturbance. Headless-verified two ways: as a
`cargo test --lib` scenario (`src/scenario.rs`'s
`physics_demo_settles_every_suspended_grain_after_enough_steps`) and as a
real-browser e2e check (`tests/e2e/physics_demo.test.mjs`) that reads actual
canvas pixels after real wall-clock time passes — not a screenshot eyeballed
once.

## Live deploy — the path actually in use

Every push to `main` builds the `viewer` crate to wasm and publishes `www/` to
GitHub Pages via `.github/workflows/deploy-pages.yml`:

**https://dontpanic345.github.io/silver-system/**

This is the path that's built and watched going forward. The native fallback
below exists and is proven, but isn't maintained day to day — per M0.3's own
rule, don't quietly maintain both once one is proven.

## Building and running the `viewer` crate

`viewer` (M0.1, the toolchain proving ground) compiles to wasm32, loads in a
browser canvas via `wasm-bindgen`/`web-sys`, and is verified headlessly with
Playwright rather than by eye (see `night-shift/cycle-log/tranche-0/m0.1/`).

Rust unit tests (native, no wasm/browser involved):

```sh
cargo test
```

### Test tagging / fast path (established M1.1 round 1)

Three speeds of test exist in this repo, and the commands below are the
convention every round from M1.1 round 1 onward is expected to keep:

- **Fast — in-crate unit and scenario tests** (`src/**/*.rs`'s own
  `#[cfg(test)] mod tests`, e.g. `src/math.rs`, `src/timestep.rs`,
  `src/grid.rs`, `src/material.rs`, `src/lib.rs`). No subprocess, no
  filesystem, no browser. This is the command to run while iterating:

  ```sh
  cargo test --lib
  ```

- **Medium — the same, plus the native-binary integration test**
  (`tests/native_fallback.rs`, which builds and runs `native_viewer` as a
  real subprocess and decodes real PNG bytes back). Still no browser, but
  slower than `--lib` alone:

  ```sh
  cargo test
  ```

- **Slow — the Playwright e2e path** (`tests/e2e/canvas_rectangle.test.mjs`,
  `tests/e2e/scenario_canvas.test.mjs`, `tests/e2e/physics_demo.test.mjs`), a
  real headless-Chromium run each. Never part of `cargo test` — run
  explicitly, and only after building the wasm module (see below):

  ```sh
  NODE_PATH=/usr/local/lib/node_modules node tests/e2e/canvas_rectangle.test.mjs
  NODE_PATH=/usr/local/lib/node_modules node tests/e2e/scenario_canvas.test.mjs
  NODE_PATH=/usr/local/lib/node_modules node tests/e2e/physics_demo.test.mjs
  ```

One `#[ignore]`-tagged test exists (`src/grid.rs`'s `reference_grid_step_timing`,
a timing measurement rather than a correctness scenario — see its own doc
comment); run it explicitly with `cargo test --lib -- --ignored`. If a future
round adds another genuinely slow `#[test]`, tag it `#[ignore]` and document
its companion command right next to this section rather than letting it
silently join the default `--lib` run.

Build the wasm module and JS glue (requires `wasm-bindgen-cli` installed at a
version matching the `wasm-bindgen` crate in `Cargo.lock` — see
`scripts/build-wasm.sh` for the install command):

```sh
bash scripts/build-wasm.sh
```

This produces `www/pkg/viewer.js` + `www/pkg/viewer_bg.wasm`. Serve `www/`
with any static file server and open it in a browser — `index.html` for the
original toolchain-proving rectangle, `scenario.html` for a static painting
of `stone_and_water_pool()`, or **`physics.html` for the live gravity/
density physics demo** (see above):

```sh
python3 -m http.server -d www 8000
```

Run the headless end-to-end tests (each drives a real headless Chromium via
Playwright and reads real canvas pixel data rather than a screenshot —
requires `www/pkg/` to already be built, see above):

```sh
NODE_PATH=/usr/local/lib/node_modules node tests/e2e/canvas_rectangle.test.mjs
NODE_PATH=/usr/local/lib/node_modules node tests/e2e/scenario_canvas.test.mjs
NODE_PATH=/usr/local/lib/node_modules node tests/e2e/physics_demo.test.mjs
```

## The fallback (M0.3, not in current use)

If wasm-in-the-browser ever stops working, a native binary is the proven plan
B: it renders the same rectangle-per-tick logic to real PNG files instead of
a canvas, sharing the geometry/colour constants with the wasm path (one
source of truth, `viewer::render_frame`).

```sh
cargo run --bin native_viewer -- /tmp/native-fallback-out
```

Writes `tick-0.png`, `tick-1.png`, `tick-2.png`, plus `scenario.png`
(`stone_and_water_pool()`, static) and `physics-demo-tick-{0,60,150,300}.png`
(`scenario::physics_demo()` stepped forward under real physics, four
snapshots of the same run) — the physics sequence is a visual sanity check,
not itself a headless assertion (that's `src/scenario.rs`'s own test); the
rectangle path is what's verified headlessly (real pixel bytes read back
from the PNGs, not eyeballed) by `tests/native_fallback.rs`, which runs as
part of `cargo test`.

## Shelved experiments

Kept for reference, not extended.

- [`terrarium/`](terrarium/) — a dependency-free browser falling-sand sim that grew
  a sealed glass jar and a closed water cycle. A successful test.
- [`stable-fluids/`](stable-fluids/) — a browser Stam stable-fluids sim with
  conservative advection, temperature and buoyancy, built test-first over seven
  rounds. Halted on a checkerboard mode in the colocated pressure projection. Its
  retrospective is why the `night-shift` cycle skills looked the way they did.
- [`night-shift/`](night-shift/) — the `tranche → milestone → round → phase` cycle
  system that built the Rust code above. Proved out its own ideas (cold review
  catches what a continuous pass misses, planning that folds forward beats
  memoryless churn) but was found to be spending a large share of its own budget
  on ceremony rather than the work. See `night-shift/CLOSEOUT.md`.
