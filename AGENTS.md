# Frostfire Agent Guide

## Purpose

Frostfire is a native desktop app (macOS, Windows, Linux) for every LG ThinQ appliance:
a Rune window, plus a menu bar item on macOS. It talks only to LG's official ThinQ
Connect API with a personal access token. Keep it small, safe and honest about what a
device can do.

## Layout

- `src/main.rs`: entry point, headless scenes for screenshots, `--dump-profiles`
- `src/app.rs`: the app state and everything the UI asks for (send, nudge, timers, settings)
- `src/worker.rs`: the one background thread for network, Keychain and disk; `Job` in, `Event` out
- `src/thinq/`: the ThinQ Connect client (`client.rs`), countries and regions (`region.rs`),
  the device-type catalog (`kinds.rs`), any device's profile read generically, with its
  state paths and checked changes (`spec.rs`), the air conditioner's richer model and
  commands (`model.rs`), the zeroizing `Secret`
- `src/store.rs`: config file and the token in the platform's credential store
- `src/status_item.rs`: the menu bar item and its menu (AppKit through objc2; the only `unsafe`)
- `src/ui/`: the window. `mod.rs` lays out the sidebar and panel, `control.rs` an air
  conditioner's pane, `device.rs` every other device's pane (drawn from its `Spec`),
  `dial.rs` the dial, `airflow.rs` louvre positions, `settings.rs`, `welcome.rs`,
  `parts.rs` shared rows and cards, `theme.rs` the tints, `icons.rs` modes and device types
- `src/gfx.rs`: the rune-fx compositor for the dial's frost, embers and glow
- `src/demo.rs`: the demo home (two ACs and five other devices), also used by scenes and tests
- `tests/fixtures/`: synthetic ThinQ responses (no real device IDs); `devices/` holds one
  profile and state per shape LG uses (plain, per location, per part, per list entry)

## Commands

```sh
cargo run
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt
cargo run -- --snap out.png --scene cool --scale 2 --size 880x660 --theme light
scripts/bundle.sh && scripts/package.sh
```

## Rules

- Security is the point. The token stays in the Keychain and in `Secret`; never log,
  print, format or persist it anywhere else. Errors go through the client's redaction.
- Only HTTPS to `api-{kic,aic,eic}.lgthinq.com`. No new hosts, telemetry or analytics.
- Validate before sending: device IDs, countries, and every change against the device's
  profile (`Command::payload` for ACs, `Spec::payload` for everything). The UI never builds
  control JSON itself.
- Network, Keychain and disk work happen on the worker thread, never in a view or `poll`.
- The UI shows only what the profile offers. Don't fake capabilities: if LG's API
  can't set something for a model, say so plainly instead of drawing a control.
- No per-model tables. New device types work through `Spec`; add a name to `kinds.rs`,
  an icon to `icons.rs` and, if a key reads badly, a word to `spec::name`.
- Remote starts of devices that heat or cut (`Kind::careful`) take a second press.
- Scrolling must never change a device; only deliberate input does (drag, buttons, keys).
- Design follows Hex: neutral OpenCode tokens (`rune-theme-opencode`), a sidebar and a
  raised panel, section labels over cards of rows. The tint (an AC's mode, or another
  device's family) appears only on the dial, the device icon and the energy bars. Effects live on the dial and must stop
  (`Gfx::animating` false) whenever nothing moves.
- Every Rune crate and `[patch.crates-io]` entry pins the same Rune revision. Bump them together.
- When a change alters the window, render the scenes in light and dark and look at them,
  and refresh `docs/images/` if the README pictures changed.
- Keep `unsafe` confined to `status_item.rs`, on the main thread.

## Releases

- Push a tag `vX.Y.Z` (matching `Cargo.toml`'s version) to publish a release:
  `.github/workflows/release.yml` runs `build.yml` (a universal macOS app, Windows x64 and
  Arm64, Linux x64) and attaches `Frostfire.dmg`, `Frostfire-macos.zip`,
  `Frostfire-windows-x64.zip`, `Frostfire-windows-arm64.zip`, `Frostfire-linux-x64.tar.gz`
  and `SHA256SUMS.txt`.
- Every push to `main` refreshes the `dev` prerelease (`.github/workflows/dev.yml`).
- The README's download links point at those asset names; keep them stable.
