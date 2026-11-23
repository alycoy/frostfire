//! Where the air goes: a side view of the unit with its up–down louvre, and a view from above for
//! the left–right one. Click or drag in a view to park the louvre; the beam shows the position
//! and a soft wedge shows a swing.

use rune::shape::{LineCap, Shape};
use rune::{El, canvas, col, el, row, text};
use rune_kit::{Button, ButtonVariant, SegmentedControl};

use crate::app::App;
use crate::thinq::model::{Command, Vane, is_moving};
use crate::ui::parts::P;

pub const W: f32 = 276.0;
pub const H: f32 = 132.0;

/// The beam's origin and the angles (degrees, 0 = right, 90 = down) its positions point at.
fn geometry(side: bool, n: usize) -> ([f32; 2], Vec<f32>) {
    let (origin, from, to) = if side { ([W / 2.0, 26.0], 40.0, 140.0) } else { ([64.0, 34.0], 8.0, 78.0) };
    let angles = (0..n).map(|i| if n == 1 { (from + to) / 2.0 } else { from + (to - from) * i as f32 / (n - 1) as f32 }).collect();
    (origin, angles)
}

fn at(o: [f32; 2], deg: f32, r: f32) -> [f32; 2] {
    let a = deg.to_radians();
    [o[0] + r * a.cos(), o[1] + r * a.sin()]
}

/// The position nearest a point in the view.
fn nearest(side: bool, n: usize, x: f32, y: f32) -> usize {
    let (o, angles) = geometry(side, n);
    let deg = (y - o[1]).atan2(x - o[0]).to_degrees();
    angles.iter().enumerate().min_by(|a, b| (a.1 - deg).abs().total_cmp(&(b.1 - deg).abs())).map(|(i, _)| i).unwrap_or(0)
}

/// One louvre's picture. `pos` is the parked position's index, `None` while it swings.
fn picture(p: P, side: bool, n: usize, pos: Option<usize>, on: bool) -> El {
    let (o, angles) = geometry(side, n.max(1));
    let ink = p.faint();
    let beam = if on { p.m.accent } else { p.muted() };
    let mut s = Shape::new();
    // The room: the wall and floor (side view), or the wall the unit hangs on (from above).
    if side {
        s = s.line([W / 2.0 - 58.0, 14.0], [W / 2.0 + 58.0, 14.0], 1.0, LineCap::Round, ink.alpha(0.6));
        let unit = [[W / 2.0 - 42.0, 14.0], [W / 2.0 + 42.0, 14.0], [W / 2.0 + 38.0, 26.0], [W / 2.0 - 38.0, 26.0]];
        s = s.fill(unit.to_vec(), p.well().mix(p.text(), 0.18));
    } else {
        s = s.line([18.0, 4.0], [18.0, H - 10.0], 1.0, LineCap::Round, ink.alpha(0.6)).line(
            [18.0, H - 10.0],
            [W - 6.0, H - 10.0],
            1.0,
            LineCap::Round,
            ink.alpha(0.6),
        );
        let unit = [[18.0, 14.0], [60.0, 14.0], [64.0, 24.0], [60.0, 36.0], [18.0, 36.0]];
        s = s.fill(unit.to_vec(), p.well().mix(p.text(), 0.18));
    }
    let reach = if side { 96.0 } else { 150.0 };
    let clip = |pt: [f32; 2]| [pt[0].clamp(4.0, W - 4.0), pt[1].clamp(4.0, H - 12.0)];
    // Every position as a faint ray with a dot to aim at.
    for (i, a) in angles.iter().enumerate() {
        if Some(i) == pos {
            continue;
        }
        let end = clip(at(o, *a, reach * 0.82));
        s = s.line(at(o, *a, 10.0), end, 1.0, LineCap::Round, ink.alpha(0.35)).fill(dot(end, 2.2), ink.alpha(0.55));
    }
    match pos {
        Some(i) => {
            let a = angles[i];
            // A beam that thins and fades as it leaves the unit.
            for (k, spread) in [(0usize, -5.0f32), (1, 0.0), (2, 5.0)] {
                let mut prev = at(o, a + spread, 8.0);
                for j in 1..=10 {
                    let t = j as f32 / 10.0;
                    let next = clip(at(o, a + spread * (1.0 + t), 8.0 + reach * t));
                    let alpha = (1.0 - t * 0.85) * if k == 1 { 1.0 } else { 0.45 };
                    s = s.line(prev, next, if k == 1 { 3.0 } else { 1.6 } * (1.0 - t * 0.5), LineCap::Round, beam.alpha(alpha));
                    prev = next;
                }
            }
            s = s.fill(dot(clip(at(o, a, reach * 0.82)), 4.0), beam);
        }
        None => {
            // A swing: a soft wedge across every position.
            let (first, last) = (angles[0], *angles.last().unwrap_or(&angles[0]));
            let mut wedge = vec![o];
            for j in 0..=24 {
                wedge.push(clip(at(o, first + (last - first) * j as f32 / 24.0, reach * 0.9)));
            }
            s = s.fill(wedge, beam.alpha(if on { 0.16 } else { 0.08 })).line(o, clip(at(o, first, reach * 0.9)), 1.4, LineCap::Round, beam.alpha(0.5)).line(
                o,
                clip(at(o, last, reach * 0.9)),
                1.4,
                LineCap::Round,
                beam.alpha(0.5),
            );
        }
    }
    canvas(s).size(W, H)
}

