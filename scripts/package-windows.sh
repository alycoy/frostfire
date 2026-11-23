#!/bin/sh
# Packs a Windows build into dist/Frostfire-windows-<arch>.zip: Frostfire.exe, LICENSE.txt and
# README.md. CI runs it in Git Bash after `cargo build --release --target <triple>`.
#
#   scripts/package-windows.sh x64 target/x86_64-pc-windows-msvc/release/frostfire.exe
set -eu
cd "$(dirname "$0")/.."
arch=${1:?usage: scripts/package-windows.sh <x64|arm64> <path to frostfire.exe>}
bin=${2:?usage: scripts/package-windows.sh <x64|arm64> <path to frostfire.exe>}
name=Frostfire-windows-$arch
stage=dist/$name
rm -rf "$stage" "dist/$name.zip"
mkdir -p "$stage"
cp "$bin" "$stage/Frostfire.exe"
cp LICENSE "$stage/LICENSE.txt"
cp README.md "$stage/README.md"
if command -v 7z >/dev/null 2>&1; then
  (cd "$stage" && 7z a -tzip -mx=9 "../$name.zip" . >/dev/null)
else
  (cd "$stage" && zip -9 -q "../$name.zip" ./*)
fi
rm -rf "$stage"
ls -l "dist/$name.zip"
