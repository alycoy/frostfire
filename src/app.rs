//! The app's state, and everything that changes it: worker answers, user commands, the
//! refresh clock.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rune::{TextEdit, Tx};

use crate::store::{Config, Effects};
use crate::thinq::model::{Command, Device, Profile, State, Timer, Unit};
use crate::worker::{Event, Job, Worker};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Reading the Keychain and config.
    Starting,
    /// No token yet: the welcome screen.
    Welcome,
    /// Checking a token with LG.
    SigningIn,
    Ready,
}

pub struct Toast {
    pub text: String,
    pub error: bool,
    pub until: f64,
}

pub struct App {
    pub worker: Worker,
    pub handle: Option<rune::AppHandle<App>>,
    pub phase: Phase,
    pub demo: bool,
    pub config: Config,
    pub token: TextEdit,
    /// The chosen country's code.
    pub country: TextEdit,
    /// What's typed in the country search.
    pub country_query: TextEdit,
    pub sign_in_error: Option<String>,
    pub devices: Vec<Device>,
    pub device: Option<String>,
    pub profile: Option<Profile>,
    pub state: Option<State>,
    pub energy: Vec<(String, f64)>,
    /// A target temperature being chosen (dial drag, ± presses), sent once it rests.
    pub pending_target: Option<(f64, f64)>,
    pub seq: u64,
    pub in_flight: Vec<u64>,
    pub clock: f64,
    pub next_refresh: f64,
    pub updated_at: Option<f64>,
    pub loading: bool,
    pub toast: Option<Toast>,
    pub settings: bool,
    /// Tells the compositor whether anything moves.
    pub fx_live: Arc<AtomicBool>,
    /// Decodes the logo off the UI thread.
    pub images: Option<rune::ImageCache>,
    pub logo: Option<rune::Image>,
    pub menu: crate::status_item::MenuModel,
    pub offline: bool,
}

/// How long a target rests before it's sent.
const DEBOUNCE: f64 = 0.7;
/// After a change, look again this soon to pick up what the unit really did.
const CONFIRM_AFTER: f64 = 4.0;

impl App {
    pub fn new(worker: Worker, handle: Option<rune::AppHandle<App>>) -> App {
        App {
            worker,
            handle,
            phase: Phase::Starting,
            demo: false,
            config: Config::default(),
            token: TextEdit::new(""),
            country: TextEdit::new(default_country()),
            country_query: TextEdit::new(""),
            sign_in_error: None,
            devices: Vec::new(),
            device: None,
            profile: None,
            state: None,
            energy: Vec::new(),
            pending_target: None,
            seq: 0,
            in_flight: Vec::new(),
            clock: 0.0,
            next_refresh: f64::INFINITY,
            updated_at: None,
            loading: false,
            toast: None,
            settings: false,
            fx_live: Arc::new(AtomicBool::new(false)),
            images: None,
            logo: None,
            menu: Default::default(),
            offline: false,
        }
    }

    pub fn unit(&self) -> Unit {
        Unit::parse(&self.config.unit).unwrap_or(Unit::C)
    }

    pub fn acs(&self) -> Vec<&Device> {
        self.devices.iter().filter(|d| d.is_ac()).collect()
    }

    pub fn current_device(&self) -> Option<&Device> {
        let id = self.device.as_ref()?;
        self.devices.iter().find(|d| &d.id == id)
    }

    pub fn power(&self) -> bool {
        self.state.as_ref().and_then(State::power).unwrap_or(false)
    }

    pub fn mode(&self) -> Option<&str> {
        self.state.as_ref().and_then(State::mode)
    }

    /// The target key, range and unit the dial works in.
    pub fn target_spec(&self) -> Option<(&'static str, crate::thinq::model::Range, Unit)> {
        let p = self.profile.as_ref()?;
        let want = self.unit();
        let unit = p.control_unit(want)?;
        let key = p.target_key(unit, self.mode())?;
        let r = p.target_range(unit, key)?;
        if unit == want {
            return Some((key, r, unit));
        }
        // Show it in the preferred unit, converted.
        let conv = |v| Unit::convert(v, unit, want);
        let step = if want == Unit::F { 1.0 } else { 0.5 };
        Some((key, crate::thinq::model::Range { min: conv(r.min).round(), max: conv(r.max).round(), step }, want))
    }

