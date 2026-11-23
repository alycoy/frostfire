//! Any LG device, read from its own profile: every property it reports, where it lives in the
//! state, and how to change it. The app draws its pane from this, so a device type the app has
//! never seen still shows everything LG says it can read and change.
//!
//! LG shapes profiles four ways (see its SDK, `devices/connect_device.py`):
//! - plain: `{"property": {resource: {key: prop}}}`, changed with `{resource: {key: value}}`
//! - per location: `{"property": [{"location": {"locationName": "UPPER"}, resource: {…}}, …]}`
//!   (ovens, cooktops, washers), changed with `{"location": {"locationName": …}, resource: {…}}`
//! - per part: `{"washer": {"property": …}, "dryer": {"property": …}}` (WashTower), changed with
//!   `{"washer": {resource: {…}}}`
//! - per entry: a resource that is a list, each entry tagged by `locationName` or `unit`
//!   (fridge compartments, temperatures), changed with `{resource: {tag: …, key: value}}`
//!
//! Every change is checked against the profile before it's built: only writable properties,
//! only values the device lists, numbers clamped and snapped to its range.

use serde_json::{Map, Value, json};

use super::model::{Kind, Prop, Range, Unit, label, parse_prop};

/// Which part of a device a property belongs to.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Scope {
    Root,
    /// An entry of a per-location profile (`location.locationName`).
    Location(String),
    /// A part of a combined device (`washer`, `dryer`).
    Sub(String),
}

/// Where one property lives: enough to read it from the state and to build its change.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Path {
    pub scope: Scope,
    pub res: String,
    /// For a resource that is a list: the tag that picks the entry (`("locationName", "FRIDGE")`).
    pub item: Option<(String, String)>,
    pub key: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub path: Path,
    pub prop: Prop,
    /// The unit the profile gives (`"C"`, `"F"`, `"%"`…).
    pub unit: Option<String>,
    /// A bare value in the profile: a read-only constant.
    pub constant: Option<String>,
    /// Range values the device refuses.
    pub except: Vec<f64>,
}

impl Field {
    pub fn writable(&self) -> bool {
        self.prop.writable
    }

    /// The writable values of an enum.
    pub fn choices(&self) -> &[String] {
        match &self.prop.kind {
            Kind::Enum { write, .. } if self.prop.writable => write,
            _ => &[],
        }
    }

    pub fn range(&self) -> Option<Range> {
        match &self.prop.kind {
            Kind::Range { write, read } => write.or(*read),
            _ => None,
        }
    }

    /// The temperature unit this field is in, when it says.
    pub fn temp_unit(&self) -> Option<Unit> {
        if let Some(("unit", u)) = self.path.item.as_ref().map(|(k, v)| (k.as_str(), v.as_str())) {
            return Unit::parse(u);
        }
        if let Some(u) = self.unit.as_deref().and_then(Unit::parse) {
            return Some(u);
        }
        let k = &self.path.key;
        if k.to_ascii_lowercase().contains("temp") {
            return match k.chars().last() {
                Some('C') => Some(Unit::C),
                Some('F') => Some(Unit::F),
                _ => None,
            };
        }
        None
    }
}

/// One part of a device and its properties, in the profile's order.
#[derive(Clone, Debug, PartialEq)]
pub struct Zone {
    pub scope: Scope,
    pub fields: Vec<Field>,
}

