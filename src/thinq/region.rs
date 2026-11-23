//! Which LG ThinQ Connect server answers for a country.
//!
//! The table follows LG's own SDK (`thinqconnect/country.py`, Apache-2.0). The host is
//! built from this enum only, so nothing a user types ever reaches the URL's authority.

/// One of the three ThinQ Connect API regions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Region {
    /// Korea, Asia and Oceania.
    Kic,
    /// The Americas.
    Aic,
    /// Europe, the Middle East and Africa.
    Eic,
}

impl Region {
    /// The API host for this region.
    pub fn host(self) -> &'static str {
        match self {
            Region::Kic => "api-kic.lgthinq.com",
            Region::Aic => "api-aic.lgthinq.com",
            Region::Eic => "api-eic.lgthinq.com",
        }
    }
}

const KIC: &[&str] = &["AU", "BD", "CN", "HK", "ID", "IN", "JP", "KH", "KR", "LA", "LK", "MM", "MY", "NP", "NZ", "PH", "SG", "TH", "TW", "VN"];
const AIC: &[&str] = &[
    "AG", "AR", "AW", "BB", "BO", "BR", "BS", "BZ", "CA", "CL", "CO", "CR", "CU", "DM", "DO", "EC", "GD", "GT", "GY", "HN", "HT", "JM", "KN", "LC", "MX", "NI",
    "PA", "PE", "PR", "PY", "SR", "SV", "TT", "US", "UY", "VC", "VE",
];
const EIC: &[&str] = &[
    "AE", "AF", "AL", "AM", "AO", "AT", "AZ", "BA", "BE", "BF", "BG", "BH", "BJ", "BY", "CD", "CF", "CG", "CH", "CI", "CM", "CV", "CY", "CZ", "DE", "DJ", "DK",
    "DZ", "EE", "EG", "ES", "ET", "FI", "FR", "GA", "GB", "GE", "GH", "GM", "GN", "GQ", "GR", "HR", "HU", "IE", "IL", "IQ", "IR", "IS", "IT", "JO", "KE", "KG",
    "KW", "KZ", "LB", "LR", "LT", "LU", "LV", "LY", "MA", "MD", "ME", "MK", "ML", "MR", "MT", "MU", "MW", "NE", "NG", "NL", "NO", "OM", "PK", "PL", "PS", "PT",
    "QA", "RO", "RS", "RU", "RW", "SA", "SD", "SE", "SI", "SK", "SL", "SN", "SO", "ST", "SY", "TD", "TG", "TN", "TR", "TZ", "UA", "UG", "UZ", "XK", "YE", "ZA",
    "ZM",
];

/// A validated ISO 3166-1 alpha-2 country code that ThinQ Connect serves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Country {
    code: [u8; 2],
    region: Region,
}

impl Country {
    /// Parses a code such as `"us"` or `"PT"`. Only countries in LG's table are accepted.
    pub fn parse(raw: &str) -> Option<Country> {
        let raw = raw.trim();
        if raw.len() != 2 || !raw.bytes().all(|b| b.is_ascii_alphabetic()) {
            return None;
        }
        let up = raw.to_ascii_uppercase();
        let region = if KIC.contains(&up.as_str()) {
            Region::Kic
        } else if AIC.contains(&up.as_str()) {
            Region::Aic
        } else if EIC.contains(&up.as_str()) {
            Region::Eic
        } else {
            return None;
        };
        let b = up.as_bytes();
        Some(Country { code: [b[0], b[1]], region })
    }

    pub fn code(&self) -> &str {
        // Both bytes are ASCII letters (checked in `parse`).
        std::str::from_utf8(&self.code).unwrap_or("US")
    }

    pub fn region(&self) -> Region {
        self.region
    }

