//! Colour: the window is neutral (OpenCode's tokens, as Hex uses them), and the one tint in it
//! follows what the unit is doing: ice blue while cooling, ember orange while heating.

use rune::Color;

/// `0xRRGGBB` (sRGB) as a Rune colour (linear).
pub fn c(hex: u32) -> Color {
    let ch = |s: u32| rune::color::srgb_decode(((hex >> s) & 0xff) as f32 / 255.0);
    Color { r: ch(16), g: ch(8), b: ch(0), a: 1.0 }
}

/// Which small effect the dial wears.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weather {
    Frost,
    Fire,
    Aurora,
    Mist,
    Breeze,
    Still,
}

#[derive(Clone, Copy, Debug)]
pub struct Mood {
    pub weather: Weather,
    /// The tint: the dial's arc, the mode's icon, the energy bars.
    pub accent: Color,
    /// The arc's far end.
    pub accent_hi: Color,
}

fn tint(weather: Weather, accent: u32, hi: u32) -> Mood {
    Mood { weather, accent: c(accent), accent_hi: c(hi) }
}

/// Reads on both light and dark surfaces.
pub fn mood(mode: Option<&str>, on: bool) -> Mood {
    if !on {
        return tint(Weather::Still, 0x8e8e93, 0xaeaeb2);
    }
    match mode.unwrap_or("COOL") {
        "HEAT" => tint(Weather::Fire, 0xff8a1f, 0xff4d2e),
        "AUTO" => tint(Weather::Aurora, 0x7a6cf5, 0x32c6c0),
        "AIR_DRY" => tint(Weather::Mist, 0x2fb4c4, 0x5ad1d8),
        "FAN" | "AIR_CLEAN" | "ENERGY_SAVING" => tint(Weather::Breeze, 0x30b46c, 0x5fcf8a),
        _ => tint(Weather::Frost, 0x5ab8f5, 0x1f7cf0),
    }
}

/// The tint for any other device: by what it does, grey while off.
pub fn kind_mood(kind: &crate::thinq::kinds::Kind, on: bool) -> Mood {
    use crate::thinq::kinds::Family;
    if !on {
        return tint(Weather::Still, 0x8e8e93, 0xaeaeb2);
    }
    match (kind.family, kind.code) {
        (Family::Air, "DEVICE_DEHUMIDIFIER" | "DEVICE_HUMIDIFIER") => tint(Weather::Mist, 0x2fb4c4, 0x5ad1d8),
        (Family::Air, _) => tint(Weather::Breeze, 0x30b46c, 0x5fcf8a),
        (Family::Heat, _) => tint(Weather::Fire, 0xff8a1f, 0xff4d2e),
        (Family::Cold, _) => tint(Weather::Frost, 0x5ab8f5, 0x1f7cf0),
        (Family::Kitchen, _) => tint(Weather::Still, 0xf2a43a, 0xff7a45),
        (Family::Laundry, _) => tint(Weather::Aurora, 0x7a6cf5, 0x32c6c0),
        (Family::Cleaning, _) => tint(Weather::Aurora, 0x5b8def, 0x7a6cf5),
        (Family::Other, _) => tint(Weather::Frost, 0x5ab8f5, 0x1f7cf0),
    }
}
