//! What an air conditioner can do (its profile), what it is doing (its state), and the
//! commands that change it.
//!
//! Resource and property names follow LG's SDK (`devices/air_conditioner.py`). Every command
//! is checked against the device's own profile before it's built, so the app never sends a
//! mode, fan speed or temperature the unit didn't say it accepts.

use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

pub const DEVICE_AIR_CONDITIONER: &str = "DEVICE_AIR_CONDITIONER";

/// One device from `GET /devices`.
#[derive(Clone, Debug, PartialEq)]
pub struct Device {
    pub id: String,
    pub kind: String,
    pub model: String,
    pub alias: String,
}

impl Device {
    pub fn is_ac(&self) -> bool {
        self.kind == DEVICE_AIR_CONDITIONER
    }

    pub fn kind(&self) -> super::kinds::Kind {
        super::kinds::kind(&self.kind)
    }

    /// What the app calls it: the name the owner gave it, else the model.
    pub fn name(&self) -> String {
        if !self.alias.is_empty() {
            self.alias.clone()
        } else if !self.model.is_empty() {
            self.model.clone()
        } else {
            self.kind().name.to_string()
        }
    }
}

/// Parses `GET /devices`, keeping devices whose id is safe to use in a path.
pub fn parse_devices(v: &Value) -> Vec<Device> {
    let list = v.as_array().cloned().unwrap_or_default();
    list.iter()
        .filter_map(|d| {
            let id = d.get("deviceId")?.as_str()?.to_string();
            super::client::check_device_id(&id).ok()?;
            let info = d.get("deviceInfo").cloned().unwrap_or(Value::Null);
            let s = |k: &str| info.get(k).and_then(Value::as_str).unwrap_or("").chars().take(80).collect::<String>();
            Some(Device { id, kind: s("deviceType"), model: s("modelName"), alias: s("alias") })
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    C,
    F,
}

impl Unit {
    pub fn as_str(self) -> &'static str {
        match self {
            Unit::C => "C",
            Unit::F => "F",
        }
    }

    pub fn parse(s: &str) -> Option<Unit> {
        match s {
            "C" | "c" => Some(Unit::C),
            "F" | "f" => Some(Unit::F),
            _ => None,
        }
    }

    pub fn convert(v: f64, from: Unit, to: Unit) -> f64 {
        match (from, to) {
            (Unit::C, Unit::F) => v * 9.0 / 5.0 + 32.0,
            (Unit::F, Unit::C) => (v - 32.0) * 5.0 / 9.0,
            _ => v,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Range {
    pub min: f64,
    pub max: f64,
    pub step: f64,
}

impl Range {
    /// Clamps to the range and snaps to its step.
    pub fn snap(&self, v: f64) -> f64 {
        let step = if self.step > 0.0 { self.step } else { 1.0 };
        let v = v.clamp(self.min, self.max);
        let snapped = self.min + ((v - self.min) / step).round() * step;
        (snapped.clamp(self.min, self.max) * 100.0).round() / 100.0
    }
}

/// What the profile says about one property.
#[derive(Clone, Debug, PartialEq)]
pub struct Prop {
    pub readable: bool,
    pub writable: bool,
    pub kind: Kind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Enum {
        read: Vec<String>,
        write: Vec<String>,
    },
    Range {
        read: Option<Range>,
        write: Option<Range>,
    },
    Bool,
    Number,
    /// Free text (`"string"`), or a bare constant in the profile.
    Text,
    Other,
}

fn parse_range(v: Option<&Value>) -> Option<Range> {
    let v = v?;
    let n = |k: &str| v.get(k).and_then(Value::as_f64);
    let (min, max) = (n("min")?, n("max")?);
    (min.is_finite() && max.is_finite() && min <= max).then(|| Range { min, max, step: n("step").filter(|s| *s > 0.0).unwrap_or(1.0) })
}

pub(crate) fn parse_prop(v: &Value) -> Prop {
    let Some(obj) = v.as_object() else {
        // A bare value in a profile is a read-only constant.
        return Prop { readable: true, writable: false, kind: Kind::Text };
    };
    let mode = obj.get("mode").and_then(Value::as_array).cloned().unwrap_or_default();
    let has = |m: &str| mode.iter().any(|x| x.as_str() == Some(m));
    let (readable, writable) = (has("r"), has("w"));
    let strings = |side: &str| -> Vec<String> {
        obj.get("value")
            .and_then(|val| val.get(side))
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
            .unwrap_or_default()
    };
    let kind = match obj.get("type").and_then(Value::as_str).unwrap_or("") {
        "enum" => Kind::Enum { read: strings("r"), write: if writable { strings("w") } else { Vec::new() } },
        "range" => {
            let val = obj.get("value");
            Kind::Range { read: parse_range(val.and_then(|v| v.get("r"))), write: parse_range(val.and_then(|v| v.get("w"))) }
        }
        "boolean" => Kind::Bool,
        "number" => Kind::Number,
        "string" => Kind::Text,
        _ => Kind::Other,
    };
    Prop { readable, writable, kind }
}

/// Where a device keeps its temperatures: newer units report one entry per unit
/// (`temperatureInUnits`), older ones a single `temperature` object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TempResource {
    InUnits,
    Legacy,
    None,
}

impl TempResource {
    fn key(self) -> &'static str {
        match self {
            TempResource::InUnits => "temperatureInUnits",
            TempResource::Legacy | TempResource::None => "temperature",
        }
    }
}

/// The four target temperatures a unit may take, by mode.
pub const TARGET_KEYS: [&str; 4] = ["targetTemperature", "coolTargetTemperature", "heatTargetTemperature", "autoTargetTemperature"];

/// Wind-direction switches, with the label the UI shows.
pub const WIND_FLAGS: [(&str, &str); 10] = [
    ("rotateUpDown", "Up–down swing"),
    ("rotateLeftRight", "Side swing"),
    ("autoFitWind", "Auto fit"),
    ("airGuideWind", "Air guide"),
    ("concentrationWind", "Focused"),
    ("forestWind", "Forest"),
    ("highCeilingWind", "High ceiling"),
    ("longPowerWind", "Long reach"),
    ("mannerWind", "Gentle"),
    ("swirlWind", "Swirl"),
];

/// A device's capabilities, parsed from `GET /devices/{id}/profile`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Profile {
    props: BTreeMap<(String, String), Prop>,
    temps: BTreeMap<&'static str, BTreeMap<String, Prop>>,
    temp_resource: Option<TempResource>,
}

impl Profile {
    pub fn parse(v: &Value) -> Profile {
        let mut p = Profile::default();
        let root = v.get("property").unwrap_or(v);
        // Multi-zone devices list their properties per location; an AC has one.
        let root = match root {
            Value::Array(a) => a.first().cloned().unwrap_or(Value::Null),
            other => other.clone(),
        };
        let Some(obj) = root.as_object() else { return p };
        for (res, body) in obj {
            match (res.as_str(), body) {
                ("temperatureInUnits", Value::Array(units)) => {
                    p.temp_resource = Some(TempResource::InUnits);
                    for entry in units {
                        let Some(unit) = entry.get("unit").and_then(Value::as_str).and_then(Unit::parse) else { continue };
                        let slot = p.temps.entry(unit.as_str()).or_default();
                        if let Some(e) = entry.as_object() {
                            for (k, prop) in e {
                                if k != "unit" {
                                    slot.insert(k.clone(), parse_prop(prop));
                                }
                            }
                        }
                    }
                }
                ("temperature", Value::Object(t)) if p.temp_resource != Some(TempResource::InUnits) => {
                    p.temp_resource = Some(TempResource::Legacy);
                    let units: Vec<Unit> = match t.get("unit").map(parse_prop) {
                        Some(Prop { kind: Kind::Enum { read, .. }, .. }) => read.iter().filter_map(|u| Unit::parse(u)).collect(),
                        _ => vec![Unit::C],
                    };
                    for unit in units.iter().take(1) {
                        let slot = p.temps.entry(unit.as_str()).or_default();
                        for (k, prop) in t {
                            if k != "unit" {
                                slot.insert(k.clone(), parse_prop(prop));
                            }
                        }
                    }
                }
                (_, Value::Object(props)) => {
                    for (k, prop) in props {
                        p.props.insert((res.clone(), k.clone()), parse_prop(prop));
                    }
                }
                _ => {}
            }
        }
        p
    }

    pub fn prop(&self, res: &str, key: &str) -> Option<&Prop> {
        self.props.get(&(res.to_string(), key.to_string()))
    }

    pub fn writable(&self, res: &str, key: &str) -> bool {
        self.prop(res, key).is_some_and(|p| p.writable)
    }

    /// The values an enum property accepts (empty when it isn't writable).
    pub fn enum_writes(&self, res: &str, key: &str) -> Vec<String> {
        match self.prop(res, key) {
            Some(Prop { writable: true, kind: Kind::Enum { write, .. }, .. }) => write.clone(),
            _ => Vec::new(),
        }
    }

    pub fn temp_resource(&self) -> TempResource {
        self.temp_resource.unwrap_or(TempResource::None)
    }

    /// Units the device takes temperatures in.
    pub fn units(&self) -> Vec<Unit> {
        self.temps.keys().filter_map(|u| Unit::parse(u)).collect()
    }

    fn temp_prop(&self, unit: Unit, key: &str) -> Option<&Prop> {
        self.temps.get(unit.as_str())?.get(key)
    }

    /// The writable range of a target temperature, in `unit`.
    pub fn target_range(&self, unit: Unit, key: &str) -> Option<Range> {
        match self.temp_prop(unit, key)? {
            Prop { writable: true, kind: Kind::Range { write, read }, .. } => write.or(*read),
            _ => None,
        }
    }

    /// Which target property a mode adjusts: the mode's own when the unit has it, else the
    /// plain target.
    pub fn target_key(&self, unit: Unit, mode: Option<&str>) -> Option<&'static str> {
        let own = match mode {
            Some("COOL") => Some("coolTargetTemperature"),
            Some("HEAT") => Some("heatTargetTemperature"),
            Some("AUTO") => Some("autoTargetTemperature"),
            _ => None,
        };
        own.into_iter().chain(["targetTemperature"]).chain(TARGET_KEYS).find(|k| self.target_range(unit, k).is_some())
    }

    /// The unit to send temperatures in: the preferred one when writable, else any.
    pub fn control_unit(&self, preferred: Unit) -> Option<Unit> {
        let writable = |u: Unit| TARGET_KEYS.iter().any(|k| self.target_range(u, k).is_some());
        if writable(preferred) {
            return Some(preferred);
        }
        self.units().into_iter().find(|u| writable(*u))
    }

    pub fn modes(&self) -> Vec<String> {
        self.enum_writes("airConJobMode", "currentJobMode")
    }

    pub fn can_power(&self) -> bool {
        let w = self.enum_writes("operation", "airConOperationMode");
        w.iter().any(|v| v == "POWER_ON") && w.iter().any(|v| v == "POWER_OFF")
    }

    /// The airflow property: the detailed one when the unit has it.
    /// Every fan speed the unit takes: its detailed speeds, plus any plain one they lack (many
    /// units take AUTO only as `windStrength`).
    pub fn wind_speeds(&self) -> Vec<String> {
        let mut v = self.enum_writes("airFlow", "windStrengthDetail");
        for s in self.enum_writes("airFlow", "windStrength") {
            if !v.contains(&s) {
                v.push(s);
            }
        }
        v
    }

    /// The `airFlow` property that sets `speed`.
    pub fn wind_key_for(&self, speed: &str) -> &'static str {
        if self.enum_writes("airFlow", "windStrengthDetail").iter().any(|s| s == speed) { "windStrengthDetail" } else { "windStrength" }
    }

    pub fn wind_flags(&self) -> Vec<(&'static str, &'static str)> {
        WIND_FLAGS.iter().copied().filter(|(k, _)| self.writable("windDirection", k)).collect()
    }

    /// Vane positions the unit takes, one entry per axis: any writable enum under
    /// `windDirection` that isn't an on/off switch (some LG models list fixed louvre steps).
    pub fn vanes(&self) -> Vec<Vane> {
        let mut out: Vec<Vane> = self
            .props
            .iter()
            .filter(|((res, key), _)| res == "windDirection" && !WIND_FLAGS.iter().any(|(k, _)| k == key))
            .filter_map(|((_, key), p)| match p {
                Prop { writable: true, kind: Kind::Enum { write, .. }, .. } if write.len() > 1 => {
                    let lower = key.to_ascii_lowercase();
                    let side = lower.contains("leftright") || lower.contains("horizontal");
                    Some(Vane { key: key.clone(), side, positions: write.clone() })
                }
                _ => None,
            })
            .collect();
        out.sort_by_key(|v| v.side);
        out
    }

    /// A numeric fan level (`airFlow.windStep`), on units that take one.
    pub fn wind_step(&self) -> Option<Range> {
        match self.prop("airFlow", "windStep") {
            Some(Prop { writable: true, kind: Kind::Range { write: Some(r), .. }, .. }) if r.max > r.min => Some(*r),
            _ => None,
        }
    }

    pub fn can_display_light(&self) -> bool {
        let w = self.enum_writes("display", "light");
        w.iter().any(|v| v == "ON") && w.iter().any(|v| v == "OFF")
    }

    pub fn can_air_clean(&self) -> bool {
        let w = self.enum_writes("operation", "airCleanOperationMode");
        w.iter().any(|v| v == "START") && w.iter().any(|v| v == "STOP")
    }

    pub fn can_timer(&self, which: Timer) -> bool {
        let (res, h, _, _) = which.keys();
        self.writable(res, h)
    }

    pub fn can_cancel(&self, which: Timer) -> bool {
        let (res, _, _, flag) = which.keys();
        self.enum_writes(res, flag).iter().any(|v| v == "UNSET")
    }
}

