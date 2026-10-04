#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
cargo build --release --locked
app="dist/Cinderwake.app"
mkdir -p "$app/Contents/MacOS"
cp target/release/cinderwake "$app/Contents/MacOS/Cinderwake"
mkdir -p "$app/Contents/Resources/licenses"
cp LICENSE assets/FONT-LICENSE.txt "$app/Contents/Resources/licenses/"
cat > "$app/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>Cinderwake</string>
<key>CFBundleDisplayName</key><string>Cinderwake</string>
<key>CFBundleExecutable</key><string>Cinderwake</string>
<key>CFBundleIdentifier</key><string>games.cinderwake.desktop</string>
<key>CFBundleVersion</key><string>1</string>
<key>CFBundleShortVersionString</key><string>0.1.0</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
codesign --force --sign - "$app"
