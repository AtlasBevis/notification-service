use async_trait::async_trait;
use deadpool_postgres::Transaction;
use serde_json::Value;

use crate::enums::Status;
use crate::error::{AppError, AppResult};
use crate::models::{SaveTemplateRequest, TemplateData};
use crate::repository::TemplateRepository;

#[async_trait]
pub trait TemplateTrait: Send + Sync {
    async fn get_active_by_id(&self, id: i32) -> AppResult<TemplateData>;
    async fn create(&self, req: SaveTemplateRequest) -> AppResult<TemplateData>;
}

#[derive(Clone)]
pub struct TemplateService {
    repo: TemplateRepository,
}

impl TemplateService {
    pub fn new(repo: TemplateRepository) -> Self {
        Self { repo }
    }

    /// Saves a new version of template `req.code` in one transaction,
    /// creating the template on first use. The template row stays locked
    /// until commit, so concurrent saves of one code take consecutive
    /// version numbers.
    async fn save(&self, req: &SaveTemplateRequest) -> AppResult<TemplateData> {
        let mut client = self.repo.client().await?;
        let tx = client.transaction().await?;

        let template_id = self.lock_or_create(&tx, req).await?;
        let version_id = self.repo.insert_version(&tx, template_id, req).await?;
        let data = self.repo.find_by_version_id(&tx, version_id).await?;

        tx.commit().await?;
        Ok(data)
    }

    /// Locks the template row of `req.code`, creating it if missing. Each way
    /// of getting the row is its own branch with its own log line:
    ///
    /// - the template exists: lock it;
    /// - it does not exist: insert it;
    /// - another request inserted it after our lookup: our insert does
    ///   nothing, so lock the row that request created.
    async fn lock_or_create(
        &self,
        tx: &Transaction<'_>,
        req: &SaveTemplateRequest,
    ) -> AppResult<i32> {
        let code = req.code.as_str();
        if let Some(id) = self.repo.lock_by_code(tx, code).await? {
            return Ok(id);
        }
        if let Some(id) = self.repo.insert_template(tx, code, &req.name).await? {
            return Ok(id);
        }
        let id = self.repo.lock_by_code(tx, code).await?.ok_or_else(|| {
            AppError::internal(format!(
                "template `{code}` conflicted on insert but cannot be found"
            ))
        })?;
        tracing::debug!(
            code,
            template_id = id,
            "template created by a concurrent request, adding a version"
        );
        Ok(id)
    }
}

#[async_trait]
impl TemplateTrait for TemplateService {
    async fn get_active_by_id(&self, id: i32) -> AppResult<TemplateData> {
        let t = self.repo.find_by_id(id).await?;
        if Status::Active != t.status {
            return Err(AppError::bad_request(format!(
                "template id `{id}` is not ACTIVE"
            )));
        }
        Ok(t)
    }

    async fn create(&self, req: SaveTemplateRequest) -> AppResult<TemplateData> {
        match self.save(&req).await {
            Ok(row) => Ok(row),
            Err(AppError::Conflict(_)) => Err(AppError::conflict(format!(
                "template `{}` version already exists",
                req.code
            ))),
            Err(err) => {
                tracing::error!(error = %err, code = %req.code, "insert template version failed");
                Err(err)
            }
        }
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
