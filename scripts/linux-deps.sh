#!/bin/sh
# Installs what building Frostfire on Debian or Ubuntu needs (CI runs it): pkg-config, and the
# headers for OpenSSL (HTTPS through ureq's native-tls), fontconfig (Rune's font discovery)
# and xkbcommon (Rune's keyboard handling). Wayland, X11 and Vulkan are loaded at run time
# and need nothing here.
set -eu
packages="pkg-config libssl-dev libfontconfig1-dev libxkbcommon-dev"
sudo=$( [ "$(id -u)" = 0 ] || echo sudo )
for attempt in 1 2 3; do
  $sudo apt-get -o Acquire::Retries=3 update -qq &&
    $sudo apt-get -o Acquire::Retries=3 install -y -qq --no-install-recommends $packages && exit 0
  [ "$attempt" = 3 ] && exit 1
  sleep $((attempt * 15))
done
