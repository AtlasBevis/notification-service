use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;

use crate::enums::{Status, TemplateFormat};
use crate::error::{AppError, AppResult};
use crate::utils::JsonUtils;

/// A template joined with the version selected for it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateData {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub status: Status,
    pub version_id: i32,
    pub version: i32,
    pub version_status: Status,
    pub title: String,
    pub content: String,
    pub format: TemplateFormat,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl TemplateData {
    pub fn map_row(row: &tokio_postgres::Row) -> AppResult<Self> {
        let status: String = row.get("status");
        let version_status: String = row.get("version_status");
        let format: String = row.get("format");
        Ok(Self {
            id: row.get("id"),
            code: row.get("code"),
            name: row.get::<_, Option<String>>("name").unwrap_or_default(),
            status: Status::parse(&status)
                .ok_or_else(|| AppError::internal(format!("unknown status `{status}`")))?,
            version_id: row.get("version_id"),
            version: row.get("version"),
            version_status: Status::parse(&version_status)
                .ok_or_else(|| AppError::internal(format!("unknown status `{version_status}`")))?,
            title: row.get("title"),
            content: row.get("content"),
            format: TemplateFormat::parse(&format)
                .ok_or_else(|| AppError::internal(format!("unknow format `{format}`")))?,
            created_at: row.get::<_, DateTime<Utc>>("created_at"),
            updated_at: row.get::<_, Option<DateTime<Utc>>>("updated_at"),
        })
    }

    pub fn is_json(&self) -> bool {
        TemplateFormat::Json == self.format
    }

    /// Unrendered webhook body of a `JSON` template.
    pub fn json_payload(&self) -> AppResult<Value> {
        if !self.is_json() {
            return Err(AppError::bad_request(format!(
                "template `{}` format `{}` is not JSON",
                self.code, self.format,
            )));
        }
        JsonUtils::str_to_json(&self.content)
    }

    /// Render templates with
    /// `title` and `content` with `{{key}}` from `variables`.
    pub fn merge_template(&self, variables: &Map<String, Value>) -> (String, String) {
        let mut vars = HashMap::new();
        flatten_variables(variables, &mut vars);

        let escape_json = self.format == TemplateFormat::AdaptiveCard || self.is_json();

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

#[derive(Debug, Clone, Deserialize)]
pub struct SaveTemplateRequest {
    pub code: String,
    pub name: String,
    pub title: String,
    pub content: String,
    pub format: TemplateFormat,
}

impl SaveTemplateRequest {
    pub fn normalize(&mut self) {
        self.code = self.code.trim().to_ascii_uppercase();
        self.name = self.name.trim().to_string();
        self.title = self.title.trim().to_string();
        self.content = self.content.trim().to_string();
    }

    pub fn validate(&self) -> AppResult<()> {
        if self.code.is_empty() {
            return Err(AppError::bad_request("code is required"));
        }
        if self.name.is_empty() {
            return Err(AppError::bad_request("name is required"));
        }
        if self.title.is_empty() {
            return Err(AppError::bad_request("title is required"));
        }
        if self.content.is_empty() {
            return Err(AppError::bad_request("content must not be empty"));
        }
        if TemplateFormat::Json == self.format {
            JsonUtils::is_object(&self.content)?;
        }
        Ok(())
    }
}
