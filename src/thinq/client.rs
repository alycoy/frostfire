//! A small, blocking client for LG's official ThinQ Connect REST API.
//!
//! Requests and headers follow LG's SDK (`thinqconnect/thinq_api.py`, Apache-2.0). Run it off
//! the UI thread (see `crate::worker`).
//!
//! Safety rails:
//! - HTTPS only, through the platform's TLS stack and trust store, to one of LG's three
//!   hosts picked by [`Region`]. Nothing the user types becomes part of the URL's authority.
//! - Device ids are checked against a strict alphabet before they go into a path.
//! - Every request has a timeout, and response bodies are capped.
//! - The token is a [`Secret`]: zeroed on drop and never printed by `Debug`.

use std::time::Duration;

use serde_json::Value;

use super::Country;
use super::secret::Secret;

/// LG's public API key for ThinQ Connect clients (from LG's SDK; it identifies the client
/// library, not the user).
const API_KEY: &str = "v6GFvkweNo7DK7yD3ylIZ9w52aKBU0eJ7wLXkSR3";
/// Largest response body accepted, in bytes. Profiles are the biggest answers, a few KiB.
const MAX_BODY: u64 = 2 * 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(15);

/// What can go wrong talking to ThinQ.
#[derive(Debug, Clone, PartialEq)]
pub enum ApiError {
    /// ThinQ answered with an error code (see [`ApiError::friendly`]).
    Api { code: String, message: String },
    /// The request didn't complete: offline, DNS, TLS, timeout.
    Network(String),
    /// An answer that wasn't the JSON ThinQ promises.
    Protocol(String),
    /// A request refused before sending (bad device id, bad token characters).
    Invalid(&'static str),
}

impl ApiError {
    /// One sentence a person can act on.
    pub fn friendly(&self) -> String {
        match self {
            ApiError::Api { code, message } => match code.as_str() {
                "1103" | "1218" | "1302" => "LG didn't accept the token. Create a new one and paste it again.".into(),
                "1306" => "Too many requests for today. ThinQ will let us back in later.".into(),
                "1307" => "ThinQ Connect isn't available for that country.".into(),
                "1222" => "The air conditioner is offline. Check its Wi-Fi.".into(),
                "2301" | "2304" => "Turn the air conditioner on first.".into(),
                "2305" => "That setting isn't available in this mode.".into(),
                "2209" | "2210" | "2212" => "The air conditioner is slow to answer. Try again in a moment.".into(),
                "1205" | "1211" | "1212" | "1213" => "That device isn't on this LG account anymore.".into(),
                "1308" => "This account can't control that device.".into(),
                _ if message.is_empty() => format!("ThinQ error {code}."),
                _ => format!("ThinQ error {code}: {}", truncate(message, 120)),
            },
            ApiError::Network(m) => format!("Couldn't reach LG ThinQ ({}).", truncate(m, 80)),
            ApiError::Protocol(m) => format!("Unexpected answer from ThinQ ({}).", truncate(m, 80)),
            ApiError::Invalid(m) => (*m).to_string(),
        }
    }

    /// The token itself was rejected, so signing in again is the fix.
    pub fn is_auth(&self) -> bool {
        matches!(self, ApiError::Api { code, .. } if matches!(code.as_str(), "1103" | "1218" | "1302" | "1303"))
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n { s.to_string() } else { s.chars().take(n).collect::<String>() + "…" }
}

/// Authenticated access to one ThinQ account.
pub struct Client {
    token: Secret,
    country: Country,
    client_id: String,
    agent: ureq::Agent,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client").field("country", &self.country.code()).field("token", &self.token).finish()
    }
}

impl Client {
    /// `client_id` is this install's random id (see `crate::config`).
    pub fn new(token: Secret, country: Country, client_id: String) -> Result<Client, ApiError> {
        if !valid_token(token.expose()) {
            return Err(ApiError::Invalid("That doesn't look like a ThinQ token. Paste it again without spaces."));
        }
        if !valid_client_id(&client_id) {
            return Err(ApiError::Invalid("The client id is malformed."));
        }
        let tls = ureq::tls::TlsConfig::builder().provider(ureq::tls::TlsProvider::NativeTls).build();
        let config = ureq::Agent::config_builder()
            .tls_config(tls)
            .https_only(true)
            .timeout_global(Some(TIMEOUT))
            .http_status_as_error(false)
            .max_redirects(0)
            .user_agent(concat!("Frostfire/", env!("CARGO_PKG_VERSION")))
            .build();
        Ok(Client { token, country, client_id, agent: config.into() })
    }

