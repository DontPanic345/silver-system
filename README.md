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

A warm spring under a pool, a glass lid held cold, a garden on a watered
bed, and four gnomes on the meadow between them. The pool evaporates because
warm water does, the humid air meets the cold lid, and dew falls back as
drops; the gnomes forage juniper, plant cuttings, and pay Gin for the magic
fountain.

The jar also has a **day**. Sunlight comes in through the lid, warms what it
lands on and goes out again at dusk, and that day is what the garden runs on:
in the light the bushes take carbon dioxide out of the air and water out of
their bed and put on weight, and in the dark they spend some of it back. The
gnomes eat the bushes and breathe the carbon out again, so the jar's carbon
goes round rather than accumulating — it is constant, across plants, air and
gnome bellies, to a part in a billion. Oxygen is a real species too, so a
sealed room can be breathed flat, and the garden is what stops it.

And since night 7 things **die and rot**. A bush drops dead leaves, a bush
that freezes dies outright, and what is left in either case is `litter`,
which falls and piles like sand. Litter rots: it breathes oxygen, puts its
carbon back into the air as carbon dioxide, and grows `fungus` on itself —
pale violet mould that then feeds on the litter around it, spreads through a
heap in the dark, and spends itself away when there is nothing left to eat.
So carbon now gets back into the air without passing through anything's
lungs, and the jar's carbon constant covers five places instead of three:
standing plant, dead leaf, mould, air, and gnome. The terrarium is seeded
with a compost heap at the garden gate, the way you would plant a real one
with a handful of leaf mould, and you can watch it go down.

All of it is emergent: there is no script, only conduction, buoyancy, one
vapour-pressure curve per liquid, a table of declared mass proportions, and a
Gin budget. Nothing in the code names juniper, litter or mould; the rot cycle
is five rows of the material table (`src/material.rs`) run by the same one
pass that already ran photosynthesis (`src/life.rs`).

And since night 6 you can **reach into it**. Point at any cell and the page
tells you what it is, how hot, how heavy, what gases are mixed into it and
how much daylight reaches it. Pick up the dig tool and click, and a gnome
walks over and digs — after it has breathed and eaten, not before. What it
digs goes into its *hands*, as real grams at the temperature it came out at,
and stays out of the world until somebody puts it down again: build is
"put down what you are carrying". There is no resource counter and no
material palette, which means **you cannot build what you have not dug**, and
the wall you get is the hole you made. Warming and chilling a cell are the
magic, and they cost the colony Gin like every other spell. See
`src/order.rs`.

Since night 8 the gnomes **find their own way there**. A gnome floods the
world with routes over the moves it can actually make — fall before walk,
climb one course, chimney straight up out of a pit, never onto the pool,
never into anything near lethal — and walks the route, so an order at the far
end of the jar behind the garden is an order that gets done (`src/path.rs`).
What it cannot reach it says so about: an order nobody can walk to is drawn
dashed and dim, counted on the page, and eventually given up on. A colony
shut in by the garden growing across its path lifts the hedge up a course and
keeps its own way open — a swap, so the bush loses nothing — and only cuts one
down when there is nowhere to lift it to, at which point what is left is what
the table already says a shed bush leaves. None of that names a material:
"crop" is *anything something in the table grows*, and "food" is a column
(`Material::nutrition`).

And since night 9 you can ask for a **material** rather than a cell. Pick
*Fetch*, choose something out of the list — which is every non-gas row of the
material table, served from the table rather than written into the page —
mark where you want it, and the colony finds the nearest one it can actually
walk to and carries it there. Nobody says where it comes from. Tick *keep it
stocked* and the order does not end when it is done: it waits and asks
again, which is what running a machine needs (`Job::Supply`, `Orders::repeat`
in `src/order.rs`). A gnome will not lift the cell it is standing on, or one
holding itself up over water — what that leaves is not a gap but a way into
the drink.