/// The three relative timers an AC may have.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Timer {
    /// Turn on after a delay (while off).
    Start,
    /// Turn off after a delay.
    Stop,
    /// The sleep timer: off after a delay, with the unit's quieter night behaviour.
    Sleep,
}

impl Timer {
    /// (resource, hour key, minute key, set/unset flag key).
    fn keys(self) -> (&'static str, &'static str, &'static str, &'static str) {
        match self {
            Timer::Start => ("timer", "relativeHourToStart", "relativeMinuteToStart", "relativeStartTimer"),
            Timer::Stop => ("timer", "relativeHourToStop", "relativeMinuteToStop", "relativeStopTimer"),
            Timer::Sleep => ("sleepTimer", "relativeHourToStop", "relativeMinuteToStop", "relativeStopTimer"),
        }
    }
}

/// A device's state from `GET /devices/{id}/state`, kept as JSON so new properties survive.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State(pub Value);

impl State {
    /// The fan speed as the unit reports it: the detailed one when there is one (it can be a
    /// speed set on the remote that the API can't set, like `LOW_MID`).
    pub fn wind(&self) -> Option<&str> {
        self.str("airFlow", "windStrengthDetail").or_else(|| self.str("airFlow", "windStrength"))
    }

    fn get(&self, res: &str, key: &str) -> Option<&Value> {
        self.0.get(res)?.get(key)
    }