    /// The target to show: the one being chosen, else the unit's.
    pub fn target(&self) -> Option<f64> {
        if let Some((v, _)) = self.pending_target {
            return Some(v);
        }
        let (key, _, unit) = self.target_spec()?;
        let s = self.state.as_ref()?;
        s.temp(key, unit).or_else(|| s.temp("targetTemperature", unit))
    }

    pub fn room(&self) -> Option<f64> {
        self.state.as_ref()?.temp("currentTemperature", self.unit())
    }

    pub fn effects_moving(&self) -> bool {
        self.config.effects != Effects::Off
            && self.phase == Phase::Ready
            && self.power()
            && !self.settings
            && matches!(self.mode(), Some("COOL" | "HEAT" | "AUTO" | "AIR_DRY" | "FAN"))
    }

    // ------------------------------------------------------------------ worker answers

    pub fn on_event(&mut self, ev: Event, tx: &mut Tx) {
        match ev {
            Event::Restored { config, has_token } => {
                self.config = config;
                if let Some(c) = &self.config.country {
                    self.country = TextEdit::new(c);
                }
                self.phase = if has_token && self.config.country.is_some() { Phase::SigningIn } else { Phase::Welcome };
                self.apply_shell_prefs();
            }
            Event::SignedIn { devices, demo } => {
                self.demo = demo;
                if !demo && self.config.country.is_none() {
                    self.config.country = crate::thinq::Country::parse(&self.country.text).map(|c| c.code().to_string());
                }
                self.phase = Phase::Ready;
                self.sign_in_error = None;
                self.token = TextEdit::new("");
                self.set_devices(devices);
            }
            Event::SignInFailed(msg) => {
                self.phase = Phase::Welcome;
                self.sign_in_error = Some(msg);
            }
            Event::SignedOut => {
                self.reset_account();
                self.phase = Phase::Welcome;
                self.say("Signed out. The token was removed from your Keychain.", false);
            }
            Event::Devices(devices) => self.set_devices(devices),
            Event::Loaded { device, profile, state } => {
                if self.device.as_deref() == Some(device.as_str()) {
                    self.profile = Some(profile);
                    self.state = Some(state);
                    self.loading = false;
                    self.offline = false;
                    self.updated_at = Some(self.clock);
                    self.schedule(self.interval());
                }
            }
            Event::State { device, state } => {
                // While changes are still in flight, the unit may report the old values.
                if self.device.as_deref() == Some(device.as_str()) && self.in_flight.is_empty() && self.pending_target.is_none() {
                    self.state = Some(state);
                    self.offline = false;
                    self.updated_at = Some(self.clock);
                }
            }
            Event::Energy { device, days } => {
                if self.device.as_deref() == Some(device.as_str()) {
                    self.energy = days;
                }
            }
            Event::ControlDone { seq } => {
                self.in_flight.retain(|s| *s != seq);
                if self.in_flight.is_empty() {
                    self.schedule(CONFIRM_AFTER);
                }
            }
            Event::ControlFailed { seq, message } => {
                self.in_flight.retain(|s| *s != seq);
                self.say(&message, true);
                // Show what the unit really has.
                self.schedule(0.0);
            }
            Event::Failed { message, auth } => {
                self.loading = false;
                if auth {
                    self.worker.send(Job::SignOut);
                    self.sign_in_error = Some(message);
                } else {
                    self.offline = true;
                    self.say(&message, true);
                    self.schedule(self.interval().max(60.0));
                }
            }
        }
        let now = tx.clock();
        tx.wake_at(now);
    }

    fn set_devices(&mut self, devices: Vec<Device>) {
        self.devices = devices;
        let acs = self.acs();
        let keep = self.device.clone().or(self.config.device.clone()).filter(|id| acs.iter().any(|d| &d.id == id));
        let pick = keep.or_else(|| acs.first().map(|d| d.id.clone()));
        if pick != self.device || self.profile.is_none() {
            self.select(pick);
        }
    }

    pub fn select(&mut self, device: Option<String>) {
        self.device = device.clone();
        self.profile = None;
        self.state = None;
        self.energy.clear();
        self.pending_target = None;
        self.in_flight.clear();
        if let Some(id) = device {
            self.loading = true;
            self.worker.send(Job::Load { device: id.clone() });
            if !self.demo {
                self.config.device = Some(id);
                self.save();
            }
        }
    }

    fn reset_account(&mut self) {
        self.devices.clear();
        self.select(None);
        self.demo = false;
        self.next_refresh = f64::INFINITY;
    }

    // ------------------------------------------------------------------ user actions

    /// Chooses a country from the search.
    pub fn pick_country(&mut self, code: &str) {
        self.country = TextEdit::new(code);
        self.country_query = TextEdit::new("");
    }