Night 9 also found why the colony had been slowly dying in every long run,
and it was five separate things, none of them the pathing everyone had
blamed: the jar's base was the one face of its shell not in contact with the
room, so the spring was heating the whole stone floor and the meadow settled
above a gnome's lethal limit; a gnome standing in leaf litter never breathed
out, so it was never hungry, so it never foraged; and a gnome's idea of what
is too hot took no account of *how much heat a cell can actually deliver*.
A sauna is 370 K and a bath at 370 K would kill you, so a cell's felt
temperature is now its real one pulled toward body heat by its heat capacity
(`gnome::felt_temperature`) — which is also what will eventually let a colony
stand beside its own still. Then: the garden's water bed was a trough deep
enough to drown in, so a gnome that fell through a gap in the hedge gasped
its flask empty and cut its way out through the crop; and foraging reached
only four neighbours while a garden grows *upward*, so the colony kept the
bottom row picked bare and could not touch the rest. With all five fixed the
jar is solvent over 80 000 steps for the first time — Gin between 272 and
367, every belly fed, and the garden growing faster than it is eaten.

```sh
# Headless: JSON snapshots (and ASCII maps on stderr) — no browser needed.
cargo run --release --bin terrarium -- --steps 8000 --every 2000 --map --temps

# The same jar with two player orders written on it before it starts: dig
# the meadow at (10, 3), and build the spoil back down at (12, 5).
cargo run --release --bin terrarium -- --steps 900 --every 300 --map \
    --dig 10,3 --build 12,5

# Why is the colony doing that? One line per gnome per step on stderr:
# where it is, what it just did, how much of the world it can currently
# reach, its Gin and its belly.
cargo run --release --bin terrarium -- --steps 400 --every 400 --trace

# In a browser, with a live conservation read-out:
bash scripts/build-wasm.sh
python3 -m http.server -d www 8000   # then open /terrarium.html
```

There are two more live pages, each with a headless runner of its own
(`--bin chamber`, `--bin still`; `--map` draws materials, `--temps` a
temperature map).

`/gases.html`: a room with a ten-atmosphere bottle of air behind one small
hole and a slab of CO₂ released at the ceiling. The bottle bleeds down until
the whole room is at one pressure, and the CO₂ sinks — it is heavier — and
then mixes up into the air rather than lying on the floor as a layer. Gas
cells hold mixtures; nothing in the code names CO₂ or says heavy gases sink.
See `src/gas.rs`.

`/still.html`: juniper standing in water in a pot held at 366 K, a wall
with a gap in it, and a cold receiver on the other side. Gin comes out of
the far end, and two thirsty gnomes drink it. Nothing in the code is a
brewing mechanic: mashing is one row of a reaction table, and distilling is
evaporation along two vapour-pressure curves read off the table's boiling
points — spirit's (351.5 K) and water's (373.15 K). Some water comes over
too, as it does from a real pot still. See `src/chemistry.rs`,
`src/vapour.rs` and `src/still.rs`.

Since night 9 the mash row turns the *botanical* into wash and leaves the
water it steeped in alone. It used to turn both cells into wash, which made
a still a machine for turning water into gin — measured at two thousand Gin
out of a pool, in a world whose whole premise is that nothing comes from
nowhere. Wash is the extract now, a gram of bush makes a gram of it, and gin
is worth 1000 Gin to the gram against juniper's 600 because ethanol carries
about 1.75 times the energy of dry plant matter. So a still concentrates
what a garden grew and cannot make more of it.

```sh
cargo run --release --bin still -- --steps 4000 --every 500 --map
cargo run --release --bin chamber -- --steps 4000 --every 500
```

### What it is actually claiming

The interesting property is not that things fall convincingly; it is that
**mass and energy are conserved by construction, and everything that isn't
conserved is written down.** Movement is a swap of whole cells, conduction
is a clamped symmetric pairwise transfer, a phase change is an algebraic
rewrite that holds a cell's energy fixed across the material switch, and a
reaction is the same rewrite over a touching pair. None of those can gain or
lose a gram or a joule.

Gas cells are the exception to "one material per cell": a gas cell holds a
*mixture*, grams of each species plus any liquid mist, so gases share cells
and mix, and every transfer between two gas cells moves mass with its own
energy and re-solves the receiving cell's temperature from its books. A
partly-empty cell of anything loose — a liquid, or something granular like
sand or leaf litter — pours into its neighbours (`physics::coalesce_loose`)
and hands the space back to the atmosphere. All of it is
built from transfers that already conserve.

