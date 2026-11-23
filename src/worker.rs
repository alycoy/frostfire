//! The one thread that talks to LG, the Keychain and the disk.
//!
//! The UI sends [`Job`]s; answers come back as [`Event`]s through `AppHandle::update`, so the
//! window never waits on the network. Jobs run one at a time, in order, so two quick changes
//! reach the air conditioner in the order they were made.

use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::app::App;
use crate::demo::{self, DemoAc};
use crate::store::{self, Config};
use crate::thinq::model::{self, Device, Profile, State};
use crate::thinq::{ApiError, Client, Country, Secret};

pub enum Job {
    /// At launch: load the config and any saved token, and sign in with it.
    Restore,
    /// Check a new token by listing devices; keep it in the Keychain only if LG accepts it.
    SignIn {
        token: Secret,
        country: String,
    },
    SignOut,
    Demo,
    Devices,
    /// Profile, state and energy for a device.
    Load {
        device: String,
    },
    Refresh {
        device: String,
    },
    Control {
        device: String,
        payload: Value,
        seq: u64,
    },
    SaveConfig(Config),
}

pub enum Event {
    Restored { config: Config, has_token: bool },
    SignedIn { devices: Vec<Device>, demo: bool },
    SignInFailed(String),
    SignedOut,
    Devices(Vec<Device>),
    Loaded { device: String, profile: Profile, state: State },
    State { device: String, state: State },
    Energy { device: String, days: Vec<(String, f64)> },
    ControlDone { seq: u64 },
    ControlFailed { seq: u64, message: String },
    Failed { message: String, auth: bool },
}

#[derive(Clone)]
pub struct Worker {
    tx: Sender<Job>,
}

impl Worker {
    pub fn spawn(handle: rune::AppHandle<App>) -> Worker {
        let (tx, rx) = channel();
        std::thread::Builder::new()
            .name("thinq".into())
            .spawn(move || run(rx, move |ev| handle.update(move |app, tx| app.on_event(ev, tx))))
            .expect("spawn the ThinQ worker");
        Worker { tx }
    }

    /// A worker whose jobs go nowhere (headless snapshots and tests).
    pub fn detached() -> Worker {
        let (tx, _rx) = channel();
        Worker { tx }
    }

    pub fn send(&self, job: Job) {
        let _ = self.tx.send(job);
    }
}

enum Backend {
    None,
    Live(Client),
    Demo { units: HashMap<String, DemoAc>, last: Instant },
}

fn run(rx: Receiver<Job>, emit: impl Fn(Event)) {
    let mut backend = Backend::None;
    let mut config = Config::default();
    while let Ok(job) = rx.recv() {
        match job {
            Job::Restore => {
                config = store::load_config();
                if config.client_id.is_none() {
                    config.client_id = crate::thinq::client::new_client_id();
                    let _ = store::save_config(&config);
                }
                let token = store::load_token();
                emit(Event::Restored { config: config.clone(), has_token: token.is_some() });
                if let (Some(token), Some(country)) = (token, config.country.clone()) {
                    sign_in(&mut backend, &mut config, token, &country, false, &emit);
                }
            }
            Job::SignIn { token, country } => sign_in(&mut backend, &mut config, token, &country, true, &emit),
            Job::SignOut => {
                store::delete_token();
                backend = Backend::None;
                config.device = None;
                let _ = store::save_config(&config);
                emit(Event::SignedOut);
            }
            Job::Demo => {
                let units = demo::devices().iter().map(|d| (d.id.clone(), DemoAc::new(&d.id))).collect();
                backend = Backend::Demo { units, last: Instant::now() };
                emit(Event::SignedIn { devices: demo::devices(), demo: true });
            }
            Job::Devices => match &backend {
                Backend::Live(c) => match c.devices() {
                    Ok(v) => emit(Event::Devices(model::parse_devices(&v))),
                    Err(e) => emit(failed(&e)),
                },
                Backend::Demo { .. } => emit(Event::Devices(demo::devices())),
                Backend::None => {}
            },
            Job::Load { device } => match &mut backend {
                Backend::Live(c) => {
                    match c.profile(&device).and_then(|p| Ok((p, c.state(&device)?))) {
                        Ok((p, s)) => emit(Event::Loaded { device: device.clone(), profile: Profile::parse(&p), state: State(s) }),
                        Err(e) => {
                            emit(failed(&e));
                            continue;
                        }
                    }
                    if let Some(days) = energy(c, &device) {
                        emit(Event::Energy { device, days });
                    }
                }
                Backend::Demo { units, last } => {
                    step_demo(units, last);
                    let Some(ac) = units.get(&device) else { continue };
                    emit(Event::Loaded { device: device.clone(), profile: demo::profile(&device), state: ac.state.clone() });
                    if device == demo::LIVING {
                        emit(Event::Energy { device, days: demo::energy_week() });
                    }
                }
                Backend::None => {}
            },
            Job::Refresh { device } => match &mut backend {
                Backend::Live(c) => match c.state(&device) {
                    Ok(s) => emit(Event::State { device, state: State(s) }),
                    Err(e) => emit(failed(&e)),
                },
                Backend::Demo { units, last } => {
                    step_demo(units, last);
                    if let Some(ac) = units.get(&device) {
                        emit(Event::State { device, state: ac.state.clone() });
                    }
                }
                Backend::None => {}
            },
            Job::Control { device, payload, seq } => match &mut backend {
                Backend::Live(c) => match c.control(&device, &payload) {
                    Ok(_) => emit(Event::ControlDone { seq }),
                    Err(e) => emit(Event::ControlFailed { seq, message: e.friendly() }),
                },
                Backend::Demo { units, last } => {
                    step_demo(units, last);
                    if let Some(ac) = units.get_mut(&device) {
                        ac.control(&payload);
                    }
                    emit(Event::ControlDone { seq });
                }
                Backend::None => emit(Event::ControlFailed { seq, message: "Not signed in.".into() }),
            },
            Job::SaveConfig(c) => {
                // The account (country and client id) is the worker's: sign-in sets it and sign-out
                // clears it. The window's copy may predate sign-in, so it never overrides them.
                config = Config { client_id: config.client_id.clone(), country: config.country.clone(), ..c }.sanitized();
                let _ = store::save_config(&config);
            }
        }
    }
}