    /// Every supported country code, sorted.
    pub fn all() -> Vec<&'static str> {
        let mut v: Vec<&'static str> = KIC.iter().chain(AIC).chain(EIC).copied().collect();
        v.sort_unstable();
        v
    }

    /// The country's English name, for the picker.
    pub fn name(code: &str) -> &'static str {
        match code {
            "AE" => "United Arab Emirates",
            "AF" => "Afghanistan",
            "AG" => "Antigua and Barbuda",
            "AL" => "Albania",
            "AM" => "Armenia",
            "AO" => "Angola",
            "AR" => "Argentina",
            "AT" => "Austria",
            "AU" => "Australia",
            "AW" => "Aruba",
            "AZ" => "Azerbaijan",
            "BA" => "Bosnia and Herzegovina",
            "BB" => "Barbados",
            "BD" => "Bangladesh",
            "BE" => "Belgium",
            "BF" => "Burkina Faso",
            "BG" => "Bulgaria",
            "BH" => "Bahrain",
            "BJ" => "Benin",
            "BO" => "Bolivia",
            "BR" => "Brazil",
            "BS" => "Bahamas",
            "BY" => "Belarus",
            "BZ" => "Belize",
            "CA" => "Canada",
            "CD" => "Congo (DRC)",
            "CF" => "Central African Republic",
            "CG" => "Congo",
            "CH" => "Switzerland",
            "CI" => "Côte d'Ivoire",
            "CL" => "Chile",
            "CM" => "Cameroon",
            "CN" => "China",
            "CO" => "Colombia",
            "CR" => "Costa Rica",
            "CU" => "Cuba",
            "CV" => "Cape Verde",
            "CY" => "Cyprus",
            "CZ" => "Czechia",
            "DE" => "Germany",
            "DJ" => "Djibouti",
            "DK" => "Denmark",
            "DM" => "Dominica",
            "DO" => "Dominican Republic",
            "DZ" => "Algeria",
            "EC" => "Ecuador",
            "EE" => "Estonia",
            "EG" => "Egypt",
            "ES" => "Spain",
            "ET" => "Ethiopia",
            "FI" => "Finland",
            "FR" => "France",
            "GA" => "Gabon",
            "GB" => "United Kingdom",
            "GD" => "Grenada",
            "GE" => "Georgia",
            "GH" => "Ghana",
            "GM" => "Gambia",
            "GN" => "Guinea",
            "GQ" => "Equatorial Guinea",
            "GR" => "Greece",
            "GT" => "Guatemala",
            "GY" => "Guyana",
            "HK" => "Hong Kong",
            "HN" => "Honduras",
            "HR" => "Croatia",
            "HT" => "Haiti",
            "HU" => "Hungary",
            "ID" => "Indonesia",
            "IE" => "Ireland",
            "IL" => "Israel",
            "IN" => "India",
            "IQ" => "Iraq",
            "IR" => "Iran",
            "IS" => "Iceland",
            "IT" => "Italy",
            "JM" => "Jamaica",
            "JO" => "Jordan",
            "JP" => "Japan",
            "KE" => "Kenya",
            "KG" => "Kyrgyzstan",
            "KH" => "Cambodia",
            "KN" => "Saint Kitts and Nevis",
            "KR" => "South Korea",
            "KW" => "Kuwait",
            "KZ" => "Kazakhstan",
            "LA" => "Laos",
            "LB" => "Lebanon",
            "LC" => "Saint Lucia",
            "LK" => "Sri Lanka",
            "LR" => "Liberia",
            "LT" => "Lithuania",
            "LU" => "Luxembourg",
            "LV" => "Latvia",
            "LY" => "Libya",
            "MA" => "Morocco",
            "MD" => "Moldova",
            "ME" => "Montenegro",
            "MK" => "North Macedonia",
            "ML" => "Mali",
            "MM" => "Myanmar",
            "MR" => "Mauritania",
            "MT" => "Malta",
            "MU" => "Mauritius",
            "MW" => "Malawi",
            "MX" => "Mexico",
            "MY" => "Malaysia",
            "NE" => "Niger",
            "NG" => "Nigeria",
            "NI" => "Nicaragua",
            "NL" => "Netherlands",
            "NO" => "Norway",
            "NP" => "Nepal",
            "NZ" => "New Zealand",
            "OM" => "Oman",
            "PA" => "Panama",
            "PE" => "Peru",
            "PH" => "Philippines",
            "PK" => "Pakistan",
            "PL" => "Poland",
            "PR" => "Puerto Rico",
            "PS" => "Palestine",
            "PT" => "Portugal",
            "PY" => "Paraguay",
            "QA" => "Qatar",
            "RO" => "Romania",
            "RS" => "Serbia",
            "RU" => "Russia",
            "RW" => "Rwanda",
            "SA" => "Saudi Arabia",
            "SD" => "Sudan",
            "SE" => "Sweden",
            "SG" => "Singapore",
            "SI" => "Slovenia",
            "SK" => "Slovakia",
            "SL" => "Sierra Leone",
            "SN" => "Senegal",
            "SO" => "Somalia",
            "SR" => "Suriname",
            "ST" => "São Tomé and Príncipe",
            "SV" => "El Salvador",
            "SY" => "Syria",
            "TD" => "Chad",
            "TG" => "Togo",
            "TH" => "Thailand",
            "TN" => "Tunisia",
            "TR" => "Türkiye",
            "TT" => "Trinidad and Tobago",
            "TW" => "Taiwan",
            "TZ" => "Tanzania",
            "UA" => "Ukraine",
            "UG" => "Uganda",
            "US" => "United States",
            "UY" => "Uruguay",
            "UZ" => "Uzbekistan",
            "VC" => "Saint Vincent and the Grenadines",
            "VE" => "Venezuela",
            "VN" => "Vietnam",
            "XK" => "Kosovo",
            "YE" => "Yemen",
            "ZA" => "South Africa",
            "ZM" => "Zambia",
            _ => "",
        }
    }

    /// Supported countries whose name or code matches `query` (case-insensitive), by name.
    pub fn search(query: &str) -> Vec<&'static str> {
        let q = query.trim().to_lowercase();
        let mut v: Vec<&'static str> =
            Country::all().into_iter().filter(|c| q.is_empty() || c.eq_ignore_ascii_case(&q) || Country::name(c).to_lowercase().contains(&q)).collect();
        // An exact code first ("pt" → Portugal), then names that start with the query, then the rest.
        v.sort_by_key(|c| (!c.eq_ignore_ascii_case(&q), !Country::name(c).to_lowercase().starts_with(&q), Country::name(c)));
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn countries_map_to_their_region() {
        assert_eq!(Country::parse("US").unwrap().region(), Region::Aic);
        assert_eq!(Country::parse("pt").unwrap().region(), Region::Eic);
        assert_eq!(Country::parse(" kr ").unwrap().region(), Region::Kic);
        assert_eq!(Country::parse("br").unwrap().code(), "BR");
    }

    #[test]
    fn every_country_has_a_name_and_search_finds_it() {
        for c in Country::all() {
            assert!(!Country::name(c).is_empty(), "{c}");
        }
        assert_eq!(Country::search("portu")[0], "PT");
        assert_eq!(Country::search("gb")[0], "GB");
        assert_eq!(Country::search("united")[0..2], ["AE", "GB"]);
        assert!(Country::search("zzzz").is_empty());
    }

    #[test]
    fn junk_is_rejected() {
        for bad in ["", "U", "USA", "1A", "ZZ", "u$", "../", "é1"] {
            assert!(Country::parse(bad).is_none(), "{bad:?}");
        }
    }

    #[test]
    fn hosts_are_lg_hosts() {
        for r in [Region::Kic, Region::Aic, Region::Eic] {
            assert!(r.host().ends_with(".lgthinq.com"));
        }
    }
}
