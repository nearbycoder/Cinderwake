#!/bin/sh
# Builds a portable Linux x86_64 tarball in dist/. It is for local sharing and
# testing, not a signed or published release.
set -eu
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
cargo build --release --locked
name="cinderwake-linux-x86_64"
stage="dist/$name"
rm -rf "$stage" "dist/$name.tar.gz"
mkdir -p "$stage/licenses"
cp target/release/cinderwake "$stage/cinderwake"
cp LICENSE "$stage/licenses/LICENSE.txt"
cp assets/FONT-LICENSE.txt "$stage/licenses/"
glibc=$(objdump -T target/release/cinderwake 2>/dev/null | grep -o 'GLIBC_[0-9.]*' | sort -uV | tail -1 | cut -d_ -f2)
cat > "$stage/README.txt" <<TXT
Cinderwake for Linux (x86_64)

Run ./cinderwake from this folder. All art, audio, fonts, and shaders are
embedded in the executable.

Requirements: glibc ${glibc:-unknown} or newer, OpenGL-capable graphics, an X11 or
XWayland session, ALSA (libasound.so.2), and libudev.so.1 for controllers; both
are usually present. Tested on
CachyOS under Wayland through XWayland with AMD Radeon graphics.

Progress is saved to \$XDG_DATA_HOME/cinderwake/ (usually
~/.local/share/cinderwake/). Press Escape (or Start) in game for controls, O for
options.

Source and documentation: https://github.com/nearbycoder/Cinderwake
Licenses are in licenses/.
TXT
tar -C dist -czf "dist/$name.tar.gz" "$name"
echo "Built dist/$name.tar.gz ($(du -h "dist/$name.tar.gz" | cut -f1))"
