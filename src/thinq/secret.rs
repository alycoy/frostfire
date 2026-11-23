//! A string that is zeroed when dropped and never shown by `Debug`.

use zeroize::Zeroize;

pub struct Secret(String);

impl Secret {
    pub fn new(s: String) -> Secret {
        Secret(s)
    }

    /// The value, for the one place that needs it (a request header, the Keychain).
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl Clone for Secret {
    fn clone(&self) -> Secret {
        Secret(self.0.clone())
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret([redacted])")
    }
}

#[cfg(test)]
mod tests {
    use super::Secret;

    #[test]
    fn debug_is_redacted() {
        let s = Secret::new("hunter2hunter2".into());
        assert_eq!(format!("{s:?}"), "Secret([redacted])");
        assert_eq!(s.expose(), "hunter2hunter2");
    }
}
