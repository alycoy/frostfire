//! The thermostat dial: a 270° arc with the target in the middle. Drag around it, use − and +,
//! or focus it and use the arrow keys. Scrolling over it scrolls the page, never the target. The one place the weather shows: frost settles on the ring
//! while cooling, and embers rise through the middle while heating.

use rune::shape::{LineCap, Shape};
use rune::{Align, El, canvas, col, el, row, text};
use rune_kit::{IconButton, IconButtonVariant};

use crate::app::{App, fmt_temp};
use crate::gfx;
use crate::ui::icons;
use crate::ui::parts::{P, heading};
use crate::ui::theme::Weather;

pub const SIZE: f32 = 224.0;
const RING: f32 = 96.0;
const WIDTH: f32 = 9.0;
const START: f32 = 135.0;
const SWEEP: f32 = 270.0;

fn point(deg: f32, r: f32) -> [f32; 2] {
    let a = deg.to_radians();
    [SIZE / 2.0 + r * a.cos(), SIZE / 2.0 + r * a.sin()]
}

fn arc(from: f32, to: f32, r: f32) -> Vec<[f32; 2]> {
    let n = (((to - from).abs() / 2.0).ceil() as usize).max(2);
    (0..=n).map(|i| point(from + (to - from) * i as f32 / n as f32, r)).collect()
}

fn disc(c: [f32; 2], r: f32) -> Vec<[f32; 2]> {
    (0..32)
        .map(|i| {
            let a = i as f32 / 32.0 * std::f32::consts::TAU;
            [c[0] + r * a.cos(), c[1] + r * a.sin()]
        })
        .collect()
}

/// Where a point in the dial's own coordinates lands on the arc, 0..1.
pub fn fraction_at(x: f32, y: f32) -> f32 {
    let deg = (y - SIZE / 2.0).atan2(x - SIZE / 2.0).to_degrees();
    let a = (deg - START).rem_euclid(360.0);
    let a = if a > SWEEP { if a > SWEEP + (360.0 - SWEEP) / 2.0 { 0.0 } else { SWEEP } } else { a };
    a / SWEEP
}

