//! The menu bar item, the app's main home, as in Hex: the room temperature beside an icon
//! for the mode, and a menu that runs the unit (power, warmer and cooler, mode, fan speed,
//! timers, extras, which unit) without opening the window.
//!
//! How it stays simple and safe:
//! - The app builds a plain [`MenuModel`] from its state on every poll. Only when the model
//!   changes is the native menu rebuilt, so AppKit never reads app state.
//! - Each menu item carries a tag into the list of [`MenuAction`]s made with the menu. A pick
//!   becomes an `AppHandle::update`, so app state is only touched on Rune's side.
//! - This is the only file with `unsafe` (AppKit through objc2), and all of it runs on the
//!   main thread, checked with `MainThreadMarker`.

use crate::app::{App, Phase, fmt_minutes, fmt_temp};
use crate::thinq::model::{Timer, label};

static HANDLE: std::sync::OnceLock<rune::AppHandle<App>> = std::sync::OnceLock::new();

/// Gives the menu its way back into the app. Call once, before `run`.
pub fn install(handle: rune::AppHandle<App>) {
    let _ = HANDLE.set(handle);
}

/// What a menu pick asks the app to do.
#[derive(Clone, Debug, PartialEq)]
pub enum MenuAction {
    Open,
    Settings,
    Quit,
    Power,
    Warmer,
    Cooler,
    Mode(String),
    Fan(String),
    TimerIn(Timer, u32),
    TimerCancel(Timer),
    Eco(bool),
    Swing(bool),
    Device(String),
    Refresh,
}

/// One row of the menu.
#[derive(Clone, Debug, PartialEq)]
pub enum Row {
    /// Greyed-out information.
    Info(String),
    Action {
        title: String,
        action: MenuAction,
        checked: bool,
        key: &'static str,
    },
    Sub {
        title: String,
        rows: Vec<Row>,
    },
    Separator,
}

/// Everything the menu bar shows, as plain data.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MenuModel {
    /// SF Symbol for the button.
    pub symbol: &'static str,
    /// Text beside it.
    pub title: String,
    pub rows: Vec<Row>,
}

fn act(title: impl Into<String>, action: MenuAction) -> Row {
    Row::Action { title: title.into(), action, checked: false, key: "" }
}

fn check(title: impl Into<String>, action: MenuAction, checked: bool) -> Row {
    Row::Action { title: title.into(), action, checked, key: "" }
}

fn symbol_for(mode: Option<&str>, on: bool) -> &'static str {
    if !on {
        return "power";
    }
    match mode {
        Some("HEAT") => "flame",
        Some("AUTO") => "thermometer.medium",
        Some("AIR_DRY") => "drop",
        Some("FAN") => "fan",
        Some("AIR_CLEAN") => "sparkles",
        _ => "snowflake",
    }
}

