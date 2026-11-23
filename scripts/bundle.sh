#!/bin/sh
# Builds dist/Frostfire.app with its icon.
#
#   scripts/bundle.sh                         release build for this Mac
#   FROSTFIRE_BIN=path scripts/bundle.sh      bundle a prebuilt binary (CI's universal one)
set -eu
cd "$(dirname "$0")/.."
bin=${FROSTFIRE_BIN:-}
if [ -z "$bin" ]; then
  cargo build --release
  bin=target/release/frostfire
fi
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
app=dist/Frostfire.app
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$bin" "$app/Contents/MacOS/Frostfire"
cp assets/Frostfire.icns "$app/Contents/Resources/Frostfire.icns"
cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Frostfire</string>
  <key>CFBundleDisplayName</key><string>Frostfire</string>
  <key>CFBundleIdentifier</key><string>io.github.alycoy.frostfire</string>
  <key>CFBundleExecutable</key><string>Frostfire</string>
  <key>CFBundleIconFile</key><string>Frostfire</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>LSMinimumSystemVersion</key><string>13.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.utilities</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSHumanReadableCopyright</key><string>MIT License</string>
</dict>
</plist>
PLIST
# Sign with a stable identity when there is one, so the Keychain's "Always Allow" survives
# rebuilds (an ad-hoc signature changes with every build and macOS asks again). Set
# FROSTFIRE_SIGN to your identity, or create a local one named "Frostfire Local Signing".
identity=${FROSTFIRE_SIGN:-"Frostfire Local Signing"}
if security find-identity -p codesigning 2>/dev/null | grep -q "\"$identity\""; then
  codesign --force --options runtime --sign "$identity" "$app"
else
  echo "note: no \"$identity\" signing identity; signing ad-hoc (the Keychain will ask after each rebuild)" >&2
  codesign --force --sign - "$app"
fi
echo "$app"