    pub fn str(&self, res: &str, key: &str) -> Option<&str> {
        self.get(res, key)?.as_str()
    }

    pub fn num(&self, res: &str, key: &str) -> Option<f64> {
        self.get(res, key)?.as_f64()
    }

    pub fn bool(&self, res: &str, key: &str) -> Option<bool> {
        match self.get(res, key)? {
            Value::Bool(b) => Some(*b),
            Value::String(s) if s == "true" || s == "ON" => Some(true),
            Value::String(s) if s == "false" || s == "OFF" => Some(false),
            _ => None,
        }
    }

    pub fn power(&self) -> Option<bool> {
        match self.str("operation", "airConOperationMode")? {
            "POWER_ON" => Some(true),
            "POWER_OFF" => Some(false),
            _ => None,
        }
    }

    pub fn mode(&self) -> Option<&str> {
        self.str("airConJobMode", "currentJobMode")
    }

    /// A temperature property, in `want`, converting from whichever unit the device reports.
    pub fn temp(&self, key: &str, want: Unit) -> Option<f64> {
        let entries: Vec<&Value> = match self.0.get("temperatureInUnits") {
            Some(Value::Array(a)) => a.iter().collect(),
            _ => self.0.get("temperature").into_iter().collect(),
        };
        let read = |e: &Value| -> Option<(Unit, f64)> {
            let unit = e.get("unit").and_then(Value::as_str).and_then(Unit::parse).unwrap_or(Unit::C);
            Some((unit, e.get(key)?.as_f64()?))
        };
        if let Some((_, v)) = entries.iter().filter_map(|e| read(e)).find(|(u, _)| *u == want) {
            return Some(v);
        }
        entries.iter().filter_map(|e| read(e)).next().map(|(u, v)| Unit::convert(v, u, want))
    }