/// The menu for the app as it is now.
pub fn model(app: &App) -> MenuModel {
    let tail = [
        Row::Separator,
        Row::Action { title: "Open Frostfire".into(), action: MenuAction::Open, checked: false, key: "o" },
        Row::Action { title: "Settings…".into(), action: MenuAction::Settings, checked: false, key: "," },
        Row::Separator,
        Row::Action { title: "Quit Frostfire".into(), action: MenuAction::Quit, checked: false, key: "q" },
    ];
    let (Phase::Ready, Some(state)) = (app.phase, app.state.as_ref()) else {
        let info = match app.phase {
            Phase::Starting | Phase::SigningIn => "Connecting to LG ThinQ…",
            Phase::Welcome => "Not connected",
            Phase::Ready if app.devices.is_empty() => "No devices found",
            Phase::Ready => "Loading…",
        };
        let mut rows = vec![Row::Info(info.into())];
        rows.extend(tail);
        return MenuModel { symbol: "house", title: String::new(), rows };
    };
    let Some(profile) = app.profile.as_ref() else { return generic(app, tail) };

    let on = app.power();
    let mode = app.mode();
    let room = app.room();
    let target = app.target();
    let name = app.current_device().map(|d| d.name()).unwrap_or_default();
    let doing = match (on, mode) {
        (false, _) => "Off".to_string(),
        (true, Some(m)) => label(m),
        (true, None) => "On".into(),
    };
    let mut rows = vec![Row::Info(format!("{name} · {doing}"))];
    let mut line = Vec::new();
    if let Some(r) = room {
        line.push(format!("Room {}°", fmt_temp(r)));
    }
    if let Some(t) = target {
        line.push(format!("Target {}°", fmt_temp(t)));
    }
    if !line.is_empty() {
        rows.push(Row::Info(line.join("  ·  ")));
    }
    rows.push(Row::Separator);

    if profile.can_power() {
        rows.push(Row::Action { title: if on { "Turn Off".into() } else { "Turn On".into() }, action: MenuAction::Power, checked: false, key: "" });
    }
    if app.target_spec().is_some() {
        rows.push(act("Warmer  +1°", MenuAction::Warmer));
        rows.push(act("Cooler  −1°", MenuAction::Cooler));
    }

    let modes = profile.modes();
    if !modes.is_empty() {
        let sub = modes.iter().map(|m| check(label(m), MenuAction::Mode(m.clone()), Some(m.as_str()) == mode)).collect();
        rows.push(Row::Sub { title: format!("Mode: {}", mode.map(label).unwrap_or_else(|| "—".into())), rows: sub });
    }
    let speeds = profile.wind_speeds();
    if !speeds.is_empty() {
        let cur = state.wind();
        let sub = speeds.iter().map(|s| check(label(s), MenuAction::Fan(s.clone()), Some(s.as_str()) == cur)).collect();
        rows.push(Row::Sub { title: format!("Fan: {}", cur.map(label).unwrap_or_else(|| "—".into())), rows: sub });
    }

    let mut timers = Vec::new();
    for (which, title) in [(Timer::Stop, "Turn off in"), (Timer::Sleep, "Sleep timer"), (Timer::Start, "Turn on in")] {
        if !profile.can_timer(which) || (which == Timer::Start) == on {
            continue;
        }
        let left = state.timer_minutes(which);
        let mut sub: Vec<Row> = [30u32, 60, 120, 240, 480].iter().map(|m| act(fmt_minutes(*m), MenuAction::TimerIn(which, *m))).collect();
        if left.is_some() && profile.can_cancel(which) {
            sub.push(Row::Separator);
            sub.push(act("Cancel", MenuAction::TimerCancel(which)));
        }
        let title = match left {
            Some(m) => format!("{title}: {} left", fmt_minutes(m)),
            None => title.to_string(),
        };
        timers.push(Row::Sub { title, rows: sub });
    }
    if !timers.is_empty() {
        rows.push(Row::Separator);
        rows.extend(timers);
    }

    let mut extras = Vec::new();
    if profile.writable("powerSave", "powerSaveEnabled") {
        let v = state.bool("powerSave", "powerSaveEnabled").unwrap_or(false);
        extras.push(check("Energy saver", MenuAction::Eco(!v), v));
    }
    if profile.wind_flags().iter().any(|(k, _)| *k == "rotateUpDown") {
        let v = state.bool("windDirection", "rotateUpDown").unwrap_or(false);
        extras.push(check("Swing", MenuAction::Swing(!v), v));
    }
    if !extras.is_empty() {
        rows.push(Row::Separator);
        rows.extend(extras);
    }

    rows.extend(devices(app));
    rows.push(Row::Separator);
    rows.push(act("Refresh", MenuAction::Refresh));
    rows.extend(tail);

    let title = match (on, room) {
        (_, Some(r)) => format!("{}°", fmt_temp(r.round())),
        (false, None) => "Off".into(),
        (true, None) => String::new(),
    };
    MenuModel { symbol: symbol_for(mode, on), title, rows }
}

/// The device switcher, when there's more than one.
fn devices(app: &App) -> Vec<Row> {
    let all = app.sorted_devices();
    if all.len() < 2 {
        return Vec::new();
    }
    let sub = all.iter().map(|d| check(d.name(), MenuAction::Device(d.id.clone()), app.device.as_deref() == Some(d.id.as_str()))).collect();
    vec![Row::Separator, Row::Sub { title: "Device".into(), rows: sub }]
}

/// The menu for any device but an air conditioner: what it's doing, power, its dial, and the
/// device switcher.
fn generic(app: &App, tail: [Row; 5]) -> MenuModel {
    let kind = app.kind().unwrap_or_else(|| crate::thinq::kinds::kind(""));
    let on = app.power();
    let name = app.current_device().map(|d| d.name()).unwrap_or_default();
    let run = app.spec.as_ref().and_then(|sp| sp.find("runState", "currentState")).and_then(|f| app.value(&f.path)).and_then(|v| v.as_str().map(label));
    let doing = run.unwrap_or_else(|| {
        if !app.has_power() {
            kind.name.to_string()
        } else if on {
            "On".into()
        } else {
            "Off".into()
        }
    });
    let mut rows = vec![Row::Info(format!("{name} · {doing}"))];
    let dial = app.dial();
    if let Some((_, Some(v), suffix)) = dial {
        rows.push(Row::Info(format!("Target {}{suffix}", fmt_temp(v))));
    }
    rows.push(Row::Separator);
    if app.has_power() {
        rows.push(act(if on { "Turn Off" } else { "Turn On" }, MenuAction::Power));
    }
    if let Some((r, _, suffix)) = dial {
        rows.push(act(format!("Raise  +{}{suffix}", fmt_temp(r.step)), MenuAction::Warmer));
        rows.push(act(format!("Lower  −{}{suffix}", fmt_temp(r.step)), MenuAction::Cooler));
    }
    rows.extend(devices(app));
    rows.push(Row::Separator);
    rows.push(act("Refresh", MenuAction::Refresh));
    rows.extend(tail);
    let symbol = if app.has_power() && !on { "power" } else { kind.symbol };
    MenuModel { symbol, title: String::new(), rows }
}

