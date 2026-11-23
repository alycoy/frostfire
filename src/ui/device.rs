//! The pane for every device that isn't an air conditioner, drawn from its own profile: what it
//! is doing up top (with a dial when it holds one humidity or temperature), then each thing LG
//! lets you change, then everything else it reports. A device type Frostfire has never seen
//! still gets every control its profile offers, and nothing it doesn't.

use std::collections::HashSet;

use rune::{El, col, icon, row, text};
use rune_kit::{Button, ButtonVariant, IconButton, IconButtonVariant, SegmentedControl, Select, Switch};
use serde_json::Value;

use crate::app::{App, fmt_minutes};
use crate::thinq::model::{Kind, label};
use crate::thinq::spec::{self, Field, Path, Spec, Zone};
use crate::ui::parts::{self, P, card, detail, hairline, heading, section, setting};
use crate::ui::{control, dial, icons};

/// One line of the pane: a property, or an hour and minute pair read as one time.
enum Entry<'a> {
    One(&'a Field),
    Time { hour: &'a Field, minute: &'a Field, stem: String },
}

impl Entry<'_> {
    fn paths(&self) -> Vec<&Path> {
        match self {
            Entry::One(f) => vec![&f.path],
            Entry::Time { hour, minute, .. } => vec![&hour.path, &minute.path],
        }
    }
}

/// A zone's fields, with read-only `…Hour…`/`…Minute…` pairs joined.
fn entries<'a>(spec: &Spec, zone: &'a Zone, unit: crate::thinq::model::Unit) -> Vec<Entry<'a>> {
    let mut out = Vec::new();
    let mut taken: HashSet<&Path> = HashSet::new();
    for f in zone.fields.iter().filter(|f| spec.visible(f, unit)) {
        if taken.contains(&f.path) {
            continue;
        }
        if !f.writable() && f.path.key.contains("Hour") {
            let want = f.path.key.replacen("Hour", "Minute", 1);
            if let Some(m) = zone.fields.iter().find(|g| g.path.res == f.path.res && g.path.item == f.path.item && g.path.key == want && !g.writable()) {
                taken.insert(&m.path);
                out.push(Entry::Time { hour: f, minute: m, stem: f.path.key.replacen("Hour", "", 1) });
                continue;
            }
        }
        if f.path.key.contains("Minute") && !f.writable() {
            let want = f.path.key.replacen("Minute", "Hour", 1);
            if zone.fields.iter().any(|g| g.path.res == f.path.res && g.path.item == f.path.item && g.path.key == want && !g.writable()) {
                continue;
            }
        }
        out.push(Entry::One(f));
    }
    out
}

fn time_name(stem: &str) -> String {
    match stem {
        "remain" => "Time left".into(),
        "total" => "Cycle length".into(),
        "running" => "Running for".into(),
        "relativeToStart" => "Starts in".into(),
        "relativeToStop" => "Stops in".into(),
        "absoluteToStart" => "Starts at".into(),
        "absoluteToStop" => "Stops at".into(),
        "target" => "Cook time".into(),
        _ => spec::humanize(stem),
    }
}

/// The minutes in a pair, when the state has them.
fn minutes(s: &App, hour: &Field, minute: &Field) -> Option<(u32, u32)> {
    let h = s.value(&hour.path).and_then(|v| v.as_f64());
    let m = s.value(&minute.path).and_then(|v| v.as_f64());
    if h.is_none() && m.is_none() {
        return None;
    }
    Some((h.unwrap_or(0.0).max(0.0) as u32, m.unwrap_or(0.0).max(0.0) as u32))
}

fn time_value(stem: &str, (h, m): (u32, u32)) -> String {
    if stem.starts_with("absolute") {
        return format!("{h:02}:{m:02}");
    }
    match h * 60 + m {
        0 if stem == "remain" || stem == "running" => "—".into(),
        0 => "Not set".into(),
        total => fmt_minutes(total),
    }
}

/// A field's title: its name, with the compartment when it belongs to one ("Freezer target temperature").
fn title(f: &Field) -> String {
    let name = spec::name(&f.path.res, &f.path.key);
    match &f.path.item {
        Some((tag, place)) if tag == "locationName" => {
            let mut c = name.chars();
            let rest = c.next().map(|x| x.to_lowercase().collect::<String>() + c.as_str()).unwrap_or_default();
            format!("{} {rest}", spec::place(place))
        }
        _ => name,
    }
}

