#!/usr/bin/env bash
# Builds the `viewer` crate for wasm32 and generates the JS glue with
# wasm-bindgen-cli, so www/index.html can load it.
#
# This is the exact command sequence proved out in
# cycle-log/tranche-0/m0.1/round-01.md. CI (M0.2) should call this script
# rather than re-deriving the steps.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

# --lib only: M0.3 added a native-only binary (src/bin/native_viewer.rs,
# behind a `cfg(not(target_arch = "wasm32"))` dependency) that doesn't build
# for this target and doesn't need to — the wasm-bindgen step below only
# ever reads the library's cdylib output.
cargo build --release --lib --target wasm32-unknown-unknown

# wasm-bindgen-cli's version must match the `wasm-bindgen` crate version
# resolved in Cargo.lock, or the generated glue will refuse to load at
# runtime. Install once with:
#   cargo install wasm-bindgen-cli --version <version from Cargo.lock>
WASM_BINDGEN="${WASM_BINDGEN:-$HOME/.cargo/bin/wasm-bindgen}"

"$WASM_BINDGEN" --target web --out-dir www/pkg \
  target/wasm32-unknown-unknown/release/viewer.wasm

echo "Built www/pkg/. Serve www/ with any static file server, e.g.:"
echo "  python3 -m http.server -d www 8000"