A living cell is the same story with unequal masses: a `Metabolism` declares
how many grams of what a process takes in and gives back (`6 CO₂ + 6 H₂O →
C₆H₁₂O₆ + 6 O₂` is 1.47 g and 0.6 g in, 1.07 g out, per gram of plant), the
table asserts the two sides balance, and every parcel carries its own
enthalpy with the host settling the difference. So the heat of photosynthesis
is emergent, and the carbon in a sealed jar is a constant rather than an
approximation. Rot is the same machinery run backwards at 40% efficiency —
a gram of dead leaf is 0.4 g of mould plus 0.88 g of carbon dioxide plus
0.36 g of water, which is photosynthesis's own proportions scaled by the
share actually being burned — and the one heat figure it declares is
likewise photosynthesis's, so a compost heap warms by exactly what the sun
put into the leaves and cannot warm by more.

Conservation of *mass* is not conservation of *atoms*, and the difference is
the whole reason the carbon figure is worth quoting. A living process that
produces something solid takes a cell of air to put it in, and if that air is
simply overwritten, the carbon dioxide in it is gone: the books balance and
the jar has quietly eaten its own atmosphere. So products are placed with
`gas::displace`, gases before solids, and nothing is consumed until every
product has somewhere to go.

Three things are allowed to break that, and all of them go through the same
ledger: gnome magic (paid for in Gin), the terrarium's declared hot/cold
boundary and its sunlight (a sealed jar reaches equilibrium and its water
cycle stops — see `src/terrarium.rs`), and a gnome's belly and hands (what it
eats leaves the world and what it breathes out comes back, so a gnome
part-way through digesting a berry is a small non-zero ledger entry — and so
is one walking across the jar with a rock in its arms). So the standing
invariant is not "nothing changes" but

```text
total_mass_now == total_mass_at_start + ledger.mass_conjured
```

and the same for energy. Every long-running test asserts it to `1e-6`
relative, the headless report prints it, and the browser page displays it
live. `residual_mass_g` in that output is the whole argument in one number.

### Where the pieces live

| File | What it holds |
| --- | --- |
| `src/material.rs` | Materials, phase transitions, reactions and metabolisms as data; enthalpy offsets derived, not declared |
| `src/world.rs` | Cells with mass, temperature and latent progress; the magic ledger |
| `src/physics.rs` | Movement, buoyancy, hydrostatic levelling, coalescence of loose matter, conduction, phase change |
| `src/gas.rs` | Gas mixtures: pressure (`P = T·Σ m·R`), bulk flow with momentum, interdiffusion |
| `src/vapour.rs` | Vapour pressure: evaporation below boiling, condensation, dew, mist and rain |
| `src/chemistry.rs` | Reactions between touching cells: mashing, combustion |
| `src/light.rs` | How far the sky reaches into the world, and the day that drives it |
| `src/life.rs` | Metabolisms: stoichiometric growth, respiration and rot; how something living spreads |
| `src/path.rs` | Where a gnome can get to: the mobility graph and breadth-first routes over it |
| `src/chamber.rs` | The gas demonstration room, its bottle, vent and scrubber; the relief valve |
| `src/gnome.rs` | Gin economy, the ethereal layer, rescue, foraging, drinking, breathing, planting, harvesting, ethereal pipes |
| `src/order.rs` | The glass pane: dig/build/fetch/temper orders a player writes on cells, standing orders, and what a gnome carries |
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
NODE_PATH=/usr/local/lib/node_modules node tests/e2e/terrarium_orders.test.mjs
NODE_PATH=/usr/local/lib/node_modules node tests/e2e/terrarium_routing.test.mjs
NODE_PATH=/usr/local/lib/node_modules node tests/e2e/terrarium_fetch.test.mjs
```

The last five are the ones worth keeping green: each drives a real page in
headless Chromium, lets it run for real wall-clock seconds, and then checks
*both* the numbers the page reports and the actual canvas pixels — the CO₂
layer arriving on the floor, the gin pooling in the receiver, the hole
appearing where a real mouse clicked — so a claim has to survive being
looked at as well as being computed.

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