// Only the macOS menu calls it; elsewhere there is no menu bar item.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn run(action: MenuAction) {
    let Some(h) = HANDLE.get() else { return };
    match action {
        MenuAction::Open => h.open(),
        MenuAction::Quit => h.quit(),
        MenuAction::Settings => {
            h.update(|app, _| app.settings = true);
            h.open();
        }
        other => h.update(move |app, _| app.menu_action(other)),
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use std::cell::RefCell;

    use objc2::rc::Retained;
    use objc2::runtime::{NSObjectProtocol, Sel};
    use objc2::{MainThreadOnly, define_class, msg_send, sel};
    use objc2_app_kit::{NSCellImagePosition, NSImage, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem, NSVariableStatusItemLength};
    use objc2_foundation::{MainThreadMarker, NSObject, NSString};

    use super::{MenuAction, MenuModel, Row};

    define_class!(
        #[unsafe(super = NSObject)]
        #[thread_kind = MainThreadOnly]
        struct MenuTarget;

        unsafe impl NSObjectProtocol for MenuTarget {}

        impl MenuTarget {
            #[unsafe(method(pick:))]
            fn pick(&self, sender: &NSMenuItem) {
                let Ok(i) = usize::try_from(sender.tag()) else { return };
                let action = ITEM.with(|c| c.borrow().as_ref().and_then(|it| it.actions.get(i).cloned()));
                if let Some(a) = action {
                    super::run(a);
                }
            }
        }
    );

    impl MenuTarget {
        fn new(mtm: MainThreadMarker) -> Retained<MenuTarget> {
            let this = MenuTarget::alloc(mtm).set_ivars(());
            // SAFETY: NSObject's designated initialiser on a freshly allocated instance.
            unsafe { msg_send![super(this), init] }
        }
    }

    struct Item {
        item: Retained<NSStatusItem>,
        menu: Retained<NSMenu>,
        target: Retained<MenuTarget>,
        actions: Vec<MenuAction>,
        shown: MenuModel,
    }

    thread_local! {
        static ITEM: RefCell<Option<Item>> = const { RefCell::new(None) };
    }

    fn new_item(title: &str, sel: Option<Sel>, key: &str, mtm: MainThreadMarker) -> Retained<NSMenuItem> {
        // SAFETY: a plain NSMenuItem made on the main thread.
        unsafe { NSMenuItem::initWithTitle_action_keyEquivalent(NSMenuItem::alloc(mtm), &NSString::from_str(title), sel, &NSString::from_str(key)) }
    }

    fn fill(menu: &NSMenu, rows: &[Row], target: &MenuTarget, actions: &mut Vec<MenuAction>, mtm: MainThreadMarker) {
        menu.setAutoenablesItems(false);
        for row in rows {
            match row {
                Row::Separator => menu.addItem(&NSMenuItem::separatorItem(mtm)),
                Row::Info(text) => {
                    let it = new_item(text, None, "", mtm);
                    it.setEnabled(false);
                    menu.addItem(&it);
                }
                Row::Action { title, action, checked, key } => {
                    let it = new_item(title, Some(sel!(pick:)), key, mtm);
                    it.setTag(actions.len() as isize);
                    // SAFETY: the target answers `pick:` and lives as long as the menu (both in `ITEM`).
                    unsafe { it.setTarget(Some(target)) };
                    it.setState(if *checked { 1 } else { 0 });
                    actions.push(action.clone());
                    menu.addItem(&it);
                }
                Row::Sub { title, rows } => {
                    let it = new_item(title, None, "", mtm);
                    let sub = NSMenu::initWithTitle(NSMenu::alloc(mtm), &NSString::from_str(title));
                    fill(&sub, rows, target, actions, mtm);
                    it.setSubmenu(Some(&sub));
                    menu.addItem(&it);
                }
            }
        }
    }

    fn symbol(name: &str) -> Option<Retained<NSImage>> {
        let img = NSImage::imageWithSystemSymbolName_accessibilityDescription(&NSString::from_str(name), Some(&NSString::from_str("Frostfire")))?;
        img.setTemplate(true);
        Some(img)
    }

    fn show(it: &mut Item, model: &MenuModel, mtm: MainThreadMarker) {
        if let Some(button) = it.item.button(mtm) {
            if it.shown.symbol != model.symbol || it.shown.rows.is_empty() {
                // Older macOS lacks some symbols (air.purifier): fall back so the item never goes blank.
                button.setImage(symbol(model.symbol).or_else(|| symbol("house")).as_deref());
                button.setImagePosition(NSCellImagePosition::ImageLeading);
            }
            let title = if model.title.is_empty() { String::new() } else { format!(" {}", model.title) };
            button.setTitle(&NSString::from_str(&title));
        }
        if it.shown.rows != model.rows {
            it.menu.removeAllItems();
            let mut actions = Vec::new();
            fill(&it.menu, &model.rows, &it.target, &mut actions, mtm);
            it.actions = actions;
        }
        it.shown = model.clone();
    }

    /// Shows the item (creating it the first time) or removes it.
    pub fn set_visible(on: bool) {
        let Some(mtm) = MainThreadMarker::new() else { return };
        ITEM.with(|cell| {
            let mut cell = cell.borrow_mut();
            match (on, cell.is_some()) {
                (true, false) => {
                    let item = NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
                    let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), &NSString::from_str("Frostfire"));
                    item.setMenu(Some(&menu));
                    let mut it = Item { item, menu, target: MenuTarget::new(mtm), actions: Vec::new(), shown: MenuModel::default() };
                    show(&mut it, &MenuModel { symbol: "snowflake", ..MenuModel::default() }, mtm);
                    *cell = Some(it);
                }
                (false, true) => {
                    if let Some(it) = cell.take() {
                        NSStatusBar::systemStatusBar().removeStatusItem(&it.item);
                    }
                }
                _ => {}
            }
        });
    }

    /// Brings the item up to date with `model`; cheap when nothing changed.
    pub fn update(model: &MenuModel) {
        let Some(mtm) = MainThreadMarker::new() else { return };
        ITEM.with(|cell| {
            if let Some(it) = cell.borrow_mut().as_mut()
                && it.shown != *model
            {
                show(it, model, mtm);
            }
        });
    }
}

