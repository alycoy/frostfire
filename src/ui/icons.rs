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
