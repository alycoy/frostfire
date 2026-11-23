//! The pieces every pane shares, after Hex's settings panes: a section label over a raised card,
//! rows inside it split by hairlines, and label/value detail rows.

use rune::tok::TextStyle;
use rune::{Color, El, Icon, col, el, icon, row, text};
use rune_kit::Kit;

use crate::app::App;
use crate::ui::theme::Mood;

/// The page's left and right text edge.
pub const GUTTER: f32 = 28.0;
/// A row's text inset inside its card.
pub const ROW_PX: f32 = 16.0;

/// Everything a view needs to paint itself.
#[derive(Clone, Copy)]
pub struct P {
    pub k: &'static Kit,
    pub m: Mood,
    pub hair: f32,
    pub dark: bool,
}

impl P {
    pub fn text(self) -> Color {
        self.k.c.text_base
    }
    pub fn muted(self) -> Color {
        self.k.c.text_muted
    }
    pub fn faint(self) -> Color {
        self.k.c.text_faint
    }
    /// An opaque neutral a little off the card: dial tracks and bar wells.
    pub fn well(self) -> Color {
        self.k.c.bg_base.mix(self.k.c.text_base, if self.dark { 0.12 } else { 0.07 }).alpha(1.0)
    }
}

/// A column of sections, inset so the cards' row text lands on the gutter.
pub fn page(key: &str) -> El {
    col().key(key.to_string()).w_full().px(GUTTER - ROW_PX).pt(4.0).pb(28.0).gap(22.0)
}

/// A section: its label (and an optional note on the right) over a card.
pub fn section(p: P, key: &str, title: &str, note: Option<String>, body: El) -> El {
    let head = row()
        .key("head")
        .px(ROW_PX)
        .h(18.0)
        .center_y()
        .child(text(title.to_string()).role("heading").level(2).font(p.k.ty.ui_medium).color(p.muted()).grow(1.0))
        .children(note.map(|n| text(n).font(p.k.ty.ui).color(p.faint())));
    col().key(key.to_string()).w_full().gap(8.0).child(head).child(body)
}

/// A raised card of rows, a hairline between each pair.
pub fn card(p: P, rows: Vec<El>) -> El {
    let mut surface = col().w_full().fill(p.k.c.bg_base).radius(p.k.radius.lg);
    for shadow in &p.k.elevation.raised {
        surface = surface.box_shadow(*shadow);
    }
    for (i, r) in rows.into_iter().enumerate() {
        if i > 0 {
            surface = surface.child(el().key(format!("sep{i}")).h(p.hair).mx(ROW_PX).fill(p.k.c.border_muted).a11y_hidden());
        }
        surface = surface.child(r);
    }
    surface
}

/// A setting: an optional icon, the title and an optional description on the left, the control on the right.
pub fn setting(p: P, key: &str, glyph: Option<&'static Icon>, title: &str, description: Option<String>, control: El) -> El {
    let described = description.is_some();
    let copy = col()
        .grow(1.0)
        .shrink()
        .min_w(0.0)
        .gap(2.0)
        .child(text(title.to_string()).font(p.k.ty.ui_medium).color(p.text()).truncate())
        .children(description.map(|d| text(d).font(p.k.ty.ui).color(p.muted()).wrap()));
    row()
        .key(key.to_string())
        .w_full()
        .min_h(if described { 60.0 } else { 48.0 })
        .px(ROW_PX)
        .py(10.0)
        .gap(12.0)
        .center_y()
        .children(glyph.map(|g| icon(g, 16.0).color(p.k.c.icon_muted).a11y_hidden()))
        .child(copy)
        .child(control)
}

/// A row holding one wide control (a segmented choice, a chart).
pub fn wide(key: &str, child: El) -> El {
    col().key(key.to_string()).w_full().px(12.0).py(12.0).child(child)
}

/// A label/value line of a detail table.
pub fn detail(p: P, label: &str, value: String) -> El {
    row()
        .key(format!("detail-{label}"))
        .w_full()
        .h(34.0)
        .center_y()
        .gap(12.0)
        .child(text(label.to_string()).font(p.k.ty.ui).color(p.muted()).w(110.0))
        .child(text(value).font(p.k.ty.ui).color(p.text()).truncate().grow(1.0))
}

/// A hairline between detail lines.
pub fn hairline(p: P) -> El {
    el().h(p.hair).w_full().fill(p.k.c.border_muted).a11y_hidden()
}

/// A quiet outline pill ("Demo").
pub fn badge(p: P, label: &str) -> El {
    row()
        .key(format!("badge-{label}"))
        .h(18.0)
        .px(7.0)
        .gap(5.0)
        .center_y()
        .radius(9.0)
        .border(p.hair.max(0.5), p.k.c.border_base)
        .child(el().size(5.0, 5.0).radius(2.5).fill(p.m.accent))
        .child(text(label.to_string()).font(p.k.ty.small).color(p.muted()))
}

pub fn heading(size: f32, weight: f32) -> TextStyle {
    TextStyle::new(size, weight).tracking(-0.2 * size / 13.0)
}

/// The panel footer's status line.
pub fn status(s: &App) -> String {
    let link = match (s.demo, s.offline, s.in_flight.is_empty()) {
        (_, _, false) => "Sending…",
        (true, _, _) => "Demo · nothing leaves this computer",
        (false, true, _) => "Offline · retrying",
        (false, false, _) => "Connected to LG ThinQ",
    };
    match s.updated_at {
        Some(at) if s.in_flight.is_empty() => {
            let ago = (s.clock - at).max(0.0);
            let when = if ago < 60.0 { "just now".to_string() } else { format!("{} min ago", (ago / 60.0).round()) };
            format!("{link} · updated {when}")
        }
        _ => link.to_string(),
    }
}
