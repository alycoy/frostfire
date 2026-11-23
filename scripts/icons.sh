#!/bin/sh
# Regenerates the Windows and Linux icons from assets/icon-1024.png (macOS, needs sips and python3):
# assets/Frostfire.ico (16 to 256 px, PNG-compressed) and assets/icon-256.png.
set -eu
cd "$(dirname "$0")/.."
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
for s in 16 20 24 32 40 48 64 128 256; do
  sips -s format png -z "$s" "$s" assets/icon-1024.png --out "$tmp/$s.png" >/dev/null
done
cp "$tmp/256.png" assets/icon-256.png
python3 -I - assets/Frostfire.ico "$tmp"/*.png <<'PY'
import struct, sys
out, *paths = sys.argv[1:]
images = []
for path in paths:
    data = open(path, "rb").read()
    width, height = struct.unpack(">II", data[16:24])
    images.append((width, height, data))
images.sort(key=lambda i: i[0])
offset = 6 + 16 * len(images)
entries, blobs = b"", b""
for width, height, data in images:
    entries += struct.pack("<BBBBHHII", width % 256, height % 256, 0, 0, 1, 32, len(data), offset + len(blobs))
    blobs += data
open(out, "wb").write(struct.pack("<HHH", 0, 1, len(images)) + entries + blobs)
PY
ls -l assets/Frostfire.ico assets/icon-256.png
