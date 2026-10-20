pub struct StringUtils;

impl StringUtils {
    /// Non-empty env-style value: present and not blank after trim.
    pub fn non_empty(s: impl AsRef<str>) -> Option<String> {
        let t = s.as_ref().trim();
        if t.is_empty() {
            None
        } else {
            Some(t.to_string())
        }
    }

    /// Read env var; missing / blank → `None`.
    pub fn get_env(key: &str) -> Option<String> {
        std::env::var(key).ok().and_then(Self::non_empty)
    }

    pub fn trim_op(op: &mut Option<String>) {
        if let Some(v) = op.as_mut() {
            *v = v.trim().to_string();
            if v.is_empty() {
                *op = None;
            }
        }
    }

    pub fn is_valid_code(s: &str) -> bool {
        s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    }
}
