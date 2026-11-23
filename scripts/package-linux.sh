#!/bin/sh
# Packs a Linux build into dist/Frostfire-linux-<arch>.tar.gz: a Frostfire-linux-<arch>/ folder
# with the frostfire binary, its .desktop entry, a 256 px icon and the license.
#
#   scripts/package-linux.sh x64 target/release/frostfire
#
# To install for one user, from the unpacked folder:
#   install -Dm755 frostfire ~/.local/bin/frostfire
#   install -Dm644 io.github.alycoy.frostfire.desktop -t ~/.local/share/applications
#   install -Dm644 io.github.alycoy.frostfire.png -t ~/.local/share/icons/hicolor/256x256/apps
set -eu
cd "$(dirname "$0")/.."
arch=${1:?usage: scripts/package-linux.sh <x64|arm64> <path to frostfire>}
bin=${2:?usage: scripts/package-linux.sh <x64|arm64> <path to frostfire>}
name=Frostfire-linux-$arch
stage=dist/$name
rm -rf "$stage" "dist/$name.tar.gz"
mkdir -p "$stage"
install -m 755 "$bin" "$stage/frostfire"
install -m 644 assets/io.github.alycoy.frostfire.desktop "$stage/"
install -m 644 assets/icon-256.png "$stage/io.github.alycoy.frostfire.png"
install -m 644 LICENSE "$stage/LICENSE"
tar -C dist --owner=0 --group=0 -czf "dist/$name.tar.gz" "$name"
rm -rf "$stage"
ls -l "dist/$name.tar.gz"