    pub fn sign_in(&mut self) {
        let token = self.token.text.trim().to_string();
        let country = self.country.text.trim().to_ascii_uppercase();
        // The field is cleared at once; the worker owns the token from here.
        self.token = TextEdit::new("");
        if !crate::thinq::client::valid_token(&token) {
            self.sign_in_error = Some("Paste the whole token from connect-pat.lgthinq.com.".into());
            return;
        }
        if crate::thinq::Country::parse(&country).is_none() {
            self.sign_in_error = Some("Pick the country your LG account is in.".into());
            return;
        }
        self.sign_in_error = None;
        self.phase = Phase::SigningIn;
        self.worker.send(Job::SignIn { token: crate::thinq::Secret::new(token), country });
    }

    pub fn try_demo(&mut self) {
        self.sign_in_error = None;
        self.phase = Phase::SigningIn;
        self.worker.send(Job::Demo);
    }

    pub fn sign_out(&mut self) {
        self.settings = false;
        if self.demo {
            self.reset_account();
            self.phase = Phase::Welcome;
        } else {
            self.worker.send(Job::SignOut);
        }
    }

    pub fn refresh(&mut self) {
        if let Some(id) = self.device.clone() {
            self.worker.send(Job::Refresh { device: id });
            self.schedule(self.interval());
        } else if self.phase == Phase::Ready {
            self.worker.send(Job::Devices);
        }
    }

    pub fn send(&mut self, cmd: Command) {
        let (Some(device), Some(profile), Some(state)) = (self.device.clone(), self.profile.as_ref(), self.state.as_mut()) else {
            return;
        };
        match cmd.payload(profile, state) {
            Ok(payload) => {
                state.apply(&cmd.optimistic(&payload));
                self.seq += 1;
                self.in_flight.push(self.seq);
                self.worker.send(Job::Control { device, payload, seq: self.seq });
            }
            Err(why) => self.say(why, true),
        }
    }

    pub fn toggle_power(&mut self) {
        let on = !self.power();
        self.send(Command::Power(on));
    }

    /// Nudges the target by whole steps; sent after it rests.
    pub fn nudge(&mut self, steps: f64) {
        let Some((_, range, _)) = self.target_spec() else { return };
        let now = self.target().unwrap_or((range.min + range.max) / 2.0);
        self.choose_target(now + steps * range.step);
    }

    pub fn choose_target(&mut self, v: f64) {
        let Some((_, range, _)) = self.target_spec() else { return };
        self.pending_target = Some((range.snap(v), self.clock + DEBOUNCE));
    }

    pub fn set_timer(&mut self, which: Timer, minutes: Option<u32>) {
        match minutes {
            Some(m) => self.send(Command::TimerIn(which, m)),
            None => self.send(Command::TimerCancel(which)),
        }
    }

    pub fn set_config(&mut self, f: impl FnOnce(&mut Config)) {
        f(&mut self.config);
        self.config = self.config.clone().sanitized();
        self.apply_shell_prefs();
        self.save();
    }

    fn save(&self) {
        if !self.demo || self.config.country.is_some() {
            self.worker.send(Job::SaveConfig(self.config.clone()));
        }
    }

    /// Dock icon and menu bar item, as the settings say. At least one always shows, so the
    /// app can't become unreachable.
    pub fn apply_shell_prefs(&mut self) {
        if !self.config.dock_icon && !self.config.menu_bar {
            self.config.menu_bar = true;
        }
        if let Some(h) = &self.handle {
            h.set_dock_icon(self.config.dock_icon);
        }
        crate::status_item::set_visible(self.config.menu_bar && self.handle.is_some());
        self.menu = Default::default();
    }

    /// A pick from the menu bar menu.
    pub fn menu_action(&mut self, action: crate::status_item::MenuAction) {
        use crate::status_item::MenuAction as A;
        match action {
            A::Power => self.toggle_power(),
            A::Warmer => self.nudge_now(1.0),
            A::Cooler => self.nudge_now(-1.0),
            A::Mode(m) => self.send(Command::Mode(m)),
            A::Fan(f) => self.send(Command::Wind(f)),
            A::TimerIn(w, m) => self.set_timer(w, Some(m)),
            A::TimerCancel(w) => self.set_timer(w, None),
            A::Eco(on) => self.send(Command::PowerSave(on)),
            A::Swing(on) => self.send(Command::WindFlag("rotateUpDown".into(), on)),
            A::Device(id) => self.select(Some(id)),
            A::Refresh => self.refresh(),
            A::Settings => self.settings = true,
            A::Open | A::Quit => {}
        }
    }

