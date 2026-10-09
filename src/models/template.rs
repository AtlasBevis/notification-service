use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateData {
    pub id: i64,
    pub name: String,
    pub status: String,
    pub title: String,
    pub content: String,
    pub format: String,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TemplateData {
    pub fn map_row(row: &tokio_postgres::Row) -> Self {
        Self {
            id: row.get("id"),
            name: row.get("name"),
            status: row.get("status"),
            title: row.get("title"),
            content: row.get("content"),
            format: row.get("format"),
            version: row.get("version"),
            created_at: row.get::<_, DateTime<Utc>>("created_at"),
            updated_at: row.get::<_, DateTime<Utc>>("updated_at"),
        }
    }

    /// Render templates with
    /// `title` and `content` with `{{key}}` from `variables`.
    pub fn merge_template(&self, variables: &Map<String, Value>) -> (String, String) {
        let mut vars = HashMap::new();
        flatten_variables(variables, &mut vars);

        let escape_json = self.format.eq_ignore_ascii_case("ADAPTIVE_CARD");
        
        let title = truncate_chars(&render_placeholders(&self.title, &vars, false), 255);
        vars.insert("title".to_string(), title.clone());
        let content = render_placeholders(&self.content, &vars, escape_json);
        (title, content)
    }
}

fn flatten_variables(variables: &Map<String, Value>, out: &mut HashMap<String, String>) {
    flatten_object(None, variables, out);
}

fn flatten_object(
    prefix: Option<&str>,
    obj: &Map<String, Value>,
    out: &mut HashMap<String, String>,
) {
    for (key, value) in obj {
        let path = match prefix {
            Some(p) => format!("{p}.{key}"),
            None => key.clone(),
        };

        match value {
            Value::Object(child) => flatten_object(Some(&path), child, out),
            other => insert(out, &path, &value_to_string(other)),
        }

        if prefix.is_none() {
            insert(out, key, &value_to_string(value));
        }
    }
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

fn insert(out: &mut HashMap<String, String>, key: &str, value: &str) {
    out.insert(key.to_string(), value.to_string());
}

fn render_placeholders(
    template: &str,
    vars: &HashMap<String, String>,
    escape_json: bool,
) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find("}}") {
            Some(end) => {
                let key = after[..end].trim();
                let raw = vars.get(key).map(String::as_str).unwrap_or("");
                if escape_json {
                    out.push_str(&json_escape(raw));
                } else {
                    out.push_str(raw);
                }
                rest = &after[end + 2..];
            }
            None => {
                out.push_str("{{");
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

fn json_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    s.chars().take(max).collect()
}

/// Insert-only. Same `name` always gets `version = max(version)+1`.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateTemplateRequest {
    pub name: String,
    pub title: String,
    pub content: String,
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default = "default_active")]
    pub status: String,
}

fn default_active() -> String {
    "ACTIVE".to_string()
}

fn default_format() -> String {
    "TEXT".to_string()
}

impl CreateTemplateRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("name is required".to_string());
        }
        if self.title.trim().is_empty() {
            return Err("title is required".to_string());
        }
        if self.content.trim().is_empty() {
            return Err("content must not be empty".to_string());
        }
        Ok(())
    }
}
