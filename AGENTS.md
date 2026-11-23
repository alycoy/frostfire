# Frostfire Agent Guide

## Purpose

Frostfire is a native macOS app for LG ThinQ air conditioners: a Rune window and a
menu bar item. It talks only to LG's official ThinQ Connect API with a personal
access token. Keep it small, safe and honest about what a unit can do.

## Layout

- `src/main.rs`: entry point, headless scenes for screenshots, `--dump-profiles`
- `src/app.rs`: the app state and everything the UI asks for (send, nudge, timers, settings)
- `src/worker.rs`: the one background thread for network, Keychain and disk; `Job` in, `Event` out
- `src/thinq/`: the ThinQ Connect client (`client.rs`), countries and regions (`region.rs`),
  profiles, state and commands (`model.rs`), the zeroizing `Secret`
- `src/store.rs`: config file and Keychain
- `src/status_item.rs`: the menu bar item and its menu (AppKit through objc2; the only `unsafe`)
- `src/ui/`: the window. `mod.rs` lays out the sidebar and panel, `control.rs` a unit's pane,
  `dial.rs` the thermostat, `airflow.rs` louvre positions, `settings.rs`, `welcome.rs`,
  `parts.rs` shared rows and cards, `theme.rs` the mode tint
- `src/gfx.rs`: the rune-fx compositor for the dial's frost, embers and glow
- `src/demo.rs`: the demo account, also used by scenes and tests
- `tests/fixtures/`: synthetic ThinQ responses (no real device IDs)

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
- Validate before sending: device IDs, countries, and every command against the unit's
  profile (`Command::payload`). The UI never builds control JSON itself.
- Network, Keychain and disk work happen on the worker thread, never in a view or `poll`.
- The UI shows only what the profile offers. Don't fake capabilities: if LG's API
  can't set something for a model, say so plainly instead of drawing a control.
- Scrolling must never change the unit; only deliberate input does (drag, buttons, keys).
- Design follows Hex: neutral OpenCode tokens (`rune-theme-opencode`), a sidebar and a
  raised panel, section labels over cards of rows. The mode tint appears only on the
  dial, the mode icon and the energy bars. Effects live on the dial and must stop
  (`Gfx::animating` false) whenever nothing moves.
- Every Rune crate and `[patch.crates-io]` entry pins the same Rune revision. Bump them together.
- When a change alters the window, render the scenes in light and dark and look at them,
  and refresh `docs/images/` if the README pictures changed.
- Keep `unsafe` confined to `status_item.rs`, on the main thread.

## Releases

- Push a tag `vX.Y.Z` (matching `Cargo.toml`'s version) to publish a release:
  `.github/workflows/release.yml` builds a universal app and attaches
  `Frostfire.dmg`, `Frostfire-macos.zip` and `SHA256SUMS.txt`.
- Every push to `main` refreshes the `dev` prerelease (`.github/workflows/dev.yml`).
- The README's download links point at those asset names; keep them stable.
