#!/usr/bin/env bash
# Builds the browser version into dist/web. Serve that directory over HTTP, for
# example: python3 -m http.server -d dist/web 8080
# Extra arguments go to cargo build (for example -j 8).
set -euo pipefail
cd "$(dirname "$0")/.."

rustup target add wasm32-unknown-unknown >/dev/null
cargo build --release --locked --target wasm32-unknown-unknown "$@"

# Use the JS loader shipped with the exact macroquad version in Cargo.lock.
macroquad_dir=$(cargo metadata --locked --format-version 1 | python3 -c '
import json, os, sys
meta = json.load(sys.stdin)
print(next(os.path.dirname(p["manifest_path"]) for p in meta["packages"] if p["name"] == "macroquad"))')

out=dist/web
rm -rf "$out"
mkdir -p "$out"
# The game is one ~52 MB file, almost all embedded art. It ships as parts under
# GitHub's recommended 50 MB, named by content so a cached part from an older
# build is never mixed with a newer one; web/index.html reads game-files.json
# (fetched past the cache) and joins them.
python3 - target/wasm32-unknown-unknown/release/cinderwake.wasm "$out" <<'PY'
import hashlib, json, math, sys
wasm, out = sys.argv[1], sys.argv[2]
data = open(wasm, "rb").read()
digest = hashlib.sha256(data).hexdigest()[:12]
count = math.ceil(len(data) / (30 * 1024 * 1024))
size = math.ceil(len(data) / count)
parts = []
for i in range(count):
    name = f"cinderwake-{digest}-{i + 1}.bin"
    open(f"{out}/{name}", "wb").write(data[i * size:(i + 1) * size])
    parts.append(name)
json.dump({"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest(), "parts": parts},
          open(f"{out}/game-files.json", "w"), indent=1)
PY
cp web/index.html web/favicon.png web/cinderwake-storage.js web/cinderwake-pad.js "$out/"
cp "$macroquad_dir/js/mq_js_bundle.js" "$out/"
cp "$macroquad_dir/LICENSE-MIT" "$out/MACROQUAD-LICENSE-MIT.txt"
cp LICENSE "$out/LICENSE.txt"
cp assets/FONT-LICENSE.txt "$out/"
echo "Built $out ($(du -sh "$out" | cut -f1)). Serve it with: python3 -m http.server -d $out 8080"
