//! Frostfire's own icons: 24-point line drawings, tinted with `.color()`.

use rune::Icon;

const fn icon(name: &'static str, body: &'static str) -> Icon {
    Icon { name, view: 24.0, body }
}

macro_rules! stroke {
    ($d:literal) => {
        concat!(r#"<g fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">"#, $d, "</g>")
    };
}

pub static POWER: Icon = icon("ff-power", stroke!(r#"<path d="M12 3v8"/><path d="M6.4 6.6a8 8 0 1 0 11.2 0"/>"#));
pub static SNOW: Icon = icon(
    "ff-snow",
    stroke!(
        r#"<path d="M12 2.5v19M3.8 7.25l16.4 9.5M3.8 16.75l16.4-9.5"/><path d="M9.5 4.5 12 7l2.5-2.5M9.5 19.5 12 17l2.5 2.5M4.6 10.4l3.4-.9-.9-3.4M19.4 13.6l-3.4.9.9 3.4M4.6 13.6l3.4.9-.9 3.4M19.4 10.4l-3.4-.9.9-3.4"/>"#
    ),
);
pub static FLAME: Icon = icon(
    "ff-flame",
    stroke!(
        r#"<path d="M12 21.5c3.9 0 6.5-2.6 6.5-6.2 0-4.1-3.4-6.4-4.3-10.8-2.4 1.6-3.6 4-3.4 6.5-1.3-.7-2-1.9-2.1-3.3C6.9 9.3 5.5 11.6 5.5 15.3c0 3.6 2.6 6.2 6.5 6.2Z"/><path d="M12 21.5c-1.7 0-2.8-1.2-2.8-2.8 0-1.8 1.5-2.7 2.3-4.3 1.3 1 3.3 2.2 3.3 4.3 0 1.6-1.1 2.8-2.8 2.8Z"/>"#
    ),
);
pub static AUTO: Icon = icon("ff-auto", stroke!(r#"<path d="M20 12a8 8 0 1 1-2.3-5.7"/><path d="M20 4v4.5h-4.5"/><path d="M9 15l3-7 3 7M10 13h4"/>"#));
pub static DROP: Icon = icon("ff-drop", stroke!(r#"<path d="M12 3s6.5 7 6.5 11.5a6.5 6.5 0 0 1-13 0C5.5 10 12 3 12 3Z"/><path d="M9 15a3 3 0 0 0 3 3"/>"#));
pub static FAN: Icon = icon(
    "ff-fan",
    stroke!(
        r#"<circle cx="12" cy="12" r="1.6"/><path d="M12 10.4c-.6-3.7.6-6.9 3.3-6.9 2.4 0 3 3.3-.3 5.6M13.6 12c3.7-.6 6.9.6 6.9 3.3 0 2.4-3.3 3-5.6-.3M12 13.6c.6 3.7-.6 6.9-3.3 6.9-2.4 0-3-3.3.3-5.6M10.4 12c-3.7.6-6.9-.6-6.9-3.3 0-2.4 3.3-3 5.6.3"/>"#
    ),
);
pub static LEAF: Icon = icon("ff-leaf", stroke!(r#"<path d="M5 19c0-8 5-13.5 15-14-.5 10-6 15-14 15"/><path d="M5 19c3-4 6-6.5 9.5-8.5"/>"#));
pub static SPARKLE: Icon = icon("ff-sparkle", stroke!(r#"<path d="M12 3.5 13.8 10 20.5 12l-6.7 2L12 20.5 10.2 14 3.5 12l6.7-2L12 3.5Z"/>"#));
pub static GEAR: Icon = icon(
    "ff-gear",
    stroke!(
        r#"<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1Z"/>"#
    ),
);
pub static REFRESH: Icon =
    icon("ff-refresh", stroke!(r#"<path d="M20 12a8 8 0 0 1-14.3 4.9M4 12A8 8 0 0 1 18.3 7.1"/><path d="M18.5 3v4.3h-4.3M5.5 21v-4.3h4.3"/>"#));
pub static MINUS: Icon = icon("ff-minus", stroke!(r#"<path d="M6 12h12"/>"#));
pub static PLUS: Icon = icon("ff-plus", stroke!(r#"<path d="M12 6v12M6 12h12"/>"#));
pub static MOON: Icon = icon("ff-moon", stroke!(r#"<path d="M20 14.5A8 8 0 1 1 9.5 4a6.5 6.5 0 0 0 10.5 10.5Z"/>"#));
pub static CLOCK: Icon = icon("ff-clock", stroke!(r#"<circle cx="12" cy="12" r="8.5"/><path d="M12 7.5V12l3 2"/>"#));
pub static SWING_V: Icon = icon("ff-swing-v", stroke!(r#"<path d="M12 4v16M8.5 7.5 12 4l3.5 3.5M8.5 16.5 12 20l3.5-3.5"/>"#));
pub static SWING_H: Icon = icon("ff-swing-h", stroke!(r#"<path d="M4 12h16M7.5 8.5 4 12l3.5 3.5M16.5 8.5 20 12l-3.5 3.5"/>"#));
pub static BULB: Icon =
    icon("ff-bulb", stroke!(r#"<path d="M9 18h6M10 21h4"/><path d="M12 3a6 6 0 0 0-3.5 10.9c.6.5 1 1.2 1 2V16h5v-.1c0-.8.4-1.5 1-2A6 6 0 0 0 12 3Z"/>"#));
pub static WIND: Icon = icon("ff-wind", stroke!(r#"<path d="M3 8h11a3 3 0 1 0-3-3M3 12h16a3 3 0 1 1-3 3M3 16h8"/>"#));
pub static KEY: Icon = icon("ff-key", stroke!(r#"<circle cx="8" cy="15" r="4.5"/><path d="m11.2 11.8 8.3-8.3M16.5 6.5l2.5 2.5M14 9l2 2"/>"#));
pub static LOCK: Icon = icon("ff-lock", stroke!(r#"<rect x="5" y="10.5" width="14" height="10" rx="2"/><path d="M8 10.5V7.5a4 4 0 0 1 8 0v3"/>"#));

/// The icon for a ThinQ job mode.
pub fn for_mode(mode: &str) -> &'static Icon {
    match mode {
        "COOL" => &SNOW,
        "HEAT" => &FLAME,
        "AUTO" => &AUTO,
        "AIR_DRY" => &DROP,
        "FAN" => &FAN,
        "AIR_CLEAN" => &SPARKLE,
        "ENERGY_SAVING" => &LEAF,
        _ => &WIND,
    }
}

// ---- devices
pub static WASHER: Icon = icon(
    "ff-washer",
    stroke!(
        r#"<rect x="4.5" y="3" width="15" height="18" rx="2.5"/><path d="M4.5 7.5h15"/><circle cx="12" cy="14" r="4"/><path d="M10 14.6c.8.7 2.3.8 3.6-.3"/><path d="M7.5 5.25h.01M10 5.25h.01"/>"#
    ),
);
pub static DRYER: Icon = icon(
    "ff-dryer",
    stroke!(
        r#"<rect x="4.5" y="3" width="15" height="18" rx="2.5"/><path d="M4.5 7.5h15"/><circle cx="12" cy="14" r="4"/><path d="M11 12.5c1 .6 1 1.4 0 2s-1 1.4 0 2M13.5 12.5c1 .6 1 1.4 0 2"/><path d="M7.5 5.25h.01"/>"#
    ),
);
pub static FRIDGE: Icon =
    icon("ff-fridge", stroke!(r#"<rect x="5.5" y="2.5" width="13" height="19" rx="2.5"/><path d="M5.5 9.5h13"/><path d="M8.5 5.5v1.5M8.5 12.5v3"/>"#));
pub static OVEN: Icon = icon(
    "ff-oven",
    stroke!(
        r#"<rect x="3.5" y="3.5" width="17" height="17" rx="2.5"/><path d="M3.5 8h17"/><rect x="7" y="11" width="10" height="6.5" rx="1"/><path d="M7 5.75h.01M10 5.75h.01M13 5.75h.01"/>"#
    ),
);
pub static COOKTOP: Icon = icon(
    "ff-cooktop",
    stroke!(
        r#"<rect x="3" y="4" width="18" height="16" rx="2.5"/><circle cx="8.5" cy="9.5" r="2.5"/><circle cx="15.5" cy="9.5" r="2"/><circle cx="8.5" cy="15.5" r="2"/><circle cx="15.5" cy="15.5" r="2.5"/>"#
    ),
);
pub static MICROWAVE: Icon = icon(
    "ff-microwave",
    stroke!(
        r#"<rect x="2.5" y="5" width="19" height="14" rx="2.5"/><rect x="5" y="8" width="10" height="8" rx="1"/><path d="M18 9h.01M18 12h.01M18 15h.01"/>"#
    ),
);
pub static HOOD: Icon = icon(
    "ff-hood",
    stroke!(
        r#"<path d="M9.5 3h5v6.5l5.5 5H4l5.5-5Z"/><path d="M4 14.5v2h16v-2"/><path d="M8.5 19.5c.5.6.5 1.4 0 2M12 19.5c.5.6.5 1.4 0 2M15.5 19.5c.5.6.5 1.4 0 2"/>"#
    ),
);
pub static DISHWASHER: Icon = icon(
    "ff-dishwasher",
    stroke!(
        r#"<rect x="4.5" y="3" width="15" height="18" rx="2.5"/><path d="M4.5 7.5h15"/><path d="M8.5 11.5v6M12 11.5v6M15.5 11.5v6M7.5 17.5h9"/><path d="M15.5 5.25h1"/>"#
    ),
);
pub static ROBOT: Icon = icon("ff-robot", stroke!(r#"<circle cx="12" cy="12" r="8.5"/><circle cx="12" cy="12" r="3"/><path d="M6.5 7.5a7 7 0 0 1 11 0"/>"#));
pub static STICK: Icon = icon(
    "ff-stick",
    stroke!(
        r#"<path d="M15.5 3 10 16"/><path d="M6 16.5h7.5a1.5 1.5 0 0 1 1.5 1.5v1.5a1.5 1.5 0 0 1-1.5 1.5H6a1.5 1.5 0 0 1-1.5-1.5V18A1.5 1.5 0 0 1 6 16.5Z"/><rect x="13.5" y="5.5" width="4" height="6" rx="1.5" transform="rotate(22 15.5 8.5)"/>"#
    ),
);
pub static PURIFIER: Icon =
    icon("ff-purifier", stroke!(r#"<rect x="6" y="2.5" width="12" height="19" rx="4"/><circle cx="12" cy="9" r="2.5"/><path d="M9 15h6M9 18h6"/>"#));
pub static HUMIDIFIER: Icon =
    icon("ff-humidifier", stroke!(r#"<path d="M7 11.5h10l-1 9.5H8Z"/><path d="M12 2.5s3 3.3 3 5.3a3 3 0 0 1-6 0c0-2 3-5.3 3-5.3Z"/>"#));
pub static DEHUMIDIFIER: Icon = icon(
    "ff-dehumidifier",
    stroke!(
        r#"<rect x="5" y="3" width="14" height="18" rx="2.5"/><path d="M8 7h8"/><path d="M12 10.5s2.5 2.7 2.5 4.3a2.5 2.5 0 0 1-5 0c0-1.6 2.5-4.3 2.5-4.3Z"/>"#
    ),
);
pub static WATER: Icon =
    icon("ff-water", stroke!(r#"<path d="M6 3.5h12M8 3.5v4a4 4 0 0 0 8 0v-4"/><path d="M12 13s3.5 3.8 3.5 6a3.5 3.5 0 0 1-7 0c0-2.2 3.5-6 3.5-6Z"/>"#));
pub static HEATER: Icon = icon(
    "ff-heater",
    stroke!(
        r#"<rect x="6" y="2.5" width="12" height="19" rx="3"/><path d="M12 9.5c1.6 1.2 2 2.4 2 3.3a2 2 0 0 1-4 0c0-.6.2-1 .5-1.4.2.6.6.9 1 .9-.3-1 0-2 .5-2.8Z"/><path d="M9 18h6"/>"#
    ),
);
pub static WINE: Icon = icon("ff-wine", stroke!(r#"<path d="M8 3h8l-.5 5.5a3.5 3.5 0 0 1-7 0Z"/><path d="M12 12v7M8.5 21h7M8.3 6.5h7.4"/>"#));
pub static HANGER: Icon =
    icon("ff-hanger", stroke!(r#"<path d="M12 8.5V8a2 2 0 1 0-2-2"/><path d="M12 8.5 3 15.5a1 1 0 0 0 .6 1.8h16.8a1 1 0 0 0 .6-1.8Z"/>"#));
pub static MUG: Icon = icon(
    "ff-mug",
    stroke!(r#"<path d="M5 7h11v9a4 4 0 0 1-4 4H9a4 4 0 0 1-4-4Z"/><path d="M16 9.5h1.5a2.5 2.5 0 0 1 0 5H16"/><path d="M8.5 2.5v2M12 2.5v2"/>"#),
);
pub static HOUSE: Icon =
    icon("ff-house", stroke!(r#"<path d="M4 10.5 12 4l8 6.5V19a1.5 1.5 0 0 1-1.5 1.5h-13A1.5 1.5 0 0 1 4 19Z"/><path d="M9.5 20.5v-6h5v6"/>"#));

/// The icon for a ThinQ device type.
pub fn for_kind(code: &str) -> &'static Icon {
    match code {
        "DEVICE_AIR_CONDITIONER" => &SNOW,
        "DEVICE_AIR_PURIFIER" => &PURIFIER,
        "DEVICE_AIR_PURIFIER_FAN" | "DEVICE_CEILING_FAN" => &FAN,
        "DEVICE_DEHUMIDIFIER" => &DEHUMIDIFIER,
        "DEVICE_HUMIDIFIER" => &HUMIDIFIER,
        "DEVICE_VENTILATOR" => &WIND,
        "DEVICE_HOOD" => &HOOD,
        "DEVICE_SYSTEM_BOILER" | "DEVICE_WATER_HEATER" => &HEATER,
        "DEVICE_REFRIGERATOR" | "DEVICE_KIMCHI_REFRIGERATOR" => &FRIDGE,
        "DEVICE_WINE_CELLAR" => &WINE,
        "DEVICE_OVEN" => &OVEN,
        "DEVICE_COOKTOP" => &COOKTOP,
        "DEVICE_MICROWAVE_OVEN" => &MICROWAVE,
        "DEVICE_DISH_WASHER" => &DISHWASHER,
        "DEVICE_WATER_PURIFIER" => &WATER,
        "DEVICE_HOME_BREW" => &MUG,
        "DEVICE_PLANT_CULTIVATOR" => &LEAF,
        "DEVICE_WASHER" | "DEVICE_WASHTOWER" | "DEVICE_WASHTOWER_WASHER" | "DEVICE_WASHCOMBO_MAIN" | "DEVICE_WASHCOMBO_MINI" => &WASHER,
        "DEVICE_DRYER" | "DEVICE_WASHTOWER_DRYER" => &DRYER,
        "DEVICE_STYLER" => &HANGER,
        "DEVICE_ROBOT_CLEANER" => &ROBOT,
        "DEVICE_STICK_CLEANER" => &STICK,
        _ => &HOUSE,
    }
}
