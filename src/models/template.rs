use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;

use crate::enums::Status;
use crate::error::{AppError, AppResult};

pub const FORMAT_ADAPTIVE_CARD: &str = "ADAPTIVE_CARD";
/// Webhook request body; `content` must be a JSON object.
pub const FORMAT_JSON: &str = "JSON";

/// A template joined with the version selected for it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateData {
    pub id: i32,
    pub name: String,
    pub status: Status,
    pub version_id: i32,
    pub version: i32,
    pub version_status: Status,
    pub title: String,
    pub content: String,
    pub format: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl TemplateData {
    pub fn map_row(row: &tokio_postgres::Row) -> AppResult<Self> {
        let status: String = row.get("status");
        let version_status: String = row.get("version_status");
        Ok(Self {
            id: row.get("id"),
            name: row.get("name"),
            status: Status::parse(&status).ok_or_else(|| {
                AppError::internal(format!("unknown status `{status}` in templates"))
            })?,
            version_id: row.get("version_id"),
            version: row.get("version"),
            version_status: Status::parse(&version_status).ok_or_else(|| {
                AppError::internal(format!(
                    "unknown status `{version_status}` in template_versions"
                ))
            })?,
            title: row.get("title"),
            content: row.get("content"),
            format: row.get("format"),
            created_at: row.get::<_, DateTime<Utc>>("created_at"),
            updated_at: row.get::<_, Option<DateTime<Utc>>>("updated_at"),
        })
    }

    pub fn is_json(&self) -> bool {
        self.format.eq_ignore_ascii_case(FORMAT_JSON)
    }

    /// Unrendered webhook body of a `JSON` template.
    pub fn json_payload(&self) -> AppResult<Value> {
        if !self.is_json() {
            return Err(AppError::bad_request(format!(
                "template `{}` format `{}` cannot be used as a webhook body; expected `{FORMAT_JSON}`",
                self.name, self.format
            )));
        }
        parse_json_object(&self.content).map_err(|msg| {
            AppError::bad_request(format!(
                "template `{}` version {}: {msg}",
                self.name, self.version
            ))
        })
    }

    /// Render templates with
    /// `title` and `content` with `{{key}}` from `variables`.
    pub fn merge_template(&self, variables: &Map<String, Value>) -> (String, String) {
        let mut vars = HashMap::new();
        flatten_variables(variables, &mut vars);

        let escape_json = self.format.eq_ignore_ascii_case(FORMAT_ADAPTIVE_CARD) || self.is_json();

        let title = truncate_chars(&render_placeholders(&self.title, &vars, false), 255);
        vars.insert("title".to_string(), title.clone());
        let content = render_placeholders(&self.content, &vars, escape_json);
        (title, content)
    }
}

fn parse_json_object(content: &str) -> AppResult<Value> {
    match serde_json::from_str::<Value>(content) {
        Ok(value) if value.is_object() => Ok(value),
        _ => Err(AppError::bad_request(format!(
            "content must be a JSON object for format `{FORMAT_JSON}`"
        ))),
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

/// Insert-only. Same `name` always gets a new `template_versions` row with
/// `version = max(version)+1`; `status` applies to that version.
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
    pub fn validate(&self) -> AppResult<()> {
        if self.name.trim().is_empty() {
            return Err(AppError::bad_request("name is required"));
        }
        if self.title.trim().is_empty() {
            return Err(AppError::bad_request("title is required"));
        }
        if self.content.trim().is_empty() {
            return Err(AppError::bad_request("content must not be empty"));
        }
        if self.format.trim().eq_ignore_ascii_case(FORMAT_JSON) {
            parse_json_object(&self.content)?;
        }
        Ok(())
    }
}