fn sign_in(backend: &mut Backend, config: &mut Config, token: Secret, country: &str, fresh: bool, emit: &impl Fn(Event)) {
    let Some(country) = Country::parse(country) else {
        emit(Event::SignInFailed("Pick the country your LG account is registered in.".into()));
        return;
    };
    let Some(client_id) = config.client_id.clone().or_else(crate::thinq::client::new_client_id) else {
        emit(Event::SignInFailed("No system randomness for a client id.".into()));
        return;
    };
    let client = match Client::new(token.clone(), country, client_id.clone()) {
        Ok(c) => c,
        Err(e) => {
            emit(Event::SignInFailed(e.friendly()));
            return;
        }
    };
    match client.devices() {
        Ok(v) => {
            if fresh && let Err(e) = store::save_token(&token) {
                emit(Event::Failed { message: format!("Signed in, but the token couldn't be saved ({e})."), auth: false });
            }
            config.country = Some(country.code().to_string());
            config.client_id = Some(client_id);
            let _ = store::save_config(config);
            *backend = Backend::Live(client);
            emit(Event::SignedIn { devices: model::parse_devices(&v), demo: false });
        }
        Err(e) if e.is_auth() && !fresh => {
            // The saved token stopped working: forget it so the next launch asks again.
            store::delete_token();
            emit(Event::SignInFailed(e.friendly()));
        }
        Err(e) if !fresh && matches!(e, ApiError::Network(_)) => {
            // Offline at launch: keep the token and try again on the next refresh.
            *backend = Backend::Live(client);
            emit(Event::SignedIn { devices: Vec::new(), demo: false });
            emit(failed(&e));
        }
        Err(e) => emit(Event::SignInFailed(e.friendly())),
    }
}

fn failed(e: &ApiError) -> Event {
    Event::Failed { message: e.friendly(), auth: e.is_auth() }
}

fn step_demo(units: &mut HashMap<String, DemoAc>, last: &mut Instant) {
    let dt = last.elapsed().as_secs_f64() * 20.0; // a demo minute passes in three seconds
    *last = Instant::now();
    for ac in units.values_mut() {
        ac.tick(dt);
    }
}

/// The last seven days of energy use, when the unit reports any. Units without an energy
/// meter answer with an error, which is fine: the card just doesn't show.
fn energy(c: &Client, device: &str) -> Option<Vec<(String, f64)>> {
    let profile = c.energy_profile(device).ok()?;
    let props: Vec<String> = ["property", "result"]
        .iter()
        .find_map(|k| match profile.get(k) {
            Some(Value::Array(a)) => Some(a.clone()),
            Some(Value::Object(o)) => o.get("property").and_then(Value::as_array).cloned(),
            _ => None,
        })
        .unwrap_or_default()
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    let prop = props.iter().find(|p| p.as_str() == "totalEnergy").or(props.first())?;
    let today = days_since_epoch();
    let usage = c.energy_daily(device, prop, &yyyymmdd(today - 6), &yyyymmdd(today)).ok()?;
    let days = parse_energy(&usage);
    (!days.is_empty()).then_some(days)
}

/// Pulls (day, value) pairs from an energy answer, whatever its exact nesting: the first
/// array of objects that each hold an 8-digit date and a number.
pub fn parse_energy(v: &Value) -> Vec<(String, f64)> {
    fn rows(v: &Value) -> Option<Vec<(String, f64)>> {
        match v {
            Value::Array(a) if !a.is_empty() => {
                let parsed: Vec<(String, f64)> = a
                    .iter()
                    .filter_map(|row| {
                        let o = row.as_object()?;
                        let date = o.values().filter_map(Value::as_str).find(|s| s.len() == 8 && s.bytes().all(|b| b.is_ascii_digit()))?;
                        let n = o.iter().filter(|(k, _)| !k.to_lowercase().contains("date")).find_map(|(_, v)| v.as_f64())?;
                        Some((format!("{}/{}", &date[6..8], &date[4..6]), n))
                    })
                    .collect();
                (!parsed.is_empty()).then_some(parsed).or_else(|| a.iter().find_map(rows))
            }
            Value::Object(o) => o.values().find_map(rows),
            _ => None,
        }
    }
    let mut out = rows(v).unwrap_or_default();
    out.truncate(31);
    out
}

fn days_since_epoch() -> i64 {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO).as_secs() as i64;
    secs.div_euclid(86_400)
}

/// `YYYYMMDD` for a day number (Howard Hinnant's civil-from-days).
fn yyyymmdd(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}{m:02}{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn civil_dates() {
        assert_eq!(yyyymmdd(0), "19700101");
        assert_eq!(yyyymmdd(20_454), "20260101");
        assert_eq!(yyyymmdd(11_016), "20000229");
    }

    #[test]
    fn energy_answers_parse_loosely() {
        let v = json!({"dataList": [{"usedDate": "20261120", "energyUsage": 1200}, {"usedDate": "20261121", "energyUsage": 900.5}]});
        assert_eq!(parse_energy(&v), [("20/11".to_string(), 1200.0), ("21/11".to_string(), 900.5)]);
        assert!(parse_energy(&json!({"nothing": true})).is_empty());
    }
}
