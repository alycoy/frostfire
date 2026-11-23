//! What Frostfire remembers between launches.
//!
//! - The ThinQ token goes in the macOS login Keychain and nowhere else.
//! - Everything else (country, chosen device, preferences, a random client id) is a small
//!   JSON file in `~/Library/Application Support/Frostfire/`, readable only by you (0600).
//!
//! Both are touched from the worker thread only, never during a frame.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::thinq::Secret;

const SERVICE: &str = "io.github.alycoy.frostfire";
const ACCOUNT: &str = "thinq-personal-access-token";

/// How much the background moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Effects {
    /// Living fills, frost sparkles and embers.
    #[default]
    Full,
    /// The fills only, slowed down.
    Calm,
    /// Still gradients. Draws nothing while idle.
    Off,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub country: Option<String>,
    pub client_id: Option<String>,
    pub device: Option<String>,
    /// "C" or "F".
    pub unit: String,
    pub refresh_secs: u64,
    pub effects: Effects,
    pub menu_bar: bool,
    pub dock_icon: bool,
}

impl Default for Config {
    fn default() -> Config {
        Config { country: None, client_id: None, device: None, unit: "C".into(), refresh_secs: 60, effects: Effects::Full, menu_bar: true, dock_icon: true }
    }
}

impl Config {
    /// Keeps values in their allowed sets, whatever the file said.
    pub fn sanitized(mut self) -> Config {
        if self.unit != "C" && self.unit != "F" {
            self.unit = "C".into();
        }
        self.refresh_secs = self.refresh_secs.clamp(30, 900);
        if self.country.as_deref().and_then(crate::thinq::Country::parse).is_none() {
            self.country = None;
        }
        if self.device.as_deref().is_some_and(|d| crate::thinq::client::check_device_id(d).is_err()) {
            self.device = None;
        }
        if let Some(id) = &self.client_id
            && !(8..=64).contains(&id.len())
        {
            self.client_id = None;
        }
        self
    }
}

fn dir() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join("Frostfire"))
}

pub fn load_config() -> Config {
    let Some(path) = dir().map(|d| d.join("config.json")) else { return Config::default() };
    match std::fs::read(&path) {
        Ok(bytes) if bytes.len() < 64 * 1024 => serde_json::from_slice::<Config>(&bytes).unwrap_or_default().sanitized(),
        _ => Config::default(),
    }
}

/// Writes the config atomically (temp file, then rename), readable only by the owner.
pub fn save_config(c: &Config) -> std::io::Result<()> {
    let dir = dir().ok_or_else(|| std::io::Error::other("no config directory"))?;
    std::fs::create_dir_all(&dir)?;
    let tmp = dir.join("config.json.tmp");
    let bytes = serde_json::to_vec_pretty(c).map_err(std::io::Error::other)?;
    write_private(&tmp, &bytes)?;
    std::fs::rename(&tmp, dir.join("config.json"))
}

#[cfg(unix)]
fn write_private(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut f = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(path)?;
    f.write_all(bytes)?;
    f.sync_all()
}

#[cfg(not(unix))]
fn write_private(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    std::fs::write(path, bytes)
}

/// The saved token, if any.
#[cfg(target_os = "macos")]
pub fn load_token() -> Option<Secret> {
    let bytes = security_framework::passwords::get_generic_password(SERVICE, ACCOUNT).ok()?;
    let s = String::from_utf8(bytes).ok()?;
    Some(Secret::new(s))
}

#[cfg(target_os = "macos")]
pub fn save_token(token: &Secret) -> Result<(), String> {
    security_framework::passwords::set_generic_password(SERVICE, ACCOUNT, token.expose().as_bytes()).map_err(|e| format!("Keychain: {e}"))
}

#[cfg(target_os = "macos")]
pub fn delete_token() {
    let _ = security_framework::passwords::delete_generic_password(SERVICE, ACCOUNT);
}

// Elsewhere there's no Keychain wired up yet, so the token lives for the session only.
#[cfg(not(target_os = "macos"))]
pub fn load_token() -> Option<Secret> {
    None
}

#[cfg(not(target_os = "macos"))]
pub fn save_token(_: &Secret) -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn delete_token() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_rejects_bad_values() {
        let c = Config {
            country: Some("ZZ".into()),
            device: Some("../x".into()),
            unit: "K".into(),
            refresh_secs: 1,
            client_id: Some("x".into()),
            ..Config::default()
        }
        .sanitized();
        assert_eq!(c.country, None);
        assert_eq!(c.device, None);
        assert_eq!(c.unit, "C");
        assert_eq!(c.refresh_secs, 30);
        assert_eq!(c.client_id, None);
    }

    #[test]
    fn unknown_fields_and_missing_fields_are_fine() {
        let c: Config = serde_json::from_str(r#"{"unit":"F","extra":1}"#).unwrap();
        assert_eq!(c.unit, "F");
        assert!(c.menu_bar);
    }
}