    /// `GET /devices`: every device on the account.
    pub fn devices(&self) -> Result<Value, ApiError> {
        self.request("GET", "devices", None)
    }

    /// `GET /devices/{id}/profile`: what the device can do.
    pub fn profile(&self, device: &str) -> Result<Value, ApiError> {
        check_device_id(device)?;
        self.request("GET", &format!("devices/{device}/profile"), None)
    }

    /// `GET /devices/{id}/state`: what it's doing now.
    pub fn state(&self, device: &str) -> Result<Value, ApiError> {
        check_device_id(device)?;
        self.request("GET", &format!("devices/{device}/state"), None)
    }

    /// `POST /devices/{id}/control` with one command payload (see `model::Command::payload`).
    pub fn control(&self, device: &str, payload: &Value) -> Result<Value, ApiError> {
        check_device_id(device)?;
        self.request("POST", &format!("devices/{device}/control"), Some(payload))
    }

    /// `GET /devices/energy/{id}/profile`: which energy properties can be queried.
    pub fn energy_profile(&self, device: &str) -> Result<Value, ApiError> {
        check_device_id(device)?;
        self.request("GET", &format!("devices/energy/{device}/profile"), None)
    }

    /// `GET /devices/energy/{id}/usage`, one value per day from `start` to `end` (`YYYYMMDD`).
    pub fn energy_daily(&self, device: &str, property: &str, start: &str, end: &str) -> Result<Value, ApiError> {
        check_device_id(device)?;
        let ok_prop = !property.is_empty() && property.len() <= 64 && property.bytes().all(|b| b.is_ascii_alphanumeric());
        let ok_date = |d: &str| d.len() == 8 && d.bytes().all(|b| b.is_ascii_digit());
        if !ok_prop || !ok_date(start) || !ok_date(end) {
            return Err(ApiError::Invalid("Bad energy query."));
        }
        let path = format!("devices/energy/{device}/usage?property={property}&period=DAILY&startDate={start}&endDate={end}");
        self.request("GET", &path, None)
    }

