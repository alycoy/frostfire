//! The window, laid out like Hex: a sidebar of air conditioners and Settings, and a raised
//! panel with the chosen one. Neutral throughout; the unit's mode tints only the dial, its icon
//! and the energy bars.

pub mod airflow;
pub mod control;
pub mod dial;
pub mod icons;
pub mod parts;
pub mod settings;
pub mod theme;
pub mod welcome;

use rune::{Cx, El, Key, KeyEvent, ScrollSpec, Tx, col, el, icon, row, text};
use rune_kit::{IconButton, IconButtonVariant, Switch};

use crate::app::{App, Phase, fmt_temp};
use crate::store::Effects;
use parts::P;
use theme::{Mood, mood};

/// Room for the traffic lights above the sidebar's first row.
const SIDEBAR_TOP: f32 = 44.0;

/// The mood for the app as it is now.
pub fn current_mood(s: &App) -> Mood {
    match s.phase {
        Phase::Ready if s.state.is_some() => mood(s.mode(), s.power()),
        _ => mood(Some("COOL"), true),
    }
}

fn nav_item(p: P, key: String, glyph: &'static rune::Icon, tint: Option<rune::Color>, title: String, trailing: Option<String>, on: bool) -> El {
    let c = &p.k.c;
    let mut item = row()
        .key(key)
        .role("listitem")
        .label(title.clone())
        .h(30.0)
        .px(8.0)
        .gap(8.0)
        .center_y()
        .radius(p.k.radius.sm)
        .hover(c.hover)
        .press(c.pressed)
        .focusable(true)
        .focus_ring(2.0, -2.0, c.border_focus)
        .child(icon(glyph, 16.0).color(tint.unwrap_or(if on { c.icon_base } else { c.icon_muted })).a11y_hidden())
        .child(text(title).font(p.k.ty.ui).color(if on { c.text_base } else { c.text_muted }).truncate().grow(1.0).a11y_hidden())
        .children(trailing.map(|t| text(t).font(p.k.ty.small).color(c.text_faint).a11y_hidden()));
    if on {
        item = item.fill(c.layer_02);
    }
    item
}

fn sidebar(s: &App, p: P) -> El {
    let mut list = col().key("nav").gap(2.0).child(text("Air conditioners").font(p.k.ty.small_medium).color(p.faint()).px(8.0).pb(6.0));
    for d in s.acs() {
        let on = !s.settings && s.device.as_deref() == Some(d.id.as_str());
        let name = if d.alias.is_empty() { d.model.clone() } else { d.alias.clone() };
        let (glyph, tint, trailing) = if s.device.as_deref() == Some(d.id.as_str()) && s.state.is_some() {
            let running = s.power();
            let m = mood(s.mode(), running);
            let glyph = if running { s.mode().map(icons::for_mode).unwrap_or(&icons::POWER) } else { &icons::POWER };
            let trailing = if running { s.room().map(|r| format!("{}°", fmt_temp(r))) } else { Some("Off".into()) };
            (glyph, running.then_some(m.accent), trailing)
        } else {
            (&icons::SNOW, None, None)
        };
        let id = d.id.clone();
        list = list.child(nav_item(p, format!("ac-{}", d.id), glyph, tint, name, trailing, on).on_press(move |s: &mut App| {
            s.settings = false;
            if s.device.as_deref() != Some(id.as_str()) {
                s.select(Some(id.clone()));
            }
        }));
    }
    let settings = nav_item(p, "nav-settings".into(), &icons::GEAR, None, "Settings".into(), None, s.settings).on_press(|s: &mut App| s.settings = true);
    let quit = text("Quit Frostfire")
        .key("quit")
        .role("button")
        .label("Quit Frostfire")
        .font(p.k.ty.ui)
        .color(p.muted())
        .px(8.0)
        .py(4.0)
        .radius(p.k.radius.sm)
        .hover(p.k.c.hover)
        .focusable(true)
        .on_press_tx(|_: &mut App, tx: &mut Tx| tx.quit());
    let version = text(format!("Version {}{}", env!("CARGO_PKG_VERSION"), if s.demo { " · demo" } else { "" })).font(p.k.ty.small).color(p.faint()).px(8.0);
    col()
        .key("sidebar")
        .role("navigation")
        .w(212.0)
        .h_full()
        .pt(SIDEBAR_TOP)
        .pb(14.0)
        .pl(10.0)
        .pr(8.0)
        .window_drag()
        .child(list.no_window_drag())
        .child(el().grow(1.0))
        .child(col().gap(4.0).no_window_drag().child(settings).child(quit.mt(6.0)).child(version))
}

