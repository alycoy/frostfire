//! Settings: where Frostfire lives, units, effects, refresh and the account.

use rune::{El, icon, row, text};
use rune_kit::{Button, ButtonVariant, SegmentedControl, Switch};

use crate::app::App;
use crate::store::Effects;
use crate::ui::icons;
use crate::ui::parts::{self, P, card, section, setting};

pub fn view(s: &App, p: P) -> El {
    let cfg = &s.config;

    let shell = card(
        p,
        vec![
            setting(
                p,
                "set-menubar",
                None,
                "Show in menu bar",
                Some("Your device at a glance, and a menu to run it.".into()),
                Switch::new(cfg.menu_bar)
                    .label("Show in menu bar")
                    .hide_label()
                    .disabled(!cfg.dock_icon)
                    .on_toggle(|s: &mut App| s.set_config(|c| c.menu_bar = !c.menu_bar))
                    .build(p.k),
            ),
            setting(
                p,
                "set-dock",
                None,
                "Show in Dock",
                Some("Off: Frostfire lives only in the menu bar. Closing the window never quits it.".into()),
                Switch::new(cfg.dock_icon)
                    .label("Show in Dock")
                    .hide_label()
                    .disabled(!cfg.menu_bar)
                    .on_toggle(|s: &mut App| s.set_config(|c| c.dock_icon = !c.dock_icon))
                    .build(p.k),
            ),
        ],
    );

    let units = ["C", "F"];
    let unit = SegmentedControl::new(&["°C", "°F"], units.iter().position(|u| *u == cfg.unit))
        .label("Temperature unit")
        .width(Some(120.0))
        .on_pick(move |s: &mut App, i, _| s.set_config(|c| c.unit = units[i].into()))
        .build(p.k);
    let secs = [30u64, 60, 120, 300];
    let refresh = SegmentedControl::new(&["30 s", "1 min", "2 min", "5 min"], secs.iter().position(|v| *v == cfg.refresh_secs))
        .label("Check devices every")
        .fit()
        .on_pick(move |s: &mut App, i, _| s.set_config(|c| c.refresh_secs = secs[i]))
        .build(p.k);
    let fxs = [Effects::Full, Effects::Calm, Effects::Off];
    let fx = SegmentedControl::new(&["Alive", "Calm", "Still"], fxs.iter().position(|e| *e == cfg.effects))
        .label("Effects")
        .fit()
        .on_pick(move |s: &mut App, i, _| s.set_config(|c| c.effects = fxs[i]))
        .build(p.k);
    let general = card(
        p,
        vec![
            setting(p, "set-unit", None, "Temperature", None, unit),
            setting(p, "set-refresh", None, "Check devices every", Some("LG limits how often apps may ask. Your changes are sent at once.".into()), refresh),
            setting(p, "set-fx", None, "Effects", Some("Frost on the dial while cooling, embers while heating. Still draws nothing when idle.".into()), fx),
        ],
    );

    let who = if s.demo {
        "A pretend account. Nothing is sent anywhere.".to_string()
    } else {
        format!("LG ThinQ, {}. Your token is in your {}.", cfg.country.clone().unwrap_or_default(), crate::app::VAULT)
    };
    let leave = if s.demo { "Leave demo" } else { "Sign out" };
    let account = card(
        p,
        vec![setting(
            p,
            "account",
            Some(&icons::LOCK),
            if s.demo { "Demo" } else { "Connected" },
            Some(who),
            Button::new(leave).variant(ButtonVariant::Danger).on_press(|s: &mut App| s.sign_out()).build(p.k),
        )],
    );

    // The menu bar item and the Dock are macOS's; elsewhere Frostfire is a window.
    let mut page = parts::page("settings");
    if cfg!(target_os = "macos") {
        page = page.child(section(p, "general", "Menu bar & Dock", None, shell));
    }
    page.child(section(p, "prefs", "General", None, general)).child(section(p, "account", "Account", None, account)).child(
        row().px(parts::ROW_PX).gap(6.0).center_y().child(icon(&icons::LOCK, 12.0).color(p.faint())).child(
            text("Frostfire talks only to LG's official ThinQ Connect API. No accounts, analytics or middlemen.")
                .font(p.k.ty.small)
                .color(p.faint())
                .wrap()
                .grow(1.0),
        ),
    )
}