/// Read-only values worth a place beside the dial, best first.
const FACTS: [&str; 12] = [
    "currentTemperature",
    "currentTemperatureC",
    "currentTemperatureF",
    "currentHumidity",
    "humidity",
    "PM2",
    "PM10",
    "totalPollutionLevel",
    "percent",
    "doorState",
    "filterRemainPercent",
    "cycleCount",
];

pub fn view(s: &App, p: P, effects: bool, layer: &mut Vec<El>) -> El {
    let (Some(spec), Some(dev)) = (s.spec.as_ref(), s.current_device()) else { return control::loading(s, p) };
    let kind = dev.kind();
    let unit = s.unit();
    let on = s.power();
    let mut used: HashSet<Path> = HashSet::new();
    if let Some(f) = spec.power() {
        used.insert(f.path.clone());
    }
    let dial_field = spec.dial(unit);
    if let Some(f) = dial_field {
        used.insert(f.path.clone());
    }
    let zones: Vec<(&Zone, Vec<Entry>)> = spec.zones.iter().map(|z| (z, entries(spec, z, unit))).collect();
    let multi = zones.len() > 1;

    // ---- what it's doing: the run state of each part, the time left, a few readings
    let mut runs: Vec<(Option<String>, String)> = Vec::new();
    let mut left: Option<String> = None;
    let mut facts: Vec<(String, String)> = Vec::new();
    for (z, list) in &zones {
        for e in list {
            match e {
                Entry::One(f) if f.path.res == "runState" && f.path.key == "currentState" => {
                    if let Some(Value::String(v)) = s.value(&f.path) {
                        runs.push((z.title(), label(&v)));
                    }
                    used.insert(f.path.clone());
                }
                Entry::Time { hour, minute, stem } if stem == "remain" => {
                    if let Some(t) = minutes(s, hour, minute).filter(|(h, m)| h + m > 0) {
                        let v = time_value(stem, t);
                        left = Some(match z.title() {
                            Some(zt) if multi => format!("{zt}: {v} left"),
                            _ => format!("{v} left"),
                        });
                    }
                }
                _ => {}
            }
        }
    }
    // Without a dial, each compartment's target leads ("Fridge 3°", "Freezer −18°"); its
    // stepper stays below.
    if dial_field.is_none() {
        for f in spec.fields().filter(|f| f.path.key.starts_with("targetTemperature") && spec.visible(f, unit)) {
            if let (Some(v), Some((_, place))) = (s.value(&f.path), f.path.item.as_ref().filter(|(t, _)| t == "locationName")) {
                facts.push((spec::place(place), spec::show(f, &v)));
            }
        }
    }
    for key in FACTS {
        for (z, list) in &zones {
            for e in list {
                let Entry::One(f) = e else { continue };
                if f.path.key != key || f.writable() || used.contains(&f.path) || facts.len() >= 5 {
                    continue;
                }
                let Some(v) = s.value(&f.path) else { continue };
                let name = match z.title() {
                    Some(zt) if multi => format!("{zt} {}", title(f).to_lowercase()),
                    _ => title(f),
                };
                facts.push((name, spec::show(f, &v)));
                used.insert(f.path.clone());
            }
        }
    }
    // The mode while it runs ("Auto", "Smart humidity"), as an AC shows "Cooling".
    let mode = spec
        .fields()
        .find(|f| f.path.key == "currentJobMode" && f.path.scope == spec::Scope::Root)
        .and_then(|f| s.value(&f.path))
        .and_then(|v| v.as_str().map(label));
    let headline = match (runs.len(), spec.power(), mode) {
        (1, _, _) => runs[0].1.clone(),
        (0, Some(_), _) if !on => "Off".to_string(),
        (0, _, Some(m)) if on => m,
        (0, Some(_), None) => "On".to_string(),
        _ => kind.name.to_string(),
    };
    let subtitle = if runs.len() > 1 {
        runs.iter().map(|(z, r)| format!("{}: {r}", z.clone().unwrap_or_default())).collect::<Vec<_>>().join(" · ")
    } else if let Some(l) = &left {
        l.clone()
    } else if dev.model.is_empty() {
        kind.name.to_string()
    } else if headline == kind.name {
        dev.model.clone()
    } else {
        format!("{} · {}", kind.name, dev.model)
    };
    let mut fact_rows: Vec<El> = Vec::new();
    for (l, v) in facts {
        if !fact_rows.is_empty() {
            fact_rows.push(hairline(p));
        }
        fact_rows.push(detail(p, &l, v));
    }
    let glyph = icons::for_kind(kind.code);
    let summary = col()
        .grow(1.0)
        .min_w(0.0)
        .gap(4.0)
        .child(
            row()
                .gap(8.0)
                .center_y()
                .child(icon(glyph, 18.0).color(if on { p.m.accent } else { p.faint() }))
                .child(text(headline).font(heading(20.0, 600.0)).color(p.text()).truncate()),
        )
        .child(text(subtitle).font(p.k.ty.ui).color(p.muted()).wrap().mb(if fact_rows.is_empty() { 0.0 } else { 10.0 }))
        .children(fact_rows);
    let lead = if dial_field.is_some() {
        dial::view(s, p, effects)
    } else {
        col()
            .key("device-tile")
            .size(76.0, 76.0)
            .radius(20.0)
            .fill(if on { p.k.c.bg_base.mix(p.m.accent, if p.dark { 0.11 } else { 0.12 }).alpha(1.0) } else { p.well() })
            .center()
            .child(icon(glyph, 34.0).color(if on { p.m.accent } else { p.muted() }))
            .a11y_hidden()
    };
    let gap = if dial_field.is_some() { 28.0 } else { 20.0 };
    let mut page = parts::page("device").child(card(p, vec![row().key("hero").w_full().padding(20.0).gap(gap).center_y().child(lead).child(summary)]));

    // ---- each part: what you can change, then what it reports
    for (z, list) in &zones {
        let zkey = format!("{:?}", z.scope);
        let mut controls: Vec<El> = Vec::new();
        let mut status: Vec<El> = Vec::new();
        for e in list {
            if e.paths().iter().any(|p| used.contains(*p)) {
                continue;
            }
            match e {
                Entry::One(f) if f.writable() => {
                    if let Some(r) = control_row(s, p, f, layer) {
                        controls.push(r);
                    }
                }
                Entry::One(f) => {
                    if let Some(v) = s.value(&f.path) {
                        if !status.is_empty() {
                            status.push(hairline(p));
                        }
                        status.push(detail(p, &title(f), spec::show(f, &v)));
                    }
                }
                Entry::Time { hour, minute, stem } => {
                    if let Some(t) = minutes(s, hour, minute) {
                        if !status.is_empty() {
                            status.push(hairline(p));
                        }
                        status.push(detail(p, &time_name(stem), time_value(stem, t)));
                    }
                }
            }
        }
        let zt = if multi { z.title() } else { None };
        if !controls.is_empty() {
            let t = zt.clone().unwrap_or_else(|| "Controls".into());
            page = page.child(section(p, &format!("controls-{zkey}"), &t, None, card(p, controls)));
        }
        if !status.is_empty() {
            let t = zt.map(|z| format!("{z} status")).unwrap_or_else(|| "Status".into());
            page = page.child(section(p, &format!("status-{zkey}"), &t, None, card(p, vec![col().w_full().px(parts::ROW_PX).py(6.0).children(status)])));
        }
    }

    if !s.energy.is_empty() {
        page = page.child(control::energy(s, p));
    }
    page.child(text(parts::status(s)).font(p.k.ty.small).color(p.faint()).px(parts::ROW_PX))
}