fn dot(c: [f32; 2], r: f32) -> Vec<[f32; 2]> {
    (0..20)
        .map(|i| {
            let a = i as f32 / 20.0 * std::f32::consts::TAU;
            [c[0] + r * a.cos(), c[1] + r * a.sin()]
        })
        .collect()
}

fn position_label(v: &str) -> String {
    if is_moving(v) { crate::thinq::model::label(v) } else { v.trim_start_matches("POSITION_").to_string() }
}

/// A louvre the unit can park: the picture (click or drag to aim) over a segmented choice.
pub fn vane(s: &App, p: P, v: &Vane) -> El {
    let state = s.state.clone().unwrap_or_default();
    let current = state.str("windDirection", &v.key).unwrap_or("").to_string();
    let angles: Vec<String> = v.angles().into_iter().map(str::to_string).collect();
    let pos = angles.iter().position(|a| *a == current);
    let swing = v.positions.iter().find(|x| x.eq_ignore_ascii_case("SWING")).cloned();
    let (side, n) = (v.side, angles.len());

    let key = v.key.clone();
    let aims = angles.clone();
    let pic = el()
        .key(format!("vane-{}", v.key))
        .size(W, H)
        .role("none")
        .cursor(rune::Cursor::Pointer)
        .on_drag(move |s: &mut App, d, _| {
            if n == 0 || !matches!(d.phase, rune::Phase::Down | rune::Phase::Up) {
                return;
            }
            let i = nearest(side, n, d.x, d.y);
            let want = aims[i].clone();
            if s.state.as_ref().and_then(|st| st.str("windDirection", &key)) != Some(want.as_str()) {
                s.send(Command::Vane(key.clone(), want));
            }
        })
        .child(picture(p, side, n, pos, s.power()));

    let labels: Vec<String> = angles.iter().map(|x| position_label(x)).collect();
    let values = angles.clone();
    let key = v.key.clone();
    let mut picker = SegmentedControl::new(&labels, pos)
        .label(if side { "Left–right position" } else { "Up–down position" })
        .width(Some(if swing.is_some() { W - 70.0 } else { W }))
        .on_pick(move |s: &mut App, i, _| {
            if let Some(val) = values.get(i) {
                s.send(Command::Vane(key.clone(), val.clone()));
            }
        })
        .build(p.k);
    if let Some(sw) = swing.clone() {
        let swinging = current.eq_ignore_ascii_case(&sw);
        let key = v.key.clone();
        let button = Button::new("Swing")
            .variant(if swinging { ButtonVariant::Contrast } else { ButtonVariant::Neutral })
            .on_press(move |s: &mut App| s.send(Command::Vane(key.clone(), sw.clone())))
            .build(p.k)
            .w(64.0);
        picker = row().w(W).gap(6.0).center_y().child(picker).child(button);
    }
    let title = if side { "Left–right" } else { "Up–down" };
    let what = match (pos, swing.is_some() && current.eq_ignore_ascii_case("SWING")) {
        (Some(i), _) => format!("Position {} of {n}", i + 1),
        (None, true) => "Swinging".into(),
        (None, false) => crate::thinq::model::label(&current),
    };
    col()
        .key(format!("vane-card-{}", v.key))
        .gap(10.0)
        .items_center()
        .child(row().w(W).child(text(title).font(p.k.ty.ui_medium).color(p.text()).grow(1.0)).child(text(what).font(p.k.ty.ui).color(p.muted())))
        .child(pic)
        .child(picker)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aiming_picks_the_nearest_position() {
        let (o, angles) = geometry(false, 6);
        for (i, a) in angles.iter().enumerate() {
            let pt = at(o, *a, 60.0);
            assert_eq!(nearest(false, 6, pt[0], pt[1]), i);
        }
        let (o, angles) = geometry(true, 5);
        let pt = at(o, angles[4], 40.0);
        assert_eq!(nearest(true, 5, pt[0], pt[1]), 4);
    }
}