#[cfg(target_os = "macos")]
pub use mac::{set_visible, update};

#[cfg(not(target_os = "macos"))]
pub fn set_visible(_: bool) {}

#[cfg(not(target_os = "macos"))]
pub fn update(_: &MenuModel) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demo;
    use crate::worker::Worker;

    fn ready_app() -> App {
        let mut app = App::new(Worker::detached(), None);
        app.phase = Phase::Ready;
        app.devices = demo::devices();
        app.device = Some(demo::LIVING.into());
        app.profile = Some(demo::profile(demo::LIVING));
        app.state = Some(demo::initial_state(demo::LIVING));
        app
    }

    fn titles(rows: &[Row]) -> Vec<String> {
        rows.iter()
            .filter_map(|r| match r {
                Row::Info(t) | Row::Sub { title: t, .. } | Row::Action { title: t, .. } => Some(t.clone()),
                Row::Separator => None,
            })
            .collect()
    }

    #[test]
    fn signed_out_menu_still_opens_and_quits() {
        let app = App::new(Worker::detached(), None);
        let m = model(&app);
        let t = titles(&m.rows);
        assert!(t.contains(&"Open Frostfire".to_string()) && t.contains(&"Quit Frostfire".to_string()));
    }

    #[test]
    fn ready_menu_has_the_controls() {
        let app = ready_app();
        let m = model(&app);
        assert_eq!(m.symbol, "snowflake");
        assert_eq!(m.title, "27°", "room 26.5 rounds up");
        let t = titles(&m.rows);
        for want in [
            "Living room · Cool",
            "Turn Off",
            "Warmer  +1°",
            "Mode: Cool",
            "Fan: Medium",
            "Turn off in: 1h 30m left",
            "Sleep timer",
            "Energy saver",
            "Swing",
            "Device",
        ] {
            assert!(t.iter().any(|x| x == want), "missing {want:?} in {t:?}");
        }
        assert!(!t.iter().any(|x| x.starts_with("Turn on in")), "no start timer while running");
    }

    #[test]
    fn mode_submenu_checks_the_current_mode() {
        let app = ready_app();
        let m = model(&app);
        let Some(Row::Sub { rows, .. }) = m.rows.iter().find(|r| matches!(r, Row::Sub { title, .. } if title.starts_with("Mode"))) else { panic!() };
        let checked: Vec<_> = rows.iter().filter(|r| matches!(r, Row::Action { checked: true, .. })).cloned().collect();
        assert_eq!(checked.len(), 1);
        assert!(matches!(&checked[0], Row::Action { action: MenuAction::Mode(m), .. } if m == "COOL"));
    }
}