    /// A one-off step from the menu: sent at once, no resting.
    fn nudge_now(&mut self, steps: f64) {
        self.nudge(steps);
        if let Some((v, _)) = self.pending_target.take() {
            let unit = self.target_spec().map(|s| s.2).unwrap_or(self.unit());
            self.send(Command::Target(v, unit));
        }
    }

    pub fn say(&mut self, text: &str, error: bool) {
        self.toast = Some(Toast { text: text.to_string(), error, until: self.clock + if error { 5.0 } else { 3.0 } });
    }

    fn interval(&self) -> f64 {
        // The menu bar shows the room temperature, so it stays fresh while the window is closed.
        let visible = self.config.menu_bar || self.handle.as_ref().is_none_or(|h| h.is_visible());
        let base = self.config.refresh_secs as f64;
        if visible { base } else { (base * 4.0).max(300.0) }
    }

    fn schedule(&mut self, after: f64) {
        self.next_refresh = self.next_refresh.min(self.clock + after).max(self.clock);
    }

    // ------------------------------------------------------------------ the clock

    /// Runs before every frame (and on every `wake_after`). True when the view must rebuild.
    pub fn poll(&mut self, t: f64, tx: &mut Tx) -> bool {
        self.clock = t;
        let mut changed = false;

        if let Some(toast) = &self.toast
            && t >= toast.until
        {
            self.toast = None;
            changed = true;
        }

        if let Some((v, due)) = self.pending_target {
            if t >= due {
                self.pending_target = None;
                let unit = self.target_spec().map(|s| s.2).unwrap_or(self.unit());
                self.send(Command::Target(v, unit));
                changed = true;
            } else {
                tx.wake_at(due);
            }
        }

        if self.phase == Phase::Ready && self.device.is_some() && t >= self.next_refresh && self.in_flight.is_empty() {
            let id = self.device.clone().unwrap_or_default();
            self.worker.send(Job::Refresh { device: id });
            self.next_refresh = t + self.interval();
        }
        if self.next_refresh.is_finite() {
            tx.wake_at(self.next_refresh);
        }
        if let Some(toast) = &self.toast {
            tx.wake_at(toast.until);
        }

        self.fx_live.store(self.effects_moving(), Ordering::Relaxed);
        self.update_menu_bar();
        changed
    }

    fn update_menu_bar(&mut self) {
        if !self.config.menu_bar || self.handle.is_none() {
            return;
        }
        let model = crate::status_item::model(self);
        if model != self.menu {
            crate::status_item::update(&model);
            self.menu = model;
        }
    }
}

/// The country from the system locale (`en_US.UTF-8` or macOS's region `en_BR` → `US`, `BR`),
/// else the US.
fn default_country() -> &'static str {
    let from_env = ["LC_ALL", "LANG"].iter().filter_map(|k| std::env::var(k).ok()).find(|v| v.contains('_'));
    #[cfg(target_os = "macos")]
    let from_env = from_env.or_else(|| {
        // Apps opened from Finder get no LANG; the region is in the global defaults.
        let out = std::process::Command::new("/usr/bin/defaults").args(["read", "-g", "AppleLocale"]).output().ok()?;
        out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
    });
    let lang = from_env.unwrap_or_default();
    let code = lang.split(['.', '@']).next().and_then(|l| l.split('_').nth(1)).unwrap_or("US");
    crate::thinq::Country::all().into_iter().find(|c| c.eq_ignore_ascii_case(code)).unwrap_or("US")
}

/// Formats a temperature with its unit, without trailing `.0`.
pub fn fmt_temp(v: f64) -> String {
    if (v - v.round()).abs() < 0.05 { format!("{}", v.round() as i64) } else { format!("{v:.1}") }
}

/// "1h 30m", "45m".
pub fn fmt_minutes(m: u32) -> String {
    match (m / 60, m % 60) {
        (0, m) => format!("{m}m"),
        (h, 0) => format!("{h}h"),
        (h, m) => format!("{h}h {m}m"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats() {
        assert_eq!(fmt_temp(23.0), "23");
        assert_eq!(fmt_temp(22.5), "22.5");
        assert_eq!(fmt_minutes(90), "1h 30m");
        assert_eq!(fmt_minutes(120), "2h");
        assert_eq!(fmt_minutes(30), "30m");
    }
}