    /// Minutes left on a relative timer, when it's set.
    pub fn timer_minutes(&self, which: Timer) -> Option<u32> {
        let (res, h, m, flag) = which.keys();
        if self.str(res, flag) == Some("UNSET") {
            return None;
        }
        let hours = self.num(res, h).unwrap_or(0.0).max(0.0);
        let mins = self.num(res, m).unwrap_or(0.0).max(0.0);
        let total = (hours * 60.0 + mins) as u32;
        (total > 0).then_some(total)
    }

    /// Remaining filter life in percent, from whichever way the unit reports it.
    pub fn filter_percent(&self) -> Option<f64> {
        if let Some(p) = self.num("filterInfo", "filterRemainPercent") {
            return Some(p.clamp(0.0, 100.0));
        }
        let used = self.num("filterInfo", "usedTime")?;
        let life = self.num("filterInfo", "filterLifetime").filter(|l| *l > 0.0)?;
        Some((100.0 - used / life * 100.0).clamp(0.0, 100.0))
    }

    /// Merges a control payload into the state, so the UI shows a change before the device
    /// confirms it. Temperatures land in the entry of the unit they were sent in.
    pub fn apply(&mut self, payload: &Value) {
        let Some(patch) = payload.as_object() else { return };
        if !self.0.is_object() {
            self.0 = Value::Object(Map::new());
        }
        let root = self.0.as_object_mut().expect("object");
        for (res, body) in patch {
            match (res.as_str(), root.get_mut(res)) {
                ("temperatureInUnits", Some(Value::Array(entries))) => {
                    let unit = body.get("unit").and_then(Value::as_str).unwrap_or("C").to_string();
                    let Some(fields) = body.as_object() else { continue };
                    for entry in entries.iter_mut() {
                        let eu = entry.get("unit").and_then(Value::as_str).unwrap_or("C").to_string();
                        let Some(e) = entry.as_object_mut() else { continue };
                        for (k, v) in fields {
                            if k == "unit" {
                                continue;
                            }
                            match (eu == unit, v.as_f64(), Unit::parse(&unit), Unit::parse(&eu)) {
                                (true, _, _, _) => {
                                    e.insert(k.clone(), v.clone());
                                }
                                (false, Some(x), Some(from), Some(to)) => {
                                    let c = Unit::convert(x, from, to);
                                    e.insert(k.clone(), json!((c * 2.0).round() / 2.0));
                                }
                                _ => {}
                            }
                        }
                    }
                }
                (_, Some(Value::Object(existing))) => {
                    if let Some(fields) = body.as_object() {
                        for (k, v) in fields {
                            existing.insert(k.clone(), v.clone());
                        }
                    }
                }
                _ => {
                    root.insert(res.clone(), body.clone());
                }
            }
        }
    }
}

