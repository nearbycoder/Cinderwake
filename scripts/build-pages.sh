#!/usr/bin/env bash
# Builds the site GitHub Pages serves at https://nearbycoder.github.io/Cinderwake/
# into dist/pages (gitignored): the browser build from scripts/build-web.sh plus
# .nojekyll. Every URL in it is relative, so it works from any subpath. Check it
# with: node scripts/check-pages.mjs <url>. Extra arguments go to cargo build.
set -euo pipefail
cd "$(dirname "$0")/.."

./scripts/build-web.sh "$@"
out=dist/pages
rm -rf "$out"
cp -R dist/web "$out"
touch "$out/.nojekyll"
largest=$(find "$out" -type f -exec du -k {} + | sort -n | tail -1)
echo "Built $out ($(du -sh "$out" | cut -f1); largest file ${largest##*/}, $(( ${largest%%[[:space:]]*} / 1024 )) MB)."
