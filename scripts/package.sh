#!/bin/sh
# Packs dist/Frostfire.app into dist/Frostfire.dmg (drag to Applications) and
# dist/Frostfire-macos.zip. Run scripts/bundle.sh first.
set -eu
cd "$(dirname "$0")/.."
app=dist/Frostfire.app
[ -d "$app" ] || { echo "no $app: run scripts/bundle.sh first" >&2; exit 1; }
rm -f dist/Frostfire.dmg dist/Frostfire-macos.zip
ditto -c -k --keepParent "$app" dist/Frostfire-macos.zip
stage=$(mktemp -d)
cp -R "$app" "$stage/"
ln -s /Applications "$stage/Applications"
hdiutil create -quiet -volname Frostfire -srcfolder "$stage" -ov -format UDZO dist/Frostfire.dmg
rm -rf "$stage"
ls -lh dist/Frostfire.dmg dist/Frostfire-macos.zip
