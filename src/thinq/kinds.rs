//! Every device type LG's ThinQ Connect API lists, with the names and icons the app shows.

/// Which shelf of the home a device sits on: sets the sidebar order and the tint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Family {
    /// Air conditioners, purifiers, fans, humidity.
    Air,
    /// Boilers and water heaters.
    Heat,
    /// Fridges, kimchi fridges, wine cellars.
    Cold,
    /// Ovens, cooktops, hoods, microwaves, dishwashers, water purifiers, brewers.
    Kitchen,
    /// Washers, dryers, stylers.
    Laundry,
    /// Robot and stick vacuums.
    Cleaning,
    Other,
}

impl Family {
    /// The sidebar heading.
    pub fn title(self) -> &'static str {
        match self {
            Family::Air => "Climate",
            Family::Heat => "Heating",
            Family::Cold | Family::Kitchen => "Kitchen",
            Family::Laundry => "Laundry",
            Family::Cleaning => "Cleaning",
            Family::Other => "Other",
        }
    }
}

/// What the app knows about a device type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Kind {
    pub code: &'static str,
    pub name: &'static str,
    /// The sidebar heading for several of them.
    pub plural: &'static str,
    pub family: Family,
    /// The SF Symbol the menu bar uses.
    pub symbol: &'static str,
    /// Remote starts heat or cut: a start needs a second press.
    pub careful: bool,
}

const fn k(code: &'static str, name: &'static str, plural: &'static str, family: Family, symbol: &'static str, careful: bool) -> Kind {
    Kind { code, name, plural, family, symbol, careful }
}

use Family::*;

pub const KINDS: &[Kind] = &[
    k("DEVICE_AIR_CONDITIONER", "Air conditioner", "Air conditioners", Air, "snowflake", false),
    k("DEVICE_AIR_PURIFIER", "Air purifier", "Air purifiers", Air, "air.purifier", false),
    k("DEVICE_AIR_PURIFIER_FAN", "Purifier fan", "Purifier fans", Air, "fan", false),
    k("DEVICE_CEILING_FAN", "Ceiling fan", "Ceiling fans", Air, "fan.ceiling", false),
    k("DEVICE_DEHUMIDIFIER", "Dehumidifier", "Dehumidifiers", Air, "dehumidifier", false),
    k("DEVICE_HUMIDIFIER", "Humidifier", "Humidifiers", Air, "humidifier", false),
    k("DEVICE_VENTILATOR", "Ventilator", "Ventilators", Air, "wind", false),
    k("DEVICE_SYSTEM_BOILER", "Boiler", "Boilers", Heat, "heater.vertical", false),
    k("DEVICE_WATER_HEATER", "Water heater", "Water heaters", Heat, "drop.degreesign", false),
    k("DEVICE_REFRIGERATOR", "Refrigerator", "Refrigerators", Cold, "refrigerator", false),
    k("DEVICE_KIMCHI_REFRIGERATOR", "Kimchi refrigerator", "Kimchi refrigerators", Cold, "refrigerator", false),
    k("DEVICE_WINE_CELLAR", "Wine cellar", "Wine cellars", Cold, "wineglass", false),
    k("DEVICE_OVEN", "Oven", "Ovens", Kitchen, "oven", true),
    k("DEVICE_COOKTOP", "Cooktop", "Cooktops", Kitchen, "stove", true),
    k("DEVICE_MICROWAVE_OVEN", "Microwave", "Microwaves", Kitchen, "microwave", true),
    k("DEVICE_HOOD", "Range hood", "Range hoods", Kitchen, "wind", false),
    k("DEVICE_DISH_WASHER", "Dishwasher", "Dishwashers", Kitchen, "dishwasher", false),
    k("DEVICE_WATER_PURIFIER", "Water purifier", "Water purifiers", Kitchen, "drop", false),
    k("DEVICE_HOME_BREW", "Home brew", "Home brew", Kitchen, "mug", false),
    k("DEVICE_PLANT_CULTIVATOR", "Plant cultivator", "Plant cultivators", Kitchen, "leaf", false),
    k("DEVICE_WASHER", "Washer", "Washers", Laundry, "washer", false),
    k("DEVICE_DRYER", "Dryer", "Dryers", Laundry, "dryer", false),
    k("DEVICE_WASHTOWER", "WashTower", "WashTowers", Laundry, "washer", false),
    k("DEVICE_WASHTOWER_WASHER", "WashTower washer", "WashTower washers", Laundry, "washer", false),
    k("DEVICE_WASHTOWER_DRYER", "WashTower dryer", "WashTower dryers", Laundry, "dryer", false),
    k("DEVICE_WASHCOMBO_MAIN", "Washer", "Washers", Laundry, "washer", false),
    k("DEVICE_WASHCOMBO_MINI", "Mini washer", "Mini washers", Laundry, "washer", false),
    k("DEVICE_STYLER", "Styler", "Stylers", Laundry, "tshirt", false),
    k("DEVICE_ROBOT_CLEANER", "Robot vacuum", "Robot vacuums", Cleaning, "circle.circle", false),
    k("DEVICE_STICK_CLEANER", "Stick vacuum", "Stick vacuums", Cleaning, "battery.75percent", false),
];

const UNKNOWN: Kind = k("", "Device", "Other devices", Other, "house", false);

/// The kind for a `deviceType`; unknown types still get a sensible name.
pub fn kind(code: &str) -> Kind {
    KINDS.iter().copied().find(|k| k.code == code).unwrap_or(Kind { code: "", ..UNKNOWN })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_type_in_lgs_sdk_has_a_kind() {
        // The device types LG's SDK (thinqconnect 1.0) defines.
        for code in [
            "DEVICE_AIR_CONDITIONER",
            "DEVICE_AIR_PURIFIER",
            "DEVICE_AIR_PURIFIER_FAN",
            "DEVICE_CEILING_FAN",
            "DEVICE_COOKTOP",
            "DEVICE_DEHUMIDIFIER",
            "DEVICE_DISH_WASHER",
            "DEVICE_DRYER",
            "DEVICE_HOME_BREW",
            "DEVICE_HOOD",
            "DEVICE_HUMIDIFIER",
            "DEVICE_KIMCHI_REFRIGERATOR",
            "DEVICE_MICROWAVE_OVEN",
            "DEVICE_OVEN",
            "DEVICE_PLANT_CULTIVATOR",
            "DEVICE_REFRIGERATOR",
            "DEVICE_ROBOT_CLEANER",
            "DEVICE_STICK_CLEANER",
            "DEVICE_STYLER",
            "DEVICE_SYSTEM_BOILER",
            "DEVICE_VENTILATOR",
            "DEVICE_WASHCOMBO_MAIN",
            "DEVICE_WASHCOMBO_MINI",
            "DEVICE_WASHER",
            "DEVICE_WASHTOWER",
            "DEVICE_WASHTOWER_DRYER",
            "DEVICE_WASHTOWER_WASHER",
            "DEVICE_WATER_HEATER",
            "DEVICE_WATER_PURIFIER",
            "DEVICE_WINE_CELLAR",
        ] {
            assert_eq!(kind(code).code, code, "{code}");
        }
        assert_eq!(kind("DEVICE_TOASTER").name, "Device");
    }
}
