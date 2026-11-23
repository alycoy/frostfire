use serde_json::json;

use super::*;

fn fixture(name: &str) -> (Spec, Value) {
    let path = format!("{}/tests/fixtures/devices/{name}.json", env!("CARGO_MANIFEST_DIR"));
    let v: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    (Spec::parse(&v["profile"]), v["state"].clone())
}

fn path(scope: Scope, res: &str, item: Option<(&str, &str)>, key: &str) -> Path {
    Path { scope, res: res.into(), item: item.map(|(a, b)| (a.into(), b.into())), key: key.into() }
}

#[test]
fn plain_profiles_read_and_change() {
    let (spec, state) = fixture("purifier");
    assert_eq!(spec.zones.len(), 1);
    let power = spec.power().expect("a power switch");
    assert_eq!(power.path.key, "airPurifierOperationMode");
    assert_eq!(read(&state, &power.path), Some(&json!("POWER_ON")));
    let fan = path(Scope::Root, "airFlow", None, "windStrength");
    assert_eq!(spec.payload(&fan, json!("HIGH")).unwrap(), json!({"airFlow": {"windStrength": "HIGH"}}));
    assert!(spec.payload(&fan, json!("TURBO")).is_err(), "not a value the profile lists");
    // Read-only sensors can't be sent.
    assert!(spec.payload(&path(Scope::Root, "airQualitySensor", None, "PM2"), json!(3)).is_err());
    // Ranges clamp and snap.
    let sleep = path(Scope::Root, "sleepTimer", None, "relativeHourToStop");
    assert_eq!(spec.payload(&sleep, json!(40)).unwrap(), json!({"sleepTimer": {"relativeHourToStop": 12}}));
}

#[test]
fn the_one_humidity_target_gets_the_dial() {
    let (spec, state) = fixture("dehumidifier");
    let dial = spec.dial(Unit::C).expect("a dial");
    assert_eq!(dial.path.key, "targetHumidity");
    assert_eq!(read(&state, &dial.path), Some(&json!(45)));
    assert_eq!(spec.payload(&dial.path, json!(52)).unwrap(), json!({"humidity": {"targetHumidity": 50}}));
}

#[test]
fn per_location_profiles_name_their_location() {
    let (spec, state) = fixture("washer");
    assert_eq!(spec.zones[0].scope, Scope::Location("MAIN".into()));
    let run = spec.find("runState", "currentState").unwrap();
    assert_eq!(read(&state, &run.path), Some(&json!("RUNNING")));
    let op = spec.find("operation", "washerOperationMode").unwrap();
    assert!(op.writable() && !op.prop.readable);
    assert!(spec.power().is_none(), "START/STOP isn't a power switch");
    assert_eq!(spec.payload(&op.path, json!("STOP")).unwrap(), json!({"location": {"locationName": "MAIN"}, "operation": {"washerOperationMode": "STOP"}}));
}

#[test]
fn list_entries_are_tagged_and_the_other_unit_hides() {
    let (spec, mut state) = fixture("fridge");
    let fridge_c = path(Scope::Root, "temperatureInUnits", Some(("locationName", "FRIDGE")), "targetTemperatureC");
    let fridge_f = path(Scope::Root, "temperatureInUnits", Some(("locationName", "FRIDGE")), "targetTemperatureF");
    assert_eq!(read(&state, &fridge_c), Some(&json!(3)));
    assert!(spec.visible(spec.field(&fridge_c).unwrap(), Unit::C));
    assert!(!spec.visible(spec.field(&fridge_f).unwrap(), Unit::C));
    assert!(spec.visible(spec.field(&fridge_f).unwrap(), Unit::F));
    assert!(spec.dial(Unit::C).is_none(), "two compartments, two steppers");
    let payload = spec.payload(&fridge_c, json!(5)).unwrap();
    assert_eq!(payload, json!({"temperatureInUnits": {"locationName": "FRIDGE", "targetTemperatureC": 5}}));
    merge(&mut state, &payload);
    assert_eq!(read(&state, &fridge_c), Some(&json!(5)));
    let freezer = path(Scope::Root, "temperatureInUnits", Some(("locationName", "FREEZER")), "targetTemperatureC");
    assert_eq!(read(&state, &freezer), Some(&json!(-18)), "the other compartment is untouched");
    let door = spec.find("doorStatus", "doorState").unwrap();
    assert_eq!(read(&state, &door.path), Some(&json!("CLOSE")));
}

#[test]
fn oven_temperatures_go_with_their_location_and_unit() {
    let (spec, mut state) = fixture("oven");
    assert_eq!(spec.zones.len(), 2);
    assert_eq!(spec.zones[0].title().as_deref(), Some("Upper"));
    let t = path(Scope::Location("UPPER".into()), "temperature", Some(("unit", "C")), "targetTemperature");
    assert!(spec.visible(spec.field(&t).unwrap(), Unit::C));
    let f = path(Scope::Location("UPPER".into()), "temperature", Some(("unit", "F")), "targetTemperature");
    assert!(!spec.visible(spec.field(&f).unwrap(), Unit::C));
    let payload = spec.payload(&t, json!(200)).unwrap();
    assert_eq!(payload, json!({"location": {"locationName": "UPPER"}, "temperature": {"unit": "C", "targetTemperature": 200}}));
    merge(&mut state, &payload);
    assert_eq!(read(&state, &t), Some(&json!(200)));
}

#[test]
fn combined_devices_split_into_parts() {
    let (spec, state) = fixture("washtower");
    assert_eq!(spec.zones.len(), 2);
    let dryer = spec.zones.iter().find(|z| z.scope == Scope::Sub("dryer".into())).unwrap();
    let run = dryer.fields.iter().find(|f| f.path.key == "currentState").unwrap();
    assert_eq!(read(&state, &run.path), Some(&json!("DRYING")));
    let op = dryer.fields.iter().find(|f| f.path.res == "operation").unwrap();
    assert_eq!(spec.payload(&op.path, json!("START")).unwrap(), json!({"dryer": {"operation": {"dryerOperationMode": "START"}}}));
}

#[test]
fn words() {
    assert_eq!(humanize("relativeHourToStart"), "Relative hour to start");
    assert_eq!(humanize("PM10Level"), "PM10 level");
    assert_eq!(name("airQualitySensor", "PM2"), "PM2.5");
    assert_eq!(name("operation", "washerOperationMode"), "Operation");
    assert_eq!(place("CENTER_FRONT"), "Center front");
}

#[test]
fn hostile_profiles_dont_panic() {
    for v in [
        json!(null),
        json!([]),
        json!({"property": 3}),
        json!({"property": [1, "x", {"a": 2}]}),
        json!({"property": {"r": {"k": {"type": "range", "value": {"w": {"min": 5, "max": 1}}}}}}),
    ] {
        let spec = Spec::parse(&v);
        for f in spec.fields() {
            let _ = spec.payload(&f.path, json!(1));
        }
    }
}