    fn request(&self, method: &str, path: &str, body: Option<&Value>) -> Result<Value, ApiError> {
        let url = format!("https://{}/{}", self.country.region().host(), path);
        let auth = Secret::new(format!("Bearer {}", self.token.expose()));
        let message_id = message_id().ok_or(ApiError::Network("no system randomness".into()))?;
        let headers: [(&str, &str); 7] = [
            ("Authorization", auth.expose()),
            ("x-country", self.country.code()),
            ("x-message-id", &message_id),
            ("x-client-id", &self.client_id),
            ("x-api-key", API_KEY),
            ("x-service-phase", "OP"),
            ("Accept", "application/json"),
        ];
        let result = match (method, body) {
            ("POST", Some(body)) => {
                let mut req = self.agent.post(&url);
                for (k, v) in headers {
                    req = req.header(k, v);
                }
                let bytes = serde_json::to_vec(body).map_err(|e| ApiError::Protocol(e.to_string()))?;
                req.header("x-conditional-control", "true").header("Content-Type", "application/json").send(&bytes[..])
            }
            _ => {
                let mut req = self.agent.get(&url);
                for (k, v) in headers {
                    req = req.header(k, v);
                }
                req.call()
            }
        };
        let mut resp = result.map_err(|e| ApiError::Network(redact(&e.to_string(), self.token.expose())))?;
        let status = resp.status().as_u16();
        let text =
            resp.body_mut().with_config().limit(MAX_BODY).read_to_string().map_err(|e| ApiError::Network(redact(&e.to_string(), self.token.expose())))?;
        parse_envelope(status, &text)
    }
}

/// Unwraps ThinQ's `{"response": ..}` / `{"error": {"code", "message"}}` envelope.
pub fn parse_envelope(status: u16, text: &str) -> Result<Value, ApiError> {
    let json: Value = serde_json::from_str(text).map_err(|_| ApiError::Protocol(format!("HTTP {status}, not JSON")))?;
    if (200..300).contains(&status) {
        return Ok(json.get("response").cloned().unwrap_or(Value::Null));
    }
    let err = json.get("error");
    let code = err.and_then(|e| e.get("code")).map(|c| match c {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    });
    let message = err.and_then(|e| e.get("message")).and_then(Value::as_str).unwrap_or("").to_string();
    match code {
        Some(code) => Err(ApiError::Api { code, message }),
        None => Err(ApiError::Protocol(format!("HTTP {status}"))),
    }
}

/// ThinQ device ids are opaque, but always URL-safe. Anything else is refused so it can't
/// steer the path (`../`, `?`, `#`, `%`).
pub fn check_device_id(id: &str) -> Result<(), ApiError> {
    let ok = !id.is_empty() && id.len() <= 128 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    if ok { Ok(()) } else { Err(ApiError::Invalid("Refusing a malformed device id.")) }
}

/// A personal access token is printable ASCII with no whitespace.
pub fn valid_token(t: &str) -> bool {
    (16..=4096).contains(&t.len()) && t.bytes().all(|b| b.is_ascii_graphic())
}

fn valid_client_id(id: &str) -> bool {
    (8..=64).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

/// A fresh `x-message-id`: 16 random bytes, URL-safe base64 without padding (22 characters),
/// as LG's SDK makes them.
pub fn message_id() -> Option<String> {
    let mut b = [0u8; 16];
    getrandom::fill(&mut b).ok()?;
    Some(base64url(&b))
}

/// A new install's client id, `thinq-open-<uuid v4>` as LG's SDK names them.
pub fn new_client_id() -> Option<String> {
    let mut b = [0u8; 16];
    getrandom::fill(&mut b).ok()?;
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
    Some(format!("thinq-open-{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32]))
}

fn base64url(bytes: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (chunk[0] as u32) << 16 | (*chunk.get(1).unwrap_or(&0) as u32) << 8 | *chunk.get(2).unwrap_or(&0) as u32;
        let take = chunk.len() + 1;
        for i in 0..take {
            out.push(A[((n >> (18 - 6 * i)) & 63) as usize] as char);
        }
    }
    out
}

/// Strips the token from an error string, in case a library ever echoes a header.
fn redact(s: &str, token: &str) -> String {
    if token.len() >= 8 { s.replace(token, "[redacted]") } else { s.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_ids_look_like_lgs() {
        let id = message_id().unwrap();
        assert_eq!(id.len(), 22);
        assert!(id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'));
        assert_ne!(id, message_id().unwrap());
    }

    #[test]
    fn base64url_matches_reference() {
        assert_eq!(base64url(b"hello"), "aGVsbG8");
        assert_eq!(base64url(&[0xfb, 0xff]), "-_8");
        assert_eq!(base64url(&[0u8; 16]).len(), 22);
    }

    #[test]
    fn client_ids_are_uuid_v4() {
        let id = new_client_id().unwrap();
        assert!(id.starts_with("thinq-open-"));
        assert!(valid_client_id(&id));
        let uuid = &id["thinq-open-".len()..];
        assert_eq!(uuid.len(), 36);
        assert_eq!(&uuid[14..15], "4");
    }

    #[test]
    fn device_ids_cannot_escape_the_path() {
        assert!(check_device_id("a1b2-C3_d4").is_ok());
        for bad in ["", "../profile", "a/b", "a?b", "a#b", "a%2F", "a b", "é"] {
            assert!(check_device_id(bad).is_err(), "{bad:?}");
        }
        assert!(check_device_id(&"x".repeat(129)).is_err());
    }

    #[test]
    fn tokens_must_be_printable() {
        assert!(valid_token("thinqpat_0123456789abcdef"));
        assert!(!valid_token("short"));
        assert!(!valid_token("thinqpat_0123456789 abcdef"));
        assert!(!valid_token("thinqpat_0123456789abcdef\r\nX-Evil: 1"));
    }

    #[test]
    fn envelopes_unwrap() {
        let ok = parse_envelope(200, r#"{"messageId":"x","response":{"a":1}}"#).unwrap();
        assert_eq!(ok["a"], 1);
        let err = parse_envelope(401, r#"{"error":{"code":"1103","message":"Invalid token"}}"#).unwrap_err();
        assert!(err.is_auth());
        assert!(err.friendly().contains("token"));
        let num = parse_envelope(400, r#"{"error":{"code":2305,"message":""}}"#).unwrap_err();
        assert_eq!(num, ApiError::Api { code: "2305".into(), message: "".into() });
        assert!(matches!(parse_envelope(502, "<html>"), Err(ApiError::Protocol(_))));
    }

    #[test]
    fn debug_never_prints_the_token() {
        let c = Client::new(Secret::new("thinqpat_supersecretvalue123".into()), Country::parse("US").unwrap(), new_client_id().unwrap()).unwrap();
        let s = format!("{c:?}");
        assert!(!s.contains("supersecret"), "{s}");
    }

    #[test]
    fn redact_hides_tokens() {
        assert_eq!(redact("bad header Bearer abcdefgh123", "abcdefgh123"), "bad header Bearer [redacted]");
    }
}
