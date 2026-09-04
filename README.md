# silver-system

Building a small world that is part of a much larger universe — emergent physical
behaviour from simple interacting rules (fluids, materials, temperature, pressure,
reactions), in the spirit of Oxygen Not Included and Noita.

The work plan is [`PLAN.md`](PLAN.md): mathematics and tooling foundations, then
physics, chemistry, biology, and the glass pane. The language is Rust.

## How the work runs

`tranche → milestone → round → phase`. A round is one Red → Green → Refactor pass;
a phase is one agent's turn. The cycle is defined by the skills in
[`.claude/skills/`](.claude/skills/), starting with `cycle-contract`. `cycle-tranche`
is the top of it — it runs indefinitely, chaining from tranche to tranche once
`PLAN.md`'s are done. Round logs live under `cycle-log/`.

## Live deploy

Every push to `main` builds the `viewer` crate to wasm and publishes `www/` to
GitHub Pages via `.github/workflows/deploy-pages.yml`:

**https://dontpanic345.github.io/silver-system/**

## Building and running the `viewer` crate

`viewer` (M0.1, the toolchain proving ground) compiles to wasm32, loads in a
browser canvas via `wasm-bindgen`/`web-sys`, and is verified headlessly with
Playwright rather than by eye (see `cycle-log/tranche-0/m0.1/`).

Rust unit tests (native, no wasm/browser involved):

```sh
cargo test
```

Build the wasm module and JS glue (requires `wasm-bindgen-cli` installed at a
version matching the `wasm-bindgen` crate in `Cargo.lock` — see
`scripts/build-wasm.sh` for the install command):

```sh
bash scripts/build-wasm.sh
```

This produces `www/pkg/viewer.js` + `www/pkg/viewer_bg.wasm`. Serve `www/`
with any static file server and open it in a browser to watch it run, e.g.:

```sh
python3 -m http.server -d www 8000
```

Run the headless end-to-end test (drives a real headless Chromium via
Playwright, samples real canvas pixel data at two points in time — requires
`www/pkg/` to already be built, see above):

```sh
NODE_PATH=/usr/local/lib/node_modules node tests/e2e/canvas_rectangle.test.mjs
```

## Shelved experiments

Kept for reference, not extended.

- [`terrarium/`](terrarium/) — a dependency-free browser falling-sand sim that grew
  a sealed glass jar and a closed water cycle. A successful test.
- [`stable-fluids/`](stable-fluids/) — a browser Stam stable-fluids sim with
  conservative advection, temperature and buoyancy, built test-first over seven
  rounds. Halted on a checkerboard mode in the colocated pressure projection. Its
  retrospective is why the current cycle skills look the way they do.