fn header(s: &App, p: P) -> El {
    let title = if s.settings {
        "Settings".to_string()
    } else {
        s.current_device().map(|d| if d.alias.is_empty() { d.model.clone() } else { d.alias.clone() }).unwrap_or_else(|| "Frostfire".into())
    };
    let mut bar = row()
        .key("pane-header")
        .h(52.0)
        .px(parts::GUTTER)
        .gap(10.0)
        .center_y()
        .window_drag()
        .child(text(title).role("heading").level(1).font(p.k.ty.body_medium).color(p.text()).truncate().shrink());
    if !s.settings {
        if let Some(d) = s.current_device() {
            bar = bar.child(text(d.model.clone()).font(p.k.ty.ui).color(p.faint()).truncate().shrink());
        }
        if s.demo {
            bar = bar.child(parts::badge(p, "Demo"));
        }
    }
    bar = bar.child(el().grow(1.0));
    if !s.settings && s.profile.is_some() {
        bar = bar.child(
            IconButton::new(&icons::REFRESH)
                .variant(IconButtonVariant::Ghost)
                .label("Refresh")
                .help("Refresh (⌘R)")
                .on_press(|s: &mut App| s.refresh())
                .build(p.k)
                .no_window_drag(),
        );
        if s.profile.as_ref().is_some_and(|pr| pr.can_power()) {
            bar = bar.child(Switch::new(s.power()).label("Power").on_toggle(|s: &mut App| s.toggle_power()).build(p.k).no_window_drag());
        }
    }
    bar
}

fn toast(s: &App, p: P) -> Option<El> {
    let t = s.toast.as_ref()?;
    let c = &p.k.c;
    Some(
        col().absolute(0.0, 0.0).w_full().h_full().justify_end().items_center().pb(22.0).inert().child(
            row()
                .key("toast")
                .max_w(380.0)
                .px(14.0)
                .py(9.0)
                .radius(p.k.radius.md)
                .fill(if t.error { c.danger.bg } else { c.bg_inverse })
                .border(p.hair, if t.error { c.danger.border } else { c.bg_inverse })
                .shadow(6.0, 24.0, rune::Color::BLACK.alpha(0.18))
                .live_region(rune::LiveRegion::Polite)
                .transition(rune::Transition::opacity().combined(rune::Transition::offset(0.0, 8.0)))
                .child(text(t.text.clone()).font(p.k.ty.ui).color(if t.error { c.danger.fg } else { c.text_inverse }).wrap()),
        ),
    )
}

fn scroll(key: &str, content: El) -> El {
    col().key(key.to_string()).w_full().grow(1.0).min_h(0.0).scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(16.0).child(content)
}

pub fn view(s: &App, cx: &mut Cx<App>) -> El {
    let dark = cx.system_dark();
    let p = P { k: rune_theme_opencode::kit(dark), m: current_mood(s), hair: 1.0 / cx.env().scale.max(1.0), dark };
    let effects = s.config.effects != Effects::Off && !cx.is_reduced_motion();
    let c = &p.k.c;

    let window = if s.phase != Phase::Ready {
        // First run: one centred card, nothing else.
        col()
            .key("window")
            .w_full()
            .h_full()
            .fill(c.bg_deep)
            .child(el().h(40.0).w_full().window_drag())
            .child(scroll("welcome-scroll", col().w_full().items_center().pt(8.0).pb(32.0).child(welcome::view(s, p))))
    } else {
        let body = if s.settings { settings::view(s, p) } else { control::view(s, p, effects) };
        let panel = col()
            .key("panel")
            .role("main")
            .grow(1.0)
            .shrink()
            .min_w(0.0)
            .mt(8.0)
            .mr(8.0)
            .mb(8.0)
            .radius(p.k.radius.lg)
            .fill(c.bg_deep.mix(c.bg_base, 0.55))
            .border(p.hair, c.border_weak)
            .padding(p.hair)
            .clip_soft(0.0)
            .child(header(s, p))
            .child(scroll("pane-scroll", body));
        row().key("window").w_full().h_full().fill(c.bg_deep).child(sidebar(s, p)).child(panel)
    };
    let mut root = el().key("root").w_full().h_full().child(window);
    if let Some(t) = toast(s, p) {
        root = root.child(t);
    }
    root
}

/// Keys nothing focused took.
pub fn on_key(s: &mut App, e: &KeyEvent, tx: &mut Tx) -> bool {
    if e.is_command('q') {
        tx.quit();
        return true;
    }
    if e.is_command('w') {
        tx.close_window();
        return true;
    }
    if s.phase != Phase::Ready {
        return false;
    }
    if e.is_command('r') {
        s.refresh();
        return true;
    }
    if e.is_command(',') {
        s.settings = !s.settings;
        return true;
    }
    if tx.focused().is_some() && !matches!(e.key, Key::Escape) {
        return false;
    }
    match &e.key {
        Key::Escape if s.settings => s.settings = false,
        Key::Up if !s.settings => s.nudge(1.0),
        Key::Down if !s.settings => s.nudge(-1.0),
        Key::Text(t) if (t == "p" || t == "P") && !s.settings && !e.mods.cmd => s.toggle_power(),
        _ => return false,
    }
    true
}