/// A louvre the unit can park at fixed positions.
#[derive(Clone, Debug, PartialEq)]
pub struct Vane {
    /// The `windDirection` property.
    pub key: String,
    /// Left–right rather than up–down.
    pub side: bool,
    /// The values it takes, in the profile's order.
    pub positions: Vec<String>,
}

impl Vane {
    /// The values that are a fixed angle (not off, swing or auto), in order.
    pub fn angles(&self) -> Vec<&str> {
        self.positions.iter().map(String::as_str).filter(|v| !is_moving(v)).collect()
    }
}

/// A vane value that isn't a fixed angle.
pub fn is_moving(v: &str) -> bool {
    matches!(v.to_ascii_uppercase().as_str(), "OFF" | "SWING" | "AUTO" | "ON" | "ALL" | "STOP")
}

/// A change to send to the air conditioner.
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Power(bool),
    Mode(String),
    /// A target temperature in the given unit, for the current mode.
    Target(f64, Unit),
    Wind(String),
    PowerSave(bool),
    DisplayLight(bool),
    WindFlag(String, bool),
    /// Park a vane (`windDirection.<key>`) at a position.
    Vane(String, String),
    /// A numeric fan level.
    WindStep(f64),
    AirClean(bool),
    /// Set a relative timer to this many minutes.
    TimerIn(Timer, u32),
    TimerCancel(Timer),
}

