use serde_json::Value;

use super::string::non_empty;

/// Read a non-empty trimmed string from a JSON object field.
pub fn get_string(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(|v| v.as_str()).and_then(non_empty)
}
