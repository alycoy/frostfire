use super::*;
use serde_json::json;

fn fixture(name: &str) -> Value {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn profile() -> Profile {
    Profile::parse(&fixture("ac_profile.json"))
}

fn state() -> State {
    State(fixture("ac_state.json"))
}

#[test]
fn devices_keep_safe_ids_only() {
    let d = parse_devices(&fixture("devices.json"));
    assert_eq!(d.len(), 2, "the ../ id is dropped");
    assert!(d[0].is_ac());
    assert_eq!(d[0].alias, "Bedroom");
    assert!(!d[1].is_ac());
}

#[test]
fn profile_reads_capabilities() {
    let p = profile();
    assert_eq!(p.modes(), ["COOL", "AIR_DRY", "FAN", "HEAT", "AUTO"]);
    assert!(p.can_power());
    assert!(p.can_air_clean());
    assert!(p.can_display_light());
    assert_eq!(p.wind_speeds(), ["LOW", "MID", "HIGH", "AUTO"]);
    assert_eq!(p.wind_flags().iter().map(|f| f.0).collect::<Vec<_>>(), ["rotateUpDown", "rotateLeftRight"]);
    assert_eq!(p.temp_resource(), TempResource::InUnits);
    assert_eq!(p.units(), [Unit::C, Unit::F]);
    assert!(p.can_timer(Timer::Stop) && p.can_timer(Timer::Sleep) && p.can_timer(Timer::Start));
    assert!(p.can_cancel(Timer::Stop));
}

#[test]
fn target_key_follows_the_mode() {
    let p = profile();
    assert_eq!(p.target_key(Unit::C, Some("COOL")), Some("coolTargetTemperature"));
    assert_eq!(p.target_key(Unit::C, Some("HEAT")), Some("heatTargetTemperature"));
    assert_eq!(p.target_key(Unit::C, Some("AUTO")), Some("targetTemperature"), "no auto target: the plain one");
    assert_eq!(p.target_key(Unit::C, Some("FAN")), Some("targetTemperature"));
    let r = p.target_range(Unit::C, "coolTargetTemperature").unwrap();
    assert_eq!((r.min, r.max, r.step), (18.0, 30.0, 0.5));
}

#[test]
fn state_reads_values() {
    let s = state();
    assert_eq!(s.power(), Some(true));
    assert_eq!(s.mode(), Some("COOL"));
    assert_eq!(s.temp("currentTemperature", Unit::C), Some(26.5));
    assert_eq!(s.temp("currentTemperature", Unit::F), Some(80.0));
    assert_eq!(s.temp("targetTemperature", Unit::C), Some(23.0));
    assert_eq!(s.timer_minutes(Timer::Stop), Some(90));
    assert_eq!(s.timer_minutes(Timer::Sleep), None);
    assert_eq!(s.filter_percent(), Some(75.0));
    assert_eq!(s.num("airQualitySensor", "humidity"), Some(48.0));
    assert_eq!(s.bool("windDirection", "rotateUpDown"), Some(true));
}

#[test]
fn temperatures_convert_when_only_one_unit_is_reported() {
    let s = State(json!({"temperature": {"currentTemperature": 25, "targetTemperature": 20, "unit": "C"}}));
    assert_eq!(s.temp("currentTemperature", Unit::F), Some(77.0));
    assert_eq!(s.temp("targetTemperature", Unit::C), Some(20.0));
}

#[test]
fn commands_build_lg_payloads() {
    let (p, s) = (profile(), state());
    assert_eq!(Command::Power(false).payload(&p, &s).unwrap(), json!({"operation": {"airConOperationMode": "POWER_OFF"}}));
    assert_eq!(Command::Mode("HEAT".into()).payload(&p, &s).unwrap(), json!({"airConJobMode": {"currentJobMode": "HEAT"}}));
    assert_eq!(Command::Target(22.5, Unit::C).payload(&p, &s).unwrap(), json!({"temperatureInUnits": {"coolTargetTemperature": 22.5, "unit": "C"}}));
    assert_eq!(
        Command::Target(72.4, Unit::F).payload(&p, &s).unwrap(),
        json!({"temperatureInUnits": {"coolTargetTemperature": 72, "unit": "F"}}),
        "snapped to a whole degree and sent as an integer"
    );
    assert_eq!(Command::Wind("HIGH".into()).payload(&p, &s).unwrap(), json!({"airFlow": {"windStrength": "HIGH"}}));
    assert_eq!(Command::PowerSave(true).payload(&p, &s).unwrap(), json!({"powerSave": {"powerSaveEnabled": true}}));
    assert_eq!(Command::DisplayLight(false).payload(&p, &s).unwrap(), json!({"display": {"light": "OFF"}}));
    assert_eq!(Command::WindFlag("rotateLeftRight".into(), true).payload(&p, &s).unwrap(), json!({"windDirection": {"rotateLeftRight": true}}));
    assert_eq!(Command::AirClean(true).payload(&p, &s).unwrap(), json!({"operation": {"airCleanOperationMode": "START"}}));
    assert_eq!(Command::TimerIn(Timer::Stop, 120).payload(&p, &s).unwrap(), json!({"timer": {"relativeHourToStop": 2}}));
    assert_eq!(Command::TimerIn(Timer::Sleep, 90).payload(&p, &s).unwrap(), json!({"sleepTimer": {"relativeHourToStop": 1, "relativeMinuteToStop": 30}}));
    assert_eq!(Command::TimerCancel(Timer::Stop).payload(&p, &s).unwrap(), json!({"timer": {"relativeStopTimer": "UNSET"}}));
}

#[test]
fn unsupported_or_out_of_range_commands_are_refused_or_clamped() {
    let (p, s) = (profile(), state());
    assert!(Command::Mode("AROMA".into()).payload(&p, &s).is_err());
    assert!(Command::Wind("POWER".into()).payload(&p, &s).is_err());
    assert!(Command::WindFlag("forestWind".into(), true).payload(&p, &s).is_err());
    assert!(Command::WindFlag("../x".into(), true).payload(&p, &s).is_err());
    assert!(Command::TimerIn(Timer::Stop, 0).payload(&p, &s).is_err());
    assert!(Command::TimerIn(Timer::Stop, 25 * 60).payload(&p, &s).is_err());
    assert!(Command::Target(f64::NAN, Unit::C).payload(&p, &s).is_err());
    assert_eq!(Command::Target(99.0, Unit::C).payload(&p, &s).unwrap(), json!({"temperatureInUnits": {"coolTargetTemperature": 30, "unit": "C"}}));
    assert_eq!(Command::Target(-5.0, Unit::C).payload(&p, &s).unwrap(), json!({"temperatureInUnits": {"coolTargetTemperature": 18, "unit": "C"}}));
}

#[test]
fn an_empty_profile_refuses_everything() {
    let (p, s) = (Profile::default(), State::default());
    for c in [
        Command::Power(true),
        Command::Mode("COOL".into()),
        Command::Target(22.0, Unit::C),
        Command::Wind("LOW".into()),
        Command::PowerSave(true),
        Command::DisplayLight(true),
        Command::AirClean(true),
        Command::TimerIn(Timer::Stop, 60),
        Command::TimerCancel(Timer::Sleep),
    ] {
        assert!(c.payload(&p, &s).is_err(), "{c:?}");
    }
}

#[test]
fn legacy_profiles_use_the_temperature_object() {
    let p = Profile::parse(&fixture("ac_legacy_profile.json"));
    let s = State(json!({"airConJobMode": {"currentJobMode": "COOL"}}));
    assert_eq!(p.temp_resource(), TempResource::Legacy);
    assert_eq!(p.wind_key_for("SLOW"), "windStrengthDetail");
    assert_eq!(p.control_unit(Unit::F), Some(Unit::C), "falls back to the unit it has");
    assert_eq!(Command::Target(21.0, Unit::C).payload(&p, &s).unwrap(), json!({"temperature": {"targetTemperature": 21, "unit": "C"}}));
    assert_eq!(Command::Wind("SLOW".into()).payload(&p, &s).unwrap(), json!({"airFlow": {"windStrengthDetail": "SLOW"}}));
}

#[test]
fn optimistic_updates_merge_into_state() {
    let (p, mut s) = (profile(), state());
    let c = Command::Target(21.0, Unit::C);
    s.apply(&c.optimistic(&c.payload(&p, &s).unwrap()));
    assert_eq!(s.temp("coolTargetTemperature", Unit::C), Some(21.0));
    assert_eq!(s.temp("coolTargetTemperature", Unit::F), Some(70.0), "the other unit follows, rounded to a half");
    assert_eq!(s.temp("currentTemperature", Unit::C), Some(26.5), "untouched");

    let c = Command::TimerIn(Timer::Sleep, 60);
    s.apply(&c.optimistic(&c.payload(&p, &s).unwrap()));
    assert_eq!(s.timer_minutes(Timer::Sleep), Some(60));

    let c = Command::Power(false);
    s.apply(&c.payload(&p, &s).unwrap());
    assert_eq!(s.power(), Some(false));
    assert_eq!(s.str("operation", "airCleanOperationMode"), Some("STOP"), "siblings kept");
}

#[test]
fn labels_read_well() {
    assert_eq!(label("AIR_DRY"), "Dry");
    assert_eq!(label("MID"), "Medium");
    assert_eq!(label("SOME_NEW_MODE"), "Some new mode");
}

#[test]
fn range_snaps_to_step() {
    let r = Range { min: 16.0, max: 30.0, step: 0.5 };
    assert_eq!(r.snap(22.3), 22.5);
    assert_eq!(r.snap(22.2), 22.0);
    assert_eq!(r.snap(100.0), 30.0);
}
