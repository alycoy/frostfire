//! A pretend LG home, for trying Frostfire without one and for the headless screenshots: two
//! air conditioners, a purifier, a dehumidifier, a fridge, a washer and a robot vacuum. It
//! answers like ThinQ does; the room drifts toward the target while an AC runs, and a washer
//! started from the app starts running.

use serde_json::{Value, json};

use crate::thinq::model::{Device, Profile, State, Unit};
use crate::thinq::spec;

pub const LIVING: &str = "demo-living-room";
pub const BEDROOM: &str = "demo-bedroom";
pub const PURIFIER: &str = "demo-purifier";
pub const DEHUMIDIFIER: &str = "demo-dehumidifier";
pub const FRIDGE: &str = "demo-fridge";
pub const WASHER: &str = "demo-washer";
pub const ROBOT: &str = "demo-robot";

pub fn devices() -> Vec<Device> {
    let d = |id: &str, kind: &str, model: &str, alias: &str| Device { id: id.into(), kind: kind.into(), model: model.into(), alias: alias.into() };
    vec![
        d(LIVING, "DEVICE_AIR_CONDITIONER", "DUALCOOL S3", "Living room"),
        d(BEDROOM, "DEVICE_AIR_CONDITIONER", "ARTCOOL Mirror", "Bedroom"),
        d(PURIFIER, "DEVICE_AIR_PURIFIER", "PuriCare 360", "Bedroom purifier"),
        d(DEHUMIDIFIER, "DEVICE_DEHUMIDIFIER", "PuriCare Dehumidifier", "Basement"),
        d(FRIDGE, "DEVICE_REFRIGERATOR", "InstaView", "Kitchen fridge"),
        d(WASHER, "DEVICE_WASHER", "WashTower F4", "Washer"),
        d(ROBOT, "DEVICE_ROBOT_CLEANER", "CordZero R9", "Robot vacuum"),
    ]
}

/// A generic demo device's fixture: `{"profile": …, "state": …}`.
fn fixture(id: &str) -> Option<Value> {
    let src = match id {
        PURIFIER => include_str!("../tests/fixtures/devices/purifier.json"),
        DEHUMIDIFIER => include_str!("../tests/fixtures/devices/dehumidifier.json"),
        FRIDGE => include_str!("../tests/fixtures/devices/fridge.json"),
        WASHER => include_str!("../tests/fixtures/devices/washer.json"),
        ROBOT => include_str!("../tests/fixtures/devices/robot.json"),
        _ => return None,
    };
    serde_json::from_str(src).ok()
}

pub fn profile_json(id: &str) -> Value {
    if let Some(f) = fixture(id) {
        return f["profile"].clone();
    }
    let src = if id == BEDROOM { include_str!("../tests/fixtures/ac_legacy_profile.json") } else { include_str!("../tests/fixtures/ac_profile.json") };
    let mut p: Value = serde_json::from_str(src).unwrap_or(Value::Null);
    if id == LIVING {
        // Like LG's current split units: detailed speeds that lack AUTO (it's on windStrength).
        let rw = |values: Value| json!({"type": "enum", "mode": ["r", "w"], "value": {"r": values.clone(), "w": values}});
        if let Some(air) = p.pointer_mut("/property/airFlow").and_then(Value::as_object_mut) {
            air.insert("windStrengthDetail".into(), rw(json!(["NATURE", "LOW", "MID", "HIGH"])));
        }
    }
    p
}

pub fn profile(id: &str) -> Profile {
    Profile::parse(&profile_json(id))
}

/// The state a demo unit starts in.
pub fn initial_state(id: &str) -> State {
    if let Some(f) = fixture(id) {
        return State(f["state"].clone());
    }
    if id == BEDROOM {
        State(json!({
            "airConJobMode": {"currentJobMode": "COOL"},
            "operation": {"airConOperationMode": "POWER_OFF"},
            "temperature": {"currentTemperature": 24, "targetTemperature": 22, "unit": "C"},
            "airFlow": {"windStrengthDetail": "LOW"}
        }))
    } else {
        let mut s = State(serde_json::from_str(include_str!("../tests/fixtures/ac_state.json")).unwrap_or(Value::Null));
        s.apply(&json!({"airFlow": {"windStrengthDetail": "MID"}, "windDirection": {"rotateUpDown": false, "rotateLeftRight": true}}));
        s
    }
}

/// A demo device, simulated.
pub struct DemoDevice {
    pub state: State,
    room: f64,
    ac: bool,
}

impl DemoDevice {
    pub fn new(id: &str) -> DemoDevice {
        let state = initial_state(id);
        let room = state.temp("currentTemperature", Unit::C).unwrap_or(25.0);
        DemoDevice { state, room, ac: fixture(id).is_none() }
    }

    /// Moves the room toward the target for `dt` seconds of running.
    pub fn tick(&mut self, dt: f64) {
        if !self.ac {
            return;
        }
        let on = self.state.power() == Some(true);
        let mode = self.state.mode().unwrap_or("COOL").to_string();
        let key = match mode.as_str() {
            "COOL" if self.state.temp("coolTargetTemperature", Unit::C).is_some() => "coolTargetTemperature",
            "HEAT" if self.state.temp("heatTargetTemperature", Unit::C).is_some() => "heatTargetTemperature",
            _ => "targetTemperature",
        };
        let target = self.state.temp(key, Unit::C).unwrap_or(24.0);
        let outside = 29.0;
        let goal = match (on, mode.as_str()) {
            (true, "COOL") => target.min(self.room),
            (true, "HEAT") => target.max(self.room),
            (true, "AUTO") => target,
            _ => outside,
        };
        let rate = if on { 0.004 } else { 0.0008 };
        self.room += (goal - self.room) * (1.0 - (-rate * dt).exp());
        let room = (self.room * 2.0).round() / 2.0;
        let set = |s: &mut State, unit: &str, v: f64| match s.0.get_mut("temperatureInUnits").and_then(Value::as_array_mut) {
            Some(list) => {
                for e in list.iter_mut().filter(|e| e.get("unit").and_then(Value::as_str) == Some(unit)) {
                    e["currentTemperature"] = json!(v);
                }
            }
            None => {
                if unit == "C"
                    && let Some(t) = s.0.get_mut("temperature")
                {
                    t["currentTemperature"] = json!(v);
                }
            }
        };
        set(&mut self.state, "C", room);
        set(&mut self.state, "F", (Unit::convert(room, Unit::C, Unit::F)).round());
    }