impl Zone {
    /// "Upper", "Washer"; `None` for the device itself.
    pub fn title(&self) -> Option<String> {
        match &self.scope {
            Scope::Root => None,
            Scope::Location(n) | Scope::Sub(n) => Some(place(n)),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Spec {
    pub zones: Vec<Zone>,
}

impl Spec {
    pub fn parse(v: &Value) -> Spec {
        let mut zones = Vec::new();
        let props = v.get("extensionProperty").filter(|p| !p.is_null()).or_else(|| v.get("property"));
        match props {
            Some(Value::Array(list)) => {
                for (i, entry) in list.iter().enumerate() {
                    let Some(obj) = entry.as_object() else { continue };
                    let name = entry
                        .get("location")
                        .and_then(|l| l.get("locationName"))
                        .and_then(Value::as_str)
                        .map(str::to_string)
                        .unwrap_or_else(|| format!("UNIT_{}", i + 1));
                    let scope = Scope::Location(name);
                    zones.push(Zone { fields: fields_of(&scope, obj), scope });
                }
            }
            Some(Value::Object(obj)) => zones.push(Zone { scope: Scope::Root, fields: fields_of(&Scope::Root, obj) }),
            _ => {
                for (part, sub) in v.as_object().into_iter().flatten() {
                    if let Some(obj) = sub.get("property").and_then(Value::as_object) {
                        let scope = Scope::Sub(part.clone());
                        zones.push(Zone { fields: fields_of(&scope, obj), scope });
                    }
                }
            }
        }
        zones.retain(|z| !z.fields.is_empty());
        Spec { zones }
    }

    pub fn fields(&self) -> impl Iterator<Item = &Field> {
        self.zones.iter().flat_map(|z| z.fields.iter())
    }

    pub fn field(&self, path: &Path) -> Option<&Field> {
        self.fields().find(|f| &f.path == path)
    }

    /// The first field with this resource and key, anywhere.
    pub fn find(&self, res: &str, key: &str) -> Option<&Field> {
        self.fields().find(|f| f.path.res == res && f.path.key == key)
    }

    /// The device's on/off switch: an `operation` enum that takes both `POWER_ON` and `POWER_OFF`.
    pub fn power(&self) -> Option<&Field> {
        self.fields().find(|f| {
            let c = f.choices();
            f.path.res == "operation" && c.iter().any(|v| v == "POWER_ON") && c.iter().any(|v| v == "POWER_OFF")
        })
    }

    /// The one target worth a dial (a humidity or temperature to hold), in the preferred unit.
    /// Devices with several targets (a fridge's compartments) get steppers instead.
    pub fn dial(&self, unit: Unit) -> Option<&Field> {
        let candidates: Vec<&Field> = self
            .fields()
            .filter(|f| {
                let k = f.path.key.to_ascii_lowercase();
                f.writable() && f.range().is_some() && k.contains("target") && (k.contains("humidity") || k.contains("temp")) && self.visible(f, unit)
            })
            .collect();
        (candidates.len() == 1).then(|| candidates[0])
    }

    /// Whether the pane shows this field: not the bookkeeping (units, tags), not the other
    /// temperature unit's twin, and not something LG says is neither readable nor writable.
    pub fn visible(&self, f: &Field, unit: Unit) -> bool {
        let k = f.path.key.as_str();
        if matches!(k, "unit" | "locationName") || matches!(f.prop.kind, Kind::Other) || !(f.prop.readable || f.prop.writable) {
            return false;
        }
        let other = match unit {
            Unit::C => Unit::F,
            Unit::F => Unit::C,
        };
        // `targetTemperatureF` beside `targetTemperatureC`: keep the preferred one.
        if f.temp_unit() == Some(other) {
            let twin = |u: Unit| {
                self.fields().any(|g| {
                    g.path.scope == f.path.scope && g.path.res == f.path.res && g.temp_unit() == Some(u) && stem(&g.path.key) == stem(k) && {
                        // Same entry, or the matching entry of a per-unit list.
                        match (&g.path.item, &f.path.item) {
                            (Some((gk, _)), Some((fk, _))) if gk == "unit" && fk == "unit" => true,
                            (a, b) => a == b,
                        }
                    }
                })
            };
            if twin(unit) {
                return false;
            }
        }
        true
    }

    /// The change that sets `path` to `value`, or why the device won't take it.
    pub fn payload(&self, path: &Path, value: Value) -> Result<Value, &'static str> {
        let f = self.field(path).ok_or("This device doesn't have that setting.")?;
        if !f.writable() {
            return Err("That can't be changed remotely.");
        }
        let value = match &f.prop.kind {
            Kind::Enum { write, .. } => match value.as_str() {
                Some(s) if write.iter().any(|w| w == s) => value,
                _ => return Err("This device doesn't take that value."),
            },
            Kind::Range { .. } => {
                let r = f.range().ok_or("That can't be changed remotely.")?;
                let n = value.as_f64().filter(|n| n.is_finite()).ok_or("Not a number.")?;
                let v = r.snap(n);
                if f.except.iter().any(|e| (e - v).abs() < 1e-9) {
                    return Err("This device doesn't take that value.");
                }
                number(v)
            }
            Kind::Bool => Value::Bool(value.as_bool().ok_or("Not on or off.")?),
            Kind::Number => match value.as_f64() {
                Some(n) if n.is_finite() && n.abs() < 1e6 => number(n),
                _ => return Err("Not a number."),
            },
            Kind::Text | Kind::Other => return Err("That setting can't be changed from here."),
        };
        Ok(body(path, value))
    }
}

fn number(v: f64) -> Value {
    if v.fract() == 0.0 { json!(v as i64) } else { json!(v) }
}

/// A key without its unit letter: `targetTemperatureC` → `targetTemperature`.
fn stem(k: &str) -> &str {
    k.strip_suffix('C').or_else(|| k.strip_suffix('F')).unwrap_or(k)
}

fn body(path: &Path, value: Value) -> Value {
    let mut inner = Map::new();
    if let Some((tk, tv)) = &path.item {
        inner.insert(tk.clone(), json!(tv));
    }
    inner.insert(path.key.clone(), value);
    let mut root = Map::new();
    if let Scope::Location(n) = &path.scope {
        root.insert("location".into(), json!({"locationName": n}));
    }
    root.insert(path.res.clone(), Value::Object(inner));
    match &path.scope {
        Scope::Sub(part) => json!({part.as_str(): Value::Object(root)}),
        _ => Value::Object(root),
    }
}

fn fields_of(scope: &Scope, obj: &Map<String, Value>) -> Vec<Field> {
    let mut out = Vec::new();
    for (res, body) in obj {
        if res == "location" {
            continue;
        }
        match body {
            Value::Object(props) => {
                let unit = props.get("unit").and_then(Value::as_str);
                for (key, raw) in props {
                    out.push(field(scope, res, None, key, raw, unit));
                }
            }
            Value::Array(entries) => {
                for entry in entries {
                    let Some(e) = entry.as_object() else { continue };
                    let tag = ["locationName", "unit"].iter().find_map(|t| e.get(*t).and_then(Value::as_str).map(|v| (t.to_string(), v.to_string())));
                    let unit = e.get("unit").and_then(Value::as_str);
                    for (key, raw) in e {
                        if tag.as_ref().is_some_and(|(t, _)| t == key) {
                            continue;
                        }
                        out.push(field(scope, res, tag.clone(), key, raw, unit));
                    }
                }
            }
            _ => {}
        }
    }
    out
}

fn field(scope: &Scope, res: &str, item: Option<(String, String)>, key: &str, raw: &Value, unit: Option<&str>) -> Field {
    let prop = parse_prop(raw);
    let constant = match raw {
        Value::String(s) => Some(s.chars().take(64).collect()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    };
    let except = raw
        .get("value")
        .and_then(|v| v.get("w"))
        .and_then(|w| w.get("except"))
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_f64).collect())
        .unwrap_or_default();
    let unit = raw.get("unit").and_then(Value::as_str).or(unit).map(str::to_string);
    Field { path: Path { scope: scope.clone(), res: res.into(), item, key: key.into() }, prop, unit, constant, except }
}

// ---------------------------------------------------------------------------------- state

fn location_of(v: &Value) -> Option<&str> {
    v.get("location").and_then(|l| l.get("locationName")).and_then(Value::as_str)
}

fn zone_state<'a>(state: &'a Value, scope: &Scope) -> Option<&'a Value> {
    match scope {
        Scope::Root => Some(state),
        Scope::Location(n) => match state {
            Value::Array(a) => a.iter().find(|e| location_of(e) == Some(n)),
            Value::Object(_) if location_of(state) == Some(n) => Some(state),
            _ => None,
        },
        Scope::Sub(part) => state.get(part),
    }
}

