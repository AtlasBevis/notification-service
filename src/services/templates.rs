use async_trait::async_trait;
use serde_json::Value;

use crate::error::{AppError, AppResult};
use crate::models::{CreateTemplateRequest, TemplateData};
use crate::repository::TemplatesRepository;

#[async_trait]
pub trait TemplatesServiceTrait: Send + Sync {
    async fn must_active_by_id(&self, id: i64) -> Result<TemplateData, String>;
    async fn create(&self, req: CreateTemplateRequest) -> AppResult<TemplateData>;
}

#[derive(Clone)]
pub struct TemplatesService {
    repo: TemplatesRepository,
}

impl TemplatesService {
    pub fn new(repo: TemplatesRepository) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl TemplatesServiceTrait for TemplatesService {
    async fn must_active_by_id(&self, id: i64) -> Result<TemplateData, String> {
        match self.repo.find_by_id(id).await {
            Ok(Some(t)) => is_active(t, id),
            Ok(None) => Err(format!("unknown template id `{id}`")),
            Err(err) => Err(format!("failed to load template `{id}`: {err}")),
        }
    }

    async fn create(&self, req: CreateTemplateRequest) -> AppResult<TemplateData> {
        let name = req.name.trim();
        let version = self.repo.next_version(name).await?;

        match self.repo.insert(&req, version).await {
            Ok(row) => {
                tracing::info!(
                    id = row.id,
                    name = %row.name,
                    version = row.version,
                    "Template inserted"
                );
                Ok(row)
            }
            Err(AppError::Conflict(_)) => Err(AppError::conflict(format!(
                "template `{}` version {} already exists",
                req.name.trim(),
                version
            ))),
            Err(err) => {
                tracing::error!(error = %err, name = %req.name, "insert template failed");
                Err(err)
            }
        }
    }
}

fn is_active(t: TemplateData, id: i64) -> Result<TemplateData, String> {
    if t.status.eq_ignore_ascii_case("ACTIVE") {
        Ok(t)
    } else {
        Err(format!("template id `{id}` is not ACTIVE"))
    }
}

pub fn wrap_teams_message(card: Value) -> Value {
    serde_json::json!({
        "type": "message",
        "attachments": [{
            "contentType": "application/vnd.microsoft.teams.card.adaptive",
            "contentUrl": null,
            "content": card
        }]
    })
}
