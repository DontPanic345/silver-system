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
Playwright rather than by eye (see `cycle-log/tranche-0/m0.1/`).

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

- **Slow — the Playwright e2e path** (`tests/e2e/canvas_rectangle.test.mjs`),
  a real headless-Chromium run. Never part of `cargo test` — run explicitly,
  and only after building the wasm module (see below):

  ```sh
  NODE_PATH=/usr/local/lib/node_modules node tests/e2e/canvas_rectangle.test.mjs
  ```

No `#[ignore]`-tagged tests exist yet; if a future round adds a genuinely
slow `#[test]` (a long-running headless scenario, say), tag it `#[ignore]`
and document its companion command (`cargo test --lib -- --ignored`) right
next to this section rather than letting it silently join the default
`--lib` run.

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
Playwright, samples real canvas pixel data at three points in time — requires
`www/pkg/` to already be built, see above):

```sh
NODE_PATH=/usr/local/lib/node_modules node tests/e2e/canvas_rectangle.test.mjs
```

## The fallback (M0.3, not in current use)

If wasm-in-the-browser ever stops working, a native binary is the proven plan
B: it renders the same rectangle-per-tick logic to real PNG files instead of
a canvas, sharing the geometry/colour constants with the wasm path (one
source of truth, `viewer::render_frame`).

```sh
cargo run --bin native_viewer -- /tmp/native-fallback-out
```

Writes `tick-0.png`, `tick-1.png`, `tick-2.png`. Verified headlessly (real
pixel bytes read back from the PNGs, not eyeballed) by
`tests/native_fallback.rs`, which runs as part of `cargo test`.

## Shelved experiments

Kept for reference, not extended.

- [`terrarium/`](terrarium/) — a dependency-free browser falling-sand sim that grew
  a sealed glass jar and a closed water cycle. A successful test.
- [`stable-fluids/`](stable-fluids/) — a browser Stam stable-fluids sim with
  conservative advection, temperature and buoyancy, built test-first over seven
  rounds. Halted on a checkerboard mode in the colocated pressure projection. Its
  retrospective is why the current cycle skills look the way they do.