impl Command {
    /// The JSON body for `POST /control`, or why the device can't take it.
    pub fn payload(&self, profile: &Profile, state: &State) -> Result<Value, &'static str> {
        match self {
            Command::Power(on) => {
                if !profile.can_power() {
                    return Err("This unit can't be switched on and off remotely.");
                }
                Ok(json!({"operation": {"airConOperationMode": if *on { "POWER_ON" } else { "POWER_OFF" }}}))
            }
            Command::Mode(mode) => {
                if !profile.modes().iter().any(|m| m == mode) {
                    return Err("This unit doesn't have that mode.");
                }
                Ok(json!({"airConJobMode": {"currentJobMode": mode}}))
            }
            Command::Target(value, unit) => {
                let unit = profile.control_unit(*unit).ok_or("This unit's temperature can't be set remotely.")?;
                let key = profile.target_key(unit, state.mode()).ok_or("This unit's temperature can't be set remotely.")?;
                let range = profile.target_range(unit, key).ok_or("This unit's temperature can't be set remotely.")?;
                if !value.is_finite() {
                    return Err("Not a temperature.");
                }
                let v = range.snap(*value);
                let num = if v.fract() == 0.0 { json!(v as i64) } else { json!(v) };
                let mut body = Map::new();
                body.insert(key.to_string(), num);
                body.insert("unit".into(), json!(unit.as_str()));
                Ok(Value::Object(Map::from_iter([(profile.temp_resource().key().to_string(), Value::Object(body))])))
            }
            Command::Wind(speed) => {
                let key = profile.wind_key_for(speed);
                if !profile.wind_speeds().iter().any(|s| s == speed) {
                    return Err("This unit doesn't have that fan speed.");
                }
                Ok(json!({"airFlow": {key: speed}}))
            }
            Command::PowerSave(on) => {
                if !profile.writable("powerSave", "powerSaveEnabled") {
                    return Err("This unit has no energy saver.");
                }
                Ok(json!({"powerSave": {"powerSaveEnabled": on}}))
            }
            Command::DisplayLight(on) => {
                if !profile.can_display_light() {
                    return Err("This unit's display light can't be changed.");
                }
                Ok(json!({"display": {"light": if *on { "ON" } else { "OFF" }}}))
            }
            Command::WindFlag(key, on) => {
                if !profile.wind_flags().iter().any(|(k, _)| k == key) {
                    return Err("This unit doesn't have that airflow option.");
                }
                Ok(json!({"windDirection": {key.as_str(): on}}))
            }
            Command::Vane(key, pos) => {
                if !profile.vanes().iter().any(|v| &v.key == key && v.positions.iter().any(|p| p == pos)) {
                    return Err("This unit's louvre can't take that position.");
                }
                Ok(json!({"windDirection": {key.as_str(): pos}}))
            }
            Command::WindStep(n) => {
                let r = profile.wind_step().ok_or("This unit has no fan levels.")?;
                if !n.is_finite() {
                    return Err("Not a fan level.");
                }
                Ok(json!({"airFlow": {"windStep": r.snap(*n) as i64}}))
            }
            Command::AirClean(on) => {
                if !profile.can_air_clean() {
                    return Err("This unit has no air purifier.");
                }
                Ok(json!({"operation": {"airCleanOperationMode": if *on { "START" } else { "STOP" }}}))
            }
            Command::TimerIn(which, minutes) => {
                if !profile.can_timer(*which) {
                    return Err("This unit doesn't have that timer.");
                }
                if *minutes == 0 || *minutes > 24 * 60 {
                    return Err("Timers run from a minute to a day.");
                }
                let (res, h, m, _) = which.keys();
                let (hours, mins) = (minutes / 60, minutes % 60);
                let mut body = Map::new();
                body.insert(h.into(), json!(hours));
                // LG's SDK leaves out a zero minute on stop timers.
                if mins != 0 || *which == Timer::Start || *which == Timer::Sleep {
                    body.insert(m.into(), json!(mins));
                }
                Ok(Value::Object(Map::from_iter([(res.to_string(), Value::Object(body))])))
            }
            Command::TimerCancel(which) => {
                if !profile.can_cancel(*which) {
                    return Err("That timer can't be cancelled remotely.");
                }
                let (res, _, _, flag) = which.keys();
                Ok(json!({res: {flag: "UNSET"}}))
            }
        }
    }

    /// What the change does to the state the UI shows until the device reports back.
    pub fn optimistic(&self, payload: &Value) -> Value {
        match self {
            Command::TimerIn(which, _) => {
                let (res, _, _, flag) = which.keys();
                let mut p = payload.clone();
                if let Some(o) = p.get_mut(res).and_then(Value::as_object_mut) {
                    o.insert(flag.into(), json!("SET"));
                }
                p
            }
            // Both speed properties show the new speed until the unit reports back.
            Command::Wind(speed) => json!({"airFlow": {"windStrength": speed, "windStrengthDetail": speed}}),
            _ => payload.clone(),
        }
    }
}

/// A short label for a ThinQ enum value.
pub fn label(value: &str) -> String {
    let known = match value {
        "COOL" => "Cool",
        "HEAT" => "Heat",
        "AUTO" => "Auto",
        "AIR_DRY" => "Dry",
        "FAN" => "Fan",
        "AIR_CLEAN" => "Purify",
        "ENERGY_SAVING" => "Eco",
        "AROMA" => "Aroma",
        "LOW" => "Low",
        "MID" => "Medium",
        "HIGH" => "High",
        "SLOW" => "Slow",
        "SLOW_LOW" => "Slow+",
        "LOW_MID" => "Low+",
        "MID_HIGH" => "Medium–high",
        "HIGH_POWER" => "High+",
        "POWER" => "Turbo",
        "NATURE" => "Natural",
        "POWER_ON" => "On",
        "POWER_OFF" => "Off",
        "INITIAL" => "Ready",
        "END" => "Done",
        "CLOSE" => "Closed",
        "DETECTING" => "Sensing the load",
        "RESERVED" => "Scheduled",
        "ON_WORKING" => "While running",
        _ => "",
    };
    if !known.is_empty() {
        return known.to_string();
    }
    let lower = value.replace('_', " ").to_lowercase();
    let mut c = lower.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