pub fn view(s: &App, p: P, effects: bool) -> El {
    let m = p.m;
    let dial = s.dial();
    let spec = dial.map(|(r, _, _)| r);
    let target = dial.and_then(|(_, v, _)| v);
    let suffix = dial.map(|(_, _, s)| s).unwrap_or("°");
    let on = s.power();
    let frac = match (spec, target) {
        (Some(r), Some(t)) if r.max > r.min => ((t - r.min) / (r.max - r.min)).clamp(0.0, 1.0) as f32,
        _ => 0.0,
    };

    let track = p.well();
    let mut shape = Shape::new().stroke_with(arc(START, START + SWEEP, RING), false, WIDTH, LineCap::Round, track);
    for i in 0..=30 {
        let d = START + SWEEP * i as f32 / 30.0;
        let long = i % 5 == 0;
        let (a, b) = (point(d, RING - 13.0), point(d, RING - if long { 19.0 } else { 16.0 }));
        shape = shape.line(a, b, if long { 1.4 } else { 1.0 }, LineCap::Round, p.faint().alpha(if long { 0.7 } else { 0.35 }));
    }
    if spec.is_some() && target.is_some() {
        let end = START + SWEEP * frac;
        let steps = ((end - START) / 3.0).ceil().max(1.0) as usize;
        let tint = |t: f32| if on { m.accent.mix(m.accent_hi, t) } else { track.mix(p.text(), 0.25) };
        for i in 0..steps {
            let (a0, a1) = (START + (end - START) * i as f32 / steps as f32, START + (end - START) * (i + 1) as f32 / steps as f32);
            shape = shape.stroke_with(arc(a0, a1, RING), false, WIDTH, LineCap::Round, tint(i as f32 / steps as f32 * frac));
        }
        let k = point(end, RING);
        shape = shape.fill(disc(k, 9.5), p.k.c.border_base.alpha(1.0)).fill(disc(k, 8.5), rune::Color::WHITE).fill(disc(k, 3.5), tint(frac));
    }

    let value = target.map(fmt_temp).unwrap_or_else(|| "––".into());
    let family = s.kind().map(|k| k.family);
    let caption = match (on, s.mode()) {
        (false, _) => "Off".to_string(),
        (true, _) if !s.is_ac() => match (suffix, s.kind().map(|k| k.code)) {
            ("%", Some("DEVICE_DEHUMIDIFIER")) => "Drying to".into(),
            ("%", Some("DEVICE_HUMIDIFIER")) => "Humidifying to".into(),
            ("%", _) => "Humidity".into(),
            _ if family == Some(crate::thinq::kinds::Family::Heat) => "Heating to".into(),
            _ => "Keeping at".into(),
        },
        (true, Some("COOL")) => "Cooling to".into(),
        (true, Some("HEAT")) => "Heating to".into(),
        (true, Some("AUTO")) => "Keeping".into(),
        (true, Some("AIR_DRY")) => "Drying at".into(),
        (true, Some("FAN")) => "Fan only".into(),
        (true, Some(md)) => crate::thinq::model::label(md),
        (true, None) => "Target".into(),
    };
    let center = col().absolute(0.0, 0.0).size(SIZE, SIZE).center().gap(0.0).inert().child(text(caption).font(p.k.ty.ui_medium).color(p.muted())).child(
        row()
            .key("dial-value")
            .items_start()
            .child(text(value).font(heading(54.0, 300.0).features("'tnum' 1")).color(if on { p.text() } else { p.muted() }).align(Align::Center))
            .child(text(suffix).font(heading(22.0, 300.0)).color(p.muted()).mt(8.0)),
    );

    let range = spec;
    let (label, lower, higher) = if suffix == "%" {
        ("Target humidity", "Lower", "Higher")
    } else if s.is_ac() {
        ("Target temperature", "Cooler", "Warmer")
    } else {
        ("Target temperature", "Lower", "Higher")
    };
    let mut dial = el()
        .key("dial")
        .size(SIZE, SIZE)
        .role("slider")
        .label(label)
        .focusable(true)
        .focus_ring(2.0, 2.0, p.k.c.border_focus)
        .radius(SIZE / 2.0)
        .cursor(rune::Cursor::Pointer)
        .on_drag(|s: &mut App, d, _| {
            if let Some((r, _, _)) = s.dial() {
                let f = fraction_at(d.x, d.y) as f64;
                s.choose_target(r.min + f * (r.max - r.min));
            }
        })
        // No scroll-wheel handling: scrolling the page over the dial must never change the unit.
        .on_step(|s: &mut App, delta, _| s.nudge(delta as f64));
    if let (Some(r), Some(t)) = (range, target) {
        dial = dial.range_value(t, r.min, r.max).range_step(r.step);
    }
    let dark = if p.dark { 1.0 } else { 0.0 };
    if effects && on {
        let r = RING + WIDTH;
        dial = dial.child(
            rune::effect(gfx::GLOW)
                .effect_data([gfx::weather_index(m.weather), if p.dark { 0.10 } else { 0.14 }, 12.0, 0.0])
                .absolute(SIZE / 2.0 - r, SIZE / 2.0 - r)
                .size(r * 2.0, r * 2.0)
                .radius(r),
        );
        match m.weather {
            Weather::Frost => {
                dial = dial.child(
                    rune::effect(gfx::FROST).effect_data([14.0, 5.0, dark, 0.0]).absolute(SIZE / 2.0 - r, SIZE / 2.0 - r).size(r * 2.0, r * 2.0).radius(r),
                );
            }
            Weather::Fire => {
                // Embers rise out of the arc's open foot and fade before the number.
                let (w, h) = (88.0, 96.0);
                dial = dial.child(rune::effect(gfx::EMBERS).effect_data([0.55, 70.0, dark, 0.0]).absolute(SIZE / 2.0 - w / 2.0, SIZE - h).size(w, h));
            }
            _ => {}
        }
    }
    let stepper = row()
        .absolute(0.0, SIZE - 34.0)
        .w(SIZE)
        .justify_center()
        .gap(34.0)
        .child(
            IconButton::new(&icons::MINUS)
                .variant(IconButtonVariant::Ghost)
                .label(lower)
                .help(format!("{lower} (↓)"))
                .on_press(|s: &mut App| s.nudge(-1.0))
                .build(p.k),
        )
        .child(
            IconButton::new(&icons::PLUS)
                .variant(IconButtonVariant::Ghost)
                .label(higher)
                .help(format!("{higher} (↑)"))
                .on_press(|s: &mut App| s.nudge(1.0))
                .build(p.k),
        );
    dial.child(canvas(shape).absolute(0.0, 0.0).size(SIZE, SIZE)).child(center).child(stepper)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arc_fractions() {
        let at = |deg: f32| {
            let p = point(deg, RING);
            fraction_at(p[0], p[1])
        };
        assert!((at(START) - 0.0).abs() < 0.01);
        assert!((at(START + SWEEP / 2.0) - 0.5).abs() < 0.01, "top of the dial is the middle");
        assert!((at(START + SWEEP) - 1.0).abs() < 0.01);
        // The gap at the bottom snaps to the nearer end.
        assert_eq!(at(95.0), 0.0);
        assert_eq!(at(85.0), 1.0);
    }
}
