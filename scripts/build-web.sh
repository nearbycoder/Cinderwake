#!/usr/bin/env bash
# Builds the browser version into dist/web. Serve that directory over HTTP, for
# example: python3 -m http.server -d dist/web 8080
set -euo pipefail
cd "$(dirname "$0")/.."

rustup target add wasm32-unknown-unknown >/dev/null
cargo build --release --locked --target wasm32-unknown-unknown

# Use the JS loader shipped with the exact macroquad version in Cargo.lock.
macroquad_dir=$(cargo metadata --locked --format-version 1 | python3 -c '
import json, os, sys
meta = json.load(sys.stdin)
print(next(os.path.dirname(p["manifest_path"]) for p in meta["packages"] if p["name"] == "macroquad"))')

out=dist/web
rm -rf "$out"
mkdir -p "$out"
cp target/wasm32-unknown-unknown/release/cinderwake.wasm "$out/"
cp web/index.html web/cinderwake-storage.js "$out/"
cp "$macroquad_dir/js/mq_js_bundle.js" "$out/"
cp "$macroquad_dir/LICENSE-MIT" "$out/MACROQUAD-LICENSE-MIT.txt"
cp LICENSE "$out/LICENSE.txt"
cp assets/FONT-LICENSE.txt "$out/"
echo "Built $out ($(du -sh "$out" | cut -f1)). Serve it with: python3 -m http.server -d $out 8080"
