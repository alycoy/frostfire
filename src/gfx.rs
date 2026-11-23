//! The compositor: Rune draws the UI as usual, and every `El::effect` slot in it becomes one
//! rune-fx layer drawn in place, all of them on the dial: frost sparkles on its ring while
//! cooling, embers rising through it while heating, and a soft glow under the arc.
//!
//! When nothing moves (effects off, or the unit is off) it asks for no frames, so an idle
//! window costs nothing.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rune::compose::{Compositor, Frame};
use rune::render::{EffectPhase, EffectSite, KIND_EFFECT};
use rune_fx::*;

use crate::app::App;
use crate::ui::theme::{Weather, mood};

/// Frost: drifting sparkles. Data: [count, size, 0, 0].
pub const FROST: u32 = 2;
/// Embers rising from the bottom edge. Data: [intensity, height, 0, 0].
pub const EMBERS: u32 = 3;
/// A glow around the dial. Data: [weather, intensity, radius, 0].
pub const GLOW: u32 = 4;

pub fn weather_index(w: Weather) -> f32 {
    match w {
        Weather::Frost => 1.0,
        Weather::Fire => 2.0,
        Weather::Aurora => 3.0,
        Weather::Mist => 4.0,
        Weather::Breeze => 5.0,
        Weather::Still => 0.0,
    }
}

fn mood_for(i: f32) -> crate::ui::theme::Mood {
    let mode = match i as u32 {
        1 => "COOL",
        2 => "HEAT",
        3 => "AUTO",
        4 => "AIR_DRY",
        5 => "FAN",
        _ => return mood(None, false),
    };
    mood(Some(mode), true)
}

fn to_rgb(c: rune::Color) -> Rgb {
    let enc = |v: f32| (rune::color::srgb_encode(v.clamp(0.0, 1.0)) * 255.0).round() as u32;
    rgb(enc(c.r) << 16 | enc(c.g) << 8 | enc(c.b))
}

fn layer(kind: u32, d: [f32; 4], shape: Shape, t: f32) -> Option<FxLayer> {
    let fx = match kind {
        FROST => Fx::sparkles(SparkleParams {
            color: rgb(if d[2] > 0.5 { 0xe6f7ff } else { 0x7cc4f5 }),
            count: d[0] as u32,
            size: d[1],
            life: 2.6,
            intensity: 0.85,
            margin: 0.0,
            burst: None,
        }),
        EMBERS => Fx::fire(FireParams { style: FireStyle::Embers, intensity: d[0], height: d[1], embers: 28, scale: 26.0, ..FireParams::default() }),
        GLOW => {
            let m = mood_for(d[0]);
            Fx::glow(GlowParams {
                style: GlowStyle::Soft,
                color: to_rgb(m.accent),
                intensity: d[1],
                radius: d[2].max(8.0),
                breathe: 0.25,
                breathe_period: 5.0,
                ..Default::default()
            })
        }
        _ => return None,
    };
    Some(fx.on(shape).at(t))
}

pub struct Gfx {
    fx: Option<FxRenderer>,
    live: Arc<AtomicBool>,
}

impl Gfx {
    pub fn new(live: Arc<AtomicBool>) -> Gfx {
        Gfx { fx: None, live }
    }
}

impl Compositor<App> for Gfx {
    fn animating(&self) -> bool {
        self.live.load(Ordering::Relaxed)
    }

    fn render(&mut self, f: &mut Frame<'_, App>) {
        let (device, queue, size) = (f.device, f.queue, f.size);
        let t = f.rt.t as f32;
        let marks = &f.rt.marks;
        let mut at: Vec<u32> = Vec::new();
        let mut layers: Vec<FxLayer> = Vec::new();
        let mut slots: Vec<FxSlot> = Vec::new();
        for (n, i) in f.rt.instances.iter().enumerate() {
            if i.params[1] != KIND_EFFECT {
                continue;
            }
            let kind = marks.get(n).copied().unwrap_or(0) & 0xff;
            let k = i.params[3];
            let [x, y, w, h] = i.rect.map(|v| v / k);
            let r = i.radii.iter().fold(0.0f32, |a, v| a.max(v.abs())) / k;
            let Some(l) = layer(kind, i.uv, Shape::rounded(x, y, w, h, r), t) else { continue };
            at.push(n as u32);
            layers.push(l);
            slots.push(FxSlot::default().scale(k).clip(i.clip, i.fade));
        }
        let fx = self.fx.get_or_insert_with(|| FxRenderer::new(device, f.format, FxOptions { dither: true }));
        fx.set_srgb_blend(f.srgb_blend());
        fx.add_format(device, wgpu::TextureFormat::Rgba16Float);
        fx.prepare_with_slots(device, queue, &FxFrame { viewport: [size.0, size.1], scale: 1.0, layers: &layers, backdrop: None, sources: &[] }, &slots);
        let clear = f.rt.clear_color();
        f.ui.prepare(device, queue, &mut f.rt.text.atlas, &f.rt.instances, size);
        f.ui.draw_with_backdrop(f.encoder, f.target, clear, &mut |phase, s: &EffectSite<'_>| {
            let Some(k) = at.iter().position(|n| *n == s.index) else { return };
            if let EffectPhase::Draw(pass) = phase {
                fx.draw_layer_in(pass, s.format, s.origin, k);
            }
        });
    }
}