/// The row for a writable property: buttons for actions, a switch for on/off, a segmented
/// choice or a dropdown for modes, a stepper for numbers.
fn control_row(s: &App, p: P, f: &Field, layer: &mut Vec<El>) -> Option<El> {
    let key = format!("f-{:?}-{}-{:?}-{}", f.path.scope, f.path.res, f.path.item, f.path.key);
    let name = title(f);
    let cur = s.value(&f.path);
    let path = f.path.clone();
    let row_with = |desc: Option<String>, control: El| Some(setting(p, &key, None, &name, desc, control));
    match &f.prop.kind {
        Kind::Enum { .. } => {
            let choices = f.choices().to_vec();
            if choices.is_empty() {
                return None;
            }
            let now = cur.as_ref().and_then(Value::as_str).map(str::to_string);
            if !f.prop.readable || choices.iter().all(|c| spec::is_action(c)) {
                let armed = s.armed.as_ref().map(|(a, _)| a.clone());
                let mut buttons = row().gap(6.0).center_y();
                for v in choices {
                    let hot = armed.as_deref() == Some(format!("{path:?}={v}").as_str());
                    let path = path.clone();
                    let value = v.clone();
                    buttons = buttons.child(
                        Button::new(if hot { "Press again".to_string() } else { spec::action_label(&v) })
                            .variant(if hot { ButtonVariant::Contrast } else { ButtonVariant::Neutral })
                            .on_press(move |s: &mut App| s.act(&path, &value))
                            .build(p.k),
                    );
                }
                return row_with(now.filter(|_| f.prop.readable).map(|n| format!("Now {}", label(&n))), buttons);
            }
            let mut sorted = choices.clone();
            sorted.sort();
            if sorted == ["OFF", "ON"] {
                let on = now.as_deref() == Some("ON");
                return row_with(
                    None,
                    Switch::new(on)
                        .label(name.clone())
                        .hide_label()
                        .on_toggle(move |s: &mut App| s.set(&path, Value::String(if on { "OFF" } else { "ON" }.into())))
                        .build(p.k),
                );
            }
            let labels: Vec<String> = choices.iter().map(|c| label(c)).collect();
            let sel = now.as_ref().and_then(|n| choices.iter().position(|c| c == n));
            if choices.len() <= 4 && labels.iter().map(|l| l.chars().count() + 3).sum::<usize>() <= 34 {
                return row_with(
                    None,
                    SegmentedControl::new(&labels, sel)
                        .label(name.clone())
                        .fit()
                        .on_pick(move |s: &mut App, i, _| {
                            if let Some(v) = choices.get(i) {
                                s.set(&path, Value::String(v.clone()));
                            }
                        })
                        .build(p.k),
                );
            }
            let open = s.open_select.as_deref() == Some(key.as_str());
            let (k1, k2) = (key.clone(), key.clone());
            let pairs: Vec<(String, String)> = labels.iter().cloned().zip(choices.iter().cloned()).collect();
            let select = Select::new(key.clone(), &labels)
                .label(name.clone())
                .width(190.0)
                .current(sel.map(|i| labels[i].as_str()))
                .placeholder("Choose")
                .open(open)
                .on_toggle(move |s: &mut App| s.open_select = if s.open_select.as_deref() == Some(k1.as_str()) { None } else { Some(k1.clone()) })
                .on_close(move |s: &mut App| {
                    if s.open_select.as_deref() == Some(k2.as_str()) {
                        s.open_select = None;
                    }
                })
                .on_choose(move |s: &mut App, picked, _| {
                    s.open_select = None;
                    if let Some((_, v)) = pairs.iter().find(|(l, _)| l == picked) {
                        s.set(&path, Value::String(v.clone()));
                    }
                })
                .build(p.k, layer);
            row_with(None, select)
        }
        Kind::Bool => {
            let on = cur.as_ref().and_then(Value::as_bool).unwrap_or(false);
            row_with(None, Switch::new(on).label(name.clone()).hide_label().on_toggle(move |s: &mut App| s.set(&path, Value::Bool(!on))).build(p.k))
        }
        Kind::Range { .. } => {
            let r = f.range()?;
            let at = cur.as_ref().and_then(Value::as_f64);
            // A delay below its range (0 hours) is a timer that isn't set.
            let shown = match (&cur, at) {
                (_, Some(v)) if v < r.min && v <= 0.0 => "Off".to_string(),
                (Some(v), _) => spec::show(f, v),
                _ => "—".into(),
            };
            let (down, up) = (path.clone(), path.clone());
            let stepper = row()
                .gap(2.0)
                .center_y()
                .child(
                    IconButton::new(&icons::MINUS)
                        .variant(IconButtonVariant::Ghost)
                        .label(format!("Lower {}", name.to_lowercase()))
                        .disabled(at.is_some_and(|v| v <= r.min))
                        .on_press(move |s: &mut App| s.step(&down, -1.0))
                        .build(p.k),
                )
                .child(text(shown).font(p.k.ty.ui_medium.features("'tnum' 1")).color(p.text()).min_w(58.0).align(rune::Align::Center))
                .child(
                    IconButton::new(&icons::PLUS)
                        .variant(IconButtonVariant::Ghost)
                        .label(format!("Raise {}", name.to_lowercase()))
                        .disabled(at.is_some_and(|v| v >= r.max))
                        .on_press(move |s: &mut App| s.step(&up, 1.0))
                        .build(p.k),
                );
            let span = format!("{} to {}", spec::show(f, &serde_json::json!(r.min)), spec::show(f, &serde_json::json!(r.max)));
            row_with(Some(span), stepper)
        }
        _ => None,
    }
}