    pub fn control(&mut self, payload: &Value) {
        if !self.ac {
            return self.control_generic(payload);
        }
        let mut p = payload.clone();
        // A relative timer that's set reports SET, as the real units do.
        for (res, flag, keys) in [
            ("timer", "relativeStopTimer", ["relativeHourToStop", "relativeMinuteToStop"]),
            ("timer", "relativeStartTimer", ["relativeHourToStart", "relativeMinuteToStart"]),
            ("sleepTimer", "relativeStopTimer", ["relativeHourToStop", "relativeMinuteToStop"]),
        ] {
            if let Some(o) = p.get_mut(res).and_then(Value::as_object_mut)
                && keys.iter().any(|k| o.contains_key(*k))
            {
                if !o.contains_key(keys[1]) {
                    o.insert(keys[1].into(), json!(0));
                }
                o.insert(flag.into(), json!("SET"));
            }
        }
        self.state.apply(&p);
    }

    /// Merges the change, and answers an operation the way the device would.
    fn control_generic(&mut self, payload: &Value) {
        spec::merge(&mut self.state.0, payload);
        let ops = [
            ("washerOperationMode", "START", "RUNNING"),
            ("washerOperationMode", "STOP", "INITIAL"),
            ("washerOperationMode", "POWER_OFF", "POWER_OFF"),
            ("cleanOperationMode", "START", "WORKING"),
            ("cleanOperationMode", "RESUME", "WORKING"),
            ("cleanOperationMode", "PAUSE", "PAUSE"),
            ("cleanOperationMode", "HOMING", "HOMING"),
        ];
        let loc = payload.get("location").cloned();
        for (key, value, run) in ops {
            if payload.pointer(&format!("/operation/{key}")).and_then(Value::as_str) == Some(value) {
                let mut p = json!({"runState": {"currentState": run}});
                if let Some(l) = &loc {
                    p["location"] = l.clone();
                }
                spec::merge(&mut self.state.0, &p);
            }
        }
    }
}

/// A week of plausible daily energy use (Wh), oldest first.
pub fn energy_week() -> Vec<(String, f64)> {
    let days = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let wh = [2100.0, 2650.0, 1800.0, 3200.0, 2900.0, 4100.0, 3600.0];
    days.iter().zip(wh).map(|(d, w)| ((*d).to_string(), w)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thinq::model::{Command, Timer};

    #[test]
    fn demo_profiles_parse() {
        assert_eq!(profile(LIVING).modes().len(), 5);
        assert!(profile(BEDROOM).can_power());
    }

    #[test]
    fn auto_fan_comes_from_the_plain_speed() {
        // As LG's split units report it: detailed speeds without AUTO, AUTO on windStrength.
        let p = profile(LIVING);
        assert_eq!(p.wind_speeds(), ["NATURE", "LOW", "MID", "HIGH", "AUTO"]);
        assert_eq!(p.wind_key_for("AUTO"), "windStrength");
        let s = initial_state(LIVING);
        assert_eq!(Command::Wind("AUTO".into()).payload(&p, &s).unwrap(), json!({"airFlow": {"windStrength": "AUTO"}}));
        assert_eq!(Command::Wind("LOW".into()).payload(&p, &s).unwrap(), json!({"airFlow": {"windStrengthDetail": "LOW"}}));
    }

    #[test]
    fn a_running_unit_cools_the_room() {
        let mut ac = DemoDevice::new(LIVING);
        let before = ac.state.temp("currentTemperature", Unit::C).unwrap();
        for _ in 0..60 {
            ac.tick(60.0);
        }
        let after = ac.state.temp("currentTemperature", Unit::C).unwrap();
        assert!(after < before, "{before} → {after}");
    }

    #[test]
    fn every_demo_device_loads_and_a_washer_starts() {
        for d in devices() {
            let spec = crate::thinq::spec::Spec::parse(&profile_json(&d.id));
            assert!(!spec.zones.is_empty(), "{}", d.id);
        }
        let mut w = DemoDevice::new(WASHER);
        let spec = crate::thinq::spec::Spec::parse(&profile_json(WASHER));
        let op = spec.find("operation", "washerOperationMode").unwrap().path.clone();
        w.control(&spec.payload(&op, json!("STOP")).unwrap());
        let run = spec.find("runState", "currentState").unwrap().path.clone();
        assert_eq!(crate::thinq::spec::read(&w.state.0, &run), Some(&json!("INITIAL")));
    }

    #[test]
    fn timers_report_set() {
        let mut ac = DemoDevice::new(LIVING);
        let p = profile(LIVING);
        let payload = Command::TimerIn(Timer::Sleep, 120).payload(&p, &ac.state).unwrap();
        ac.control(&payload);
        assert_eq!(ac.state.timer_minutes(Timer::Sleep), Some(120));
    }
}
