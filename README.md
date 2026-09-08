# silver-system

Building a small world that is part of a much larger universe — emergent physical
behaviour from simple interacting rules (fluids, materials, temperature, pressure,
reactions), in the spirit of Oxygen Not Included and Noita.

The language is Rust. The current experiment is **Gnomes** — a conserving
mass/energy/phase simulation with a colony sim on top of it, where a gnome's
magic is the one accounted-for exception to an otherwise closed world. See
[`NORTH_STARS.md`](NORTH_STARS.md) #4 for the vision and the section below
for how to run it. Some of the Rust here predates it, built under the
`night-shift` cycle experiment (now shelved, see below) and kept as
substrate — see [`JOURNAL.md`](JOURNAL.md) for the dated, narrative thread
connecting every pivot this repo has made (including that experiment's stated
goal and why it ended), [`night-shift/CLOSEOUT.md`](night-shift/CLOSEOUT.md)
for its full retrospective, [`NORTH_STARS.md`](NORTH_STARS.md) for the
aspirational statements this and every experiment has served, and
[`PRINCIPLES.md`](PRINCIPLES.md) for the aphorisms distilled along the way.

## The gnome terrarium — the current experiment

A vent held hot under a lid held cold, water in between, four gnomes living
in the gap. The boiling, the rain, the sand piling and the gnomes' scramble
for Gin are all emergent: there is no script, only conduction, two phase
transitions, a density rule, and a Gin budget.

```sh
# Headless: JSON snapshots (and an ASCII map on stderr) — no browser needed.
cargo run --release --bin terrarium -- --steps 8000 --every 2000 --map

# In a browser, with a live conservation read-out:
bash scripts/build-wasm.sh
python3 -m http.server -d www 8000   # then open /terrarium.html
```

There are two more live pages.

`/gases.html`: a room with a ten-atmosphere bottle of air behind one small
hole and a slab of CO₂ released at the ceiling. The bottle bleeds down to
the room's pressure and the CO₂ falls, spreads and settles into a flat layer
on the floor — with nothing in the code naming CO₂ or saying heavy gases
sink. See `src/gas.rs`.

`/still.html`: juniper standing in water in a pot held at 368 K, a wall with
one gap in it, and a cold receiver on the other side. Gin comes out of the
far end, and two thirsty gnomes drink it. Nothing in the code is a brewing
mechanic: mashing is one row of a reaction table, and distilling is the
ordinary phase change that boils a kettle, applied to a liquid whose boiling
point (351.5 K) is 22 K below water's. Hold the pot between the two figures
and one boils while the other does not. See `src/chemistry.rs` and
`src/still.rs`.

```sh
cargo run --release --bin still -- --steps 4000 --every 500 --map
```

### What it is actually claiming

The interesting property is not that things fall convincingly; it is that
**mass and energy are conserved by construction, and everything that isn't
conserved is written down.** Movement is a swap of whole cells, conduction
is a clamped symmetric pairwise transfer, and a phase change is an algebraic
rewrite that holds a cell's energy fixed across the material switch. None of
those can gain or lose a gram or a joule.

Two things are allowed to break that, and both go through the same ledger:
gnome magic (paid for in Gin) and the terrarium's declared hot/cold boundary
(a sealed jar reaches equilibrium and its water cycle stops — see
`src/terrarium.rs`). So the standing invariant is not "nothing changes" but

```text
total_mass_now == total_mass_at_start + ledger.mass_conjured
```

and the same for energy. Every long-running test asserts it to `1e-6`
relative, the headless report prints it, and the browser page displays it
live. `residual_mass_g` in that output is the whole argument in one number.

### Where the pieces live

| File | What it holds |
| --- | --- |
| `src/material.rs` | Materials, phase transitions and reactions as data; enthalpy offsets derived, not declared |
| `src/world.rs` | Cells with mass, temperature and latent progress; the magic ledger |
| `src/physics.rs` | Movement, buoyancy, hydrostatic levelling, conduction, phase change |
| `src/gas.rs` | Gas pressure (`P = m·R·T`), pressure-driven diffusion and bulk flow |
| `src/chemistry.rs` | Reactions between touching cells: mashing, combustion |
| `src/chamber.rs` | The gas demonstration room, its bottle, vent and scrubber |
| `src/gnome.rs` | Gin economy, the ethereal layer, rescue, foraging, drinking, ethereal pipes |
| `src/terrarium.rs` | The flagship scenario and its declared boundary conditions |
| `src/still.rs` | The brewing scenario: mash tun, lyne arm, condenser, receiver |
| `src/report.rs` | JSON snapshots and an ASCII map, for headless verification |

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
NODE_PATH=/usr/local/lib/node_modules node tests/e2e/terrarium_canvas.test.mjs
NODE_PATH=/usr/local/lib/node_modules node tests/e2e/gases_canvas.test.mjs
NODE_PATH=/usr/local/lib/node_modules node tests/e2e/still_canvas.test.mjs
```

The last three are the ones worth keeping green: each drives a real page in
headless Chromium, lets it run for real wall-clock seconds, and then checks
*both* the numbers the page reports and the actual canvas pixels — the CO₂
layer arriving on the floor, the gin pooling in the receiver — so a claim
has to survive being looked at as well as being computed.

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
