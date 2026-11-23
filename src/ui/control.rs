//! A unit's pane: the dial beside what the room is doing, then every control the unit offers,
//! and only those.

use rune::{El, col, el, icon, row, text};
use rune_kit::{Button, ButtonVariant, SegmentedControl, Switch};

use crate::app::{App, fmt_minutes, fmt_temp};
use crate::thinq::model::{Command, Timer, label};
use crate::ui::parts::{self, P, card, detail, hairline, heading, section, setting, wide};
use crate::ui::{airflow, dial, icons};

const TIMER_CHOICES: [u32; 5] = [30, 60, 120, 240, 480];

pub fn view(s: &App, p: P, effects: bool) -> El {
    let Some(profile) = s.profile.as_ref() else { return loading(s, p) };
    let state = s.state.clone().unwrap_or_default();
    let on = s.power();
    let mut page = parts::page("device");

    // ---- the dial, and what the room is doing
    let doing = match (on, s.mode()) {
        (false, _) => "Off".to_string(),
        (true, Some("COOL")) => "Cooling".into(),
        (true, Some("HEAT")) => "Heating".into(),
        (true, Some(m)) => label(m),
        (true, None) => "On".into(),
    };
    let mode_icon = s.mode().map(icons::for_mode).unwrap_or(&icons::POWER);
    let mut facts: Vec<El> = Vec::new();
    let mut fact = |l: &str, v: String| {
        if !facts.is_empty() {
            facts.push(hairline(p));
        }
        facts.push(detail(p, l, v));
    };
    if let Some(r) = s.room() {
        fact("Room", format!("{}°", fmt_temp(r)));
    }
    if let Some(t) = s.target() {
        fact("Target", format!("{}°", fmt_temp(t)));
    }
    if let Some(h) = state.num("airQualitySensor", "humidity") {
        fact("Humidity", format!("{}%", h.round()));
    }
    let speeds = profile.wind_speeds();
    if let Some(sp) = state.wind() {
        fact("Fan", label(sp));
    }
    if let Some(level) = state.str("airQualitySensor", "totalPollutionLevel") {
        fact("Air quality", label(level));
    }
    if let Some(f) = state.filter_percent() {
        fact("Filter", format!("{}% left", f.round()));
    }
    let summary = col()
        .grow(1.0)
        .min_w(0.0)
        .gap(4.0)
        .child(
            row()
                .gap(8.0)
                .center_y()
                .child(icon(mode_icon, 18.0).color(if on { p.m.accent } else { p.faint() }))
                .child(text(doing).font(heading(20.0, 600.0)).color(p.text())),
        )
        .child(text(subtitle(s)).font(p.k.ty.ui).color(p.muted()).wrap().mb(10.0))
        .children(facts);
    let hero = card(p, vec![row().key("hero").w_full().padding(20.0).gap(28.0).center_y().child(dial::view(s, p, effects)).child(summary)]);
    page = page.child(hero);

    // ---- mode
    let modes = profile.modes();
    if !modes.is_empty() {
        let current = state.mode().unwrap_or("");
        let labels: Vec<String> = modes.iter().map(|m| label(m)).collect();
        let picks = modes.clone();
        let seg = SegmentedControl::new(&labels, modes.iter().position(|m| m == current))
            .label("Mode")
            .width(None)
            .on_pick(move |s: &mut App, i, _| {
                if let Some(m) = picks.get(i) {
                    s.send(Command::Mode(m.clone()));
                }
            })
            .build(p.k);
        page = page.child(section(p, "mode", "Mode", None, card(p, vec![wide("mode-row", seg)])));
    }

    // ---- fan
    let mut fan_rows: Vec<El> = Vec::new();
    if !speeds.is_empty() {
        let current = state.wind().unwrap_or("");
        let labels: Vec<String> = speeds.iter().map(|m| if speeds.len() > 5 { short_speed(m) } else { label(m) }).collect();
        let picks = speeds.clone();
        fan_rows.push(wide(
            "fan-row",
            SegmentedControl::new(&labels, speeds.iter().position(|m| m == current))
                .label("Fan speed")
                .width(None)
                .on_pick(move |s: &mut App, i, _| {
                    if let Some(m) = picks.get(i) {
                        s.send(Command::Wind(m.clone()));
                    }
                })
                .build(p.k),
        ));
    }
    if let Some(r) = profile.wind_step() {
        let level = state.num("airFlow", "windStep").unwrap_or(r.min);
        let levels: Vec<String> = (0..=((r.max - r.min) / r.step.max(1.0)) as usize).map(|i| fmt_temp(r.min + i as f64 * r.step.max(1.0))).collect();
        let sel = levels.iter().position(|l| *l == fmt_temp(level));
        fan_rows.push(setting(
            p,
            "fan-level",
            Some(&icons::FAN),
            "Fan level",
            Some(format!("{} of {}", fmt_temp(level), fmt_temp(r.max))),
            SegmentedControl::new(&levels, sel)
                .label("Fan level")
                .fit()
                .on_pick(move |s: &mut App, i, _| s.send(Command::WindStep(r.min + i as f64 * r.step.max(1.0))))
                .build(p.k),
        ));
    }
    if !fan_rows.is_empty() {
        // A speed set on the remote that the API can't set (LG units report LOW_MID and the like).
        let note = match state.wind() {
            Some(cur) if !speeds.iter().any(|s| s == cur) => Some(format!("Now {} · only the remote can set it", label(cur))),
            _ => None,
        };
        page = page.child(section(p, "fan", "Fan speed", note, card(p, fan_rows)));
    }

    // ---- airflow: louvre positions, swing, extras
    let vanes = profile.vanes();
    let flags = profile.wind_flags();
    let mut air_rows: Vec<El> = Vec::new();
    if !vanes.is_empty() {
        let views = row().key("vanes").w_full().px(16.0).py(16.0).gap(16.0).justify_center().flex_wrap().children(vanes.iter().map(|v| airflow::vane(s, p, v)));
        air_rows.push(views);
    }
    let positions = !vanes.is_empty();
    for (key, name) in flags {
        let v = state.bool("windDirection", key).unwrap_or(false);
        let (glyph, what) = match key {
            "rotateUpDown" if !positions => {
                (&icons::SWING_V, "Sweeps the air up and down. LG's API only takes swing on or off for this unit; set a fixed louvre position with the remote.")
            }
            "rotateUpDown" => (&icons::SWING_V, "Sweeps the air up and down the room."),
            "rotateLeftRight" => (&icons::SWING_H, "Sweeps the air from side to side."),
            _ => (&icons::WIND, ""),
        };
        air_rows.push(setting(
            p,
            &format!("flag-{key}"),
            Some(glyph),
            name,
            (!what.is_empty()).then(|| what.to_string()),
            Switch::new(v).label(name).hide_label().on_toggle(move |s: &mut App| s.send(Command::WindFlag(key.into(), !v))).build(p.k),
        ));
    }
    if !air_rows.is_empty() {
        page = page.child(section(p, "airflow", "Airflow", None, card(p, air_rows)));
    }

    // ---- extras
    let mut extra: Vec<El> = Vec::new();
    if profile.writable("powerSave", "powerSaveEnabled") {
        let v = state.bool("powerSave", "powerSaveEnabled").unwrap_or(false);
        extra.push(setting(
            p,
            "eco",
            Some(&icons::LEAF),
            "Energy saver",
            Some("Eases off once the room is close to the target.".into()),
            Switch::new(v).label("Energy saver").hide_label().on_toggle(move |s: &mut App| s.send(Command::PowerSave(!v))).build(p.k),
        ));
    }
    if profile.can_air_clean() {
        let v = state.str("operation", "airCleanOperationMode") == Some("START");
        extra.push(setting(
            p,
            "purify",
            Some(&icons::SPARKLE),
            "Purify air",
            Some("Runs the air through the filter.".into()),
            Switch::new(v).label("Purify air").hide_label().on_toggle(move |s: &mut App| s.send(Command::AirClean(!v))).build(p.k),
        ));
    }
    if profile.can_display_light() {
        let v = state.str("display", "light") != Some("OFF");
        extra.push(setting(
            p,
            "display",
            Some(&icons::BULB),
            "Display light",
            None,
            Switch::new(v).label("Display light").hide_label().on_toggle(move |s: &mut App| s.send(Command::DisplayLight(!v))).build(p.k),
        ));
    }
    if !extra.is_empty() {
        page = page.child(section(p, "extras", "Comfort", None, card(p, extra)));
    }

    // ---- timers
    let timers: Vec<(Timer, &str, &'static rune::Icon)> =
        [(Timer::Stop, "Turn off", &icons::CLOCK), (Timer::Sleep, "Sleep", &icons::MOON), (Timer::Start, "Turn on", &icons::POWER)]
            .into_iter()
            .filter(|(t, _, _)| profile.can_timer(*t) && (on || *t == Timer::Start) && (!on || *t != Timer::Start))
            .collect();
    if !timers.is_empty() {
        let rows = timers
            .into_iter()
            .map(|(which, title, glyph)| {
                let left = state.timer_minutes(which);
                let can_cancel = profile.can_cancel(which);
                let mut labels = vec!["Off".to_string()];
                labels.extend(TIMER_CHOICES.iter().map(|m| fmt_minutes(*m)));
                let sel = if left.is_none() { Some(0) } else { None };
                let desc = match (left, which) {
                    (Some(m), Timer::Start) => format!("Turns on in {}", fmt_minutes(m)),
                    (Some(m), Timer::Sleep) => format!("Sleeps in {}, easing the temperature", fmt_minutes(m)),
                    (Some(m), _) => format!("Turns off in {}", fmt_minutes(m)),
                    (None, Timer::Sleep) => "Eases the temperature, then turns off".into(),
                    (None, _) => "Not set".into(),
                };
                setting(
                    p,
                    &format!("timer-{which:?}"),
                    Some(glyph),
                    title,
                    Some(desc),
                    SegmentedControl::new(&labels, sel)
                        .label(format!("{title} timer"))
                        .fit()
                        .on_pick(move |s: &mut App, i, _| match i {
                            0 if can_cancel => s.set_timer(which, None),
                            0 => {}
                            i => s.set_timer(which, Some(TIMER_CHOICES[i - 1])),
                        })
                        .build(p.k),
                )
            })
            .collect();
        page = page.child(section(p, "timers", "Timers", None, card(p, rows)));
    }

    if !s.energy.is_empty() {
        page = page.child(energy(s, p));
    }
    page.child(text(parts::status(s)).font(p.k.ty.small).color(p.faint()).px(parts::ROW_PX))
}

/// Fan speeds short enough for eight segments.
fn short_speed(v: &str) -> String {
    match v {
        "MID" => "Mid".into(),
        "MID_HIGH" => "Mid+".into(),
        "HIGH_POWER" => "High+".into(),
        _ => label(v),
    }
}

fn subtitle(s: &App) -> String {
    let dev = s.current_device().map(|d| d.model.clone()).unwrap_or_default();
    match (s.power(), s.room(), s.target(), s.mode()) {
        (true, Some(r), Some(t), Some("COOL")) if r > t + 0.4 => format!("Bringing the room from {}° down to {}°", fmt_temp(r), fmt_temp(t)),
        (true, Some(r), Some(t), Some("HEAT")) if r < t - 0.4 => format!("Bringing the room from {}° up to {}°", fmt_temp(r), fmt_temp(t)),
        (true, Some(_), Some(t), _) => format!("Holding the room around {}°", fmt_temp(t)),
        (false, Some(r), _, _) => format!("The room is at {}°. {dev}", fmt_temp(r)),
        _ => dev,
    }
}

fn energy(s: &App, p: P) -> El {
    let max = s.energy.iter().map(|(_, v)| *v).fold(1.0f64, f64::max);
    let total: f64 = s.energy.iter().map(|(_, v)| *v).sum();
    let last = s.energy.len().saturating_sub(1);
    let bars = s.energy.iter().enumerate().map(|(i, (day, v))| {
        let h = (64.0 * (*v / max)) as f32;
        col()
            .key(format!("day{i}"))
            .grow(1.0)
            .items_center()
            .gap(6.0)
            .label(format!("{day}: {:.1} kWh", v / 1000.0))
            .child(text(format!("{:.1}", v / 1000.0)).font(p.k.ty.small).color(if i == last { p.text() } else { p.faint() }))
            .child(col().h(64.0).w_full().justify_end().items_center().child(el().w(22.0).h(h.max(3.0)).radius(4.0).fill(if i == last {
                p.m.accent
            } else {
                p.well()
            })))
            .child(text(day.clone()).font(p.k.ty.small).color(p.muted()))
    });
    let chart = row().key("bars").w_full().px(16.0).pt(14.0).pb(12.0).gap(6.0).children(bars);
    section(p, "energy", "Energy", Some(format!("Last 7 days · {:.1} kWh", total / 1000.0)), card(p, vec![chart]))
}

fn loading(s: &App, p: P) -> El {
    let (title, msg) = if s.acs().is_empty() && !s.loading {
        ("No air conditioners yet", "Add one in the LG ThinQ app, then refresh.")
    } else {
        ("Connecting", "Asking your air conditioner what it can do…")
    };
    col()
        .key("loading")
        .w_full()
        .h(360.0)
        .center()
        .gap(6.0)
        .child(text(title).font(p.k.ty.body_medium).color(p.text()))
        .child(text(msg).font(p.k.ty.ui).color(p.muted()))
        .child(Button::new("Refresh").variant(ButtonVariant::Neutral).on_press(|s: &mut App| s.refresh()).build(p.k).mt(10.0))
}