/// The value a state holds for a field.
pub fn read<'a>(state: &'a Value, p: &Path) -> Option<&'a Value> {
    let r = zone_state(state, &p.scope)?.get(&p.res)?;
    match &p.item {
        None => r.get(&p.key),
        Some((tk, tv)) => match r {
            Value::Array(a) => a.iter().find(|e| e.get(tk).and_then(Value::as_str) == Some(tv))?.get(&p.key),
            Value::Object(_) if r.get(tk).and_then(Value::as_str) == Some(tv) => r.get(&p.key),
            _ => None,
        },
    }
}

/// Merges a change into a state, the way the device will: into the right location, part and
/// list entry. Used for the optimistic update and by the demo devices.
pub fn merge(state: &mut Value, payload: &Value) {
    if let (Some(loc), Value::Array(list)) = (location_of(payload), &mut *state) {
        if let Some(entry) = list.iter_mut().find(|e| location_of(e) == Some(loc)) {
            let mut rest = payload.clone();
            if let Some(o) = rest.as_object_mut() {
                o.remove("location");
            }
            deep(entry, &rest);
        }
        return;
    }
    deep(state, payload);
}

fn deep(target: &mut Value, patch: &Value) {
    match (target, patch) {
        (Value::Object(to), Value::Object(po)) => {
            for (k, v) in po {
                match to.get_mut(k) {
                    Some(existing) => deep(existing, v),
                    None => {
                        to.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        (Value::Array(list), Value::Object(po)) => {
            let tag = ["locationName", "unit"].iter().find_map(|t| po.get(*t).map(|v| (*t, v)));
            if let Some((tk, tv)) = tag
                && let Some(entry) = list.iter_mut().find(|e| e.get(tk) == Some(tv))
            {
                deep(entry, patch);
            }
        }
        (t, p) => *t = p.clone(),
    }
}

// ---------------------------------------------------------------------------------- words

/// A place name: `UPPER` → "Upper", `CENTER_FRONT` → "Center front", `washer` → "Washer".
pub fn place(n: &str) -> String {
    match n {
        "FRIDGE" => "Fridge".into(),
        "FREEZER" => "Freezer".into(),
        "CONVERTIBLE" => "Convertible".into(),
        "MAIN" => "Main".into(),
        "MINI" => "Mini".into(),
        _ => label(n),
    }
}

/// A property's name, for people: `currentJobMode` → "Mode", `PM2` → "PM2.5".
pub fn name(res: &str, key: &str) -> String {
    let known = match key {
        "currentJobMode" => "Mode",
        "currentState" => "Status",
        "windStrength" | "windStrengthDetail" | "windStrengthLevel" => "Fan speed",
        "windStep" => "Fan level",
        "currentHumidity" | "humidity" => "Humidity",
        "targetHumidity" => "Target humidity",
        "currentTemperature" | "currentTemperatureC" | "currentTemperatureF" => "Temperature",
        "targetTemperature" | "targetTemperatureC" | "targetTemperatureF" => "Target temperature",
        "PM1" => "PM1",
        "PM2" => "PM2.5",
        "PM10" => "PM10",
        "PM1Level" => "PM1 level",
        "PM2Level" => "PM2.5 level",
        "PM10Level" => "PM10 level",
        "totalPollution" => "Pollution",
        "totalPollutionLevel" => "Air quality",
        "remoteControlEnabled" => "Remote control",
        "doorState" => "Door",
        "percent" if res == "battery" => "Battery",
        "level" if res == "battery" => "Battery level",
        "powerSaveEnabled" => "Energy saver",
        "light" if res == "display" => "Display light",
        "monitoringEnabled" => "Air monitoring",
        "cycleCount" => "Cycles run",
        "childLockEnabled" | "childLock" => "Child lock",
        "relativeHourToStart" => "Start in (hours)",
        "relativeMinuteToStart" => "Start in (minutes)",
        "relativeHourToStop" => "Stop in (hours)",
        "relativeMinuteToStop" => "Stop in (minutes)",
        "absoluteHourToStart" => "Start at (hour)",
        "absoluteMinuteToStart" => "Start at (minute)",
        "absoluteHourToStop" => "Stop at (hour)",
        "absoluteMinuteToStop" => "Stop at (minute)",
        "filterRemainPercent" => "Filter",
        "usedTime" => "Filter used",
        _ => "",
    };
    if !known.is_empty() {
        return known.into();
    }
    if key.ends_with("OperationMode") {
        return "Operation".into();
    }
    if key.ends_with("JobMode") {
        return "Mode".into();
    }
    humanize(key)
}

/// `relativeHourToStart` → "Relative hour to start".
pub fn humanize(key: &str) -> String {
    let mut words = String::new();
    let chars: Vec<char> = key.chars().collect();
    for (i, ch) in chars.iter().enumerate() {
        let next_lower = chars.get(i + 1).is_some_and(|c| c.is_lowercase());
        let prev_lower = i > 0 && (chars[i - 1].is_lowercase() || chars[i - 1].is_ascii_digit());
        if i > 0 && ch.is_uppercase() && (prev_lower || next_lower) {
            words.push(' ');
        }
        if *ch == '_' {
            words.push(' ');
            continue;
        }
        words.push(*ch);
    }
    let lower: String = words
        .split(' ')
        .filter(|w| !w.is_empty())
        .map(|w| if w.len() > 1 && w.chars().all(|c| c.is_uppercase() || c.is_ascii_digit()) { w.to_string() } else { w.to_lowercase() })
        .collect::<Vec<_>>()
        .join(" ");
    let mut c = lower.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

/// An enum value that does something rather than describing a setting.
pub fn is_action(v: &str) -> bool {
    matches!(v, "START" | "STOP" | "PAUSE" | "RESUME" | "WAKE_UP" | "POWER_ON" | "POWER_OFF" | "CANCEL" | "HOMING" | "SLEEP" | "CLEAN_START" | "SET" | "UNSET")
}

/// A button's words for an action value.
pub fn action_label(v: &str) -> String {
    match v {
        "POWER_ON" => "Turn on".into(),
        "POWER_OFF" => "Turn off".into(),
        "WAKE_UP" => "Wake up".into(),
        "HOMING" => "Send home".into(),
        "CLEAN_START" => "Start cleaning".into(),
        "UNSET" => "Clear".into(),
        _ => label(v),
    }
}

/// A value as the pane shows it, with its unit.
pub fn show(f: &Field, v: &Value) -> String {
    let k = f.path.key.to_ascii_lowercase();
    match v {
        Value::Bool(b) => (if *b { "On" } else { "Off" }).into(),
        Value::String(s) => {
            if matches!(f.prop.kind, Kind::Enum { .. }) || s.chars().all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit()) {
                label(s)
            } else {
                s.chars().take(60).collect()
            }
        }
        Value::Number(n) => {
            let x = n.as_f64().unwrap_or(0.0);
            let num = if (x - x.round()).abs() < 0.05 { format!("{}", x.round() as i64) } else { format!("{x:.1}") };
            // A real minus sign: the hyphen sits too low and too short beside digits.
            let num = num.replace('-', "−");
            if k.contains("temp") {
                format!("{num}°")
            } else if k.contains("humidity") || k.contains("percent") || f.unit.as_deref() == Some("%") {
                format!("{num}%")
            } else if k.starts_with("pm") && !k.ends_with("level") {
                format!("{num} µg/m³")
            } else {
                match f.unit.as_deref() {
                    Some(u) if !u.is_empty() && u.len() <= 6 && Unit::parse(u).is_none() => format!("{num} {u}"),
                    _ => num,
                }
            }
        }
        _ => "—".into(),
    }
}

#[cfg(test)]
#[path = "spec_tests.rs"]
mod tests;
