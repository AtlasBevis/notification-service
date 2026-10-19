use crate::enums::{Channel, Status};
use crate::error::{AppError, AppResult};
use crate::models::{
    DefinitionData, DefinitionTarget, PageData, PageRequest, SaveDefinitionRequest,
};
use crate::repository::DefinitionRepository;
use crate::services::{ChannelTrait, SourceTrait, TargetTrait, TemplateTrait};
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait DefinitionTrait: Send + Sync {
    async fn get_by_code(&self, code: &str) -> AppResult<DefinitionData>;
    async fn get_active_by_code(&self, code: &str) -> AppResult<DefinitionData>;
    async fn list(&self, page: PageRequest) -> AppResult<PageData<DefinitionData>>;
    async fn create(&self, req: SaveDefinitionRequest) -> AppResult<DefinitionData>;
    async fn update(&self, req: SaveDefinitionRequest) -> AppResult<DefinitionData>;
    async fn delete(&self, code: &str) -> AppResult<()>;
}

#[derive(Clone)]
pub struct DefinitionService {
    repo: DefinitionRepository,
    sources: Arc<dyn SourceTrait>,
    channels: Arc<dyn ChannelTrait>,
    targets: Arc<dyn TargetTrait>,
    templates: Arc<dyn TemplateTrait>,
}

/// Ids a `SaveDefinitionRequest` resolves to.
struct ResolvedLinks {
    source_id: i32,
    channel_id: i32,
    targets: Vec<DefinitionTarget>,
}

impl DefinitionService {
    pub fn new(
        repo: DefinitionRepository,
        sources: Arc<dyn SourceTrait>,
        channels: Arc<dyn ChannelTrait>,
        targets: Arc<dyn TargetTrait>,
        templates: Arc<dyn TemplateTrait>,
    ) -> Self {
        Self {
            repo,
            sources,
            channels,
            targets,
            templates,
        }
    }

    /// Resolves the codes in the request and applies the rules a send checks
    /// later, so a definition that is saved can be sent.
    async fn resolve(&self, req: &SaveDefinitionRequest) -> AppResult<ResolvedLinks> {
        let source = match self.sources.get_by_code(&req.source_code).await {
            Ok(source) => source,
            Err(AppError::NotFound(_)) => {
                return Err(AppError::bad_request(format!(
                    "unknown source `{}`",
                    req.source_code
                )))
            }
            Err(err) => return Err(err),
        };
        let channel = match self.channels.get_by_code(req.channel).await {
            Ok(channel) => channel,
            Err(AppError::NotFound(_)) => {
                return Err(AppError::bad_request(format!(
                    "unknown channel `{}`",
                    req.channel
                )))
            }
            Err(err) => return Err(err),
        };
        let is_webhook = channel.code == Channel::WEBHOOK;

        let mut targets = Vec::with_capacity(req.targets.len());
        for item in &req.targets {
            let target = match self.targets.get_by_code(&item.target_code).await {
                Ok(target) => target,
                Err(AppError::NotFound(_)) => {
                    return Err(AppError::bad_request(format!(
                        "unknown target `{}`",
                        item.target_code
                    )))
                }
                Err(err) => return Err(err),
            };
            if target.channel != channel.code {
                return Err(AppError::bad_request(format!(
                    "target `{}` does not belong to channel `{}`",
                    target.code, channel.code
                )));
            }
            if let Some(template_id) = item.template_id {
                let template = match self.templates.get_active_by_id(template_id).await {
                    Ok(template) => template,
                    Err(AppError::NotFound(msg)) => return Err(AppError::bad_request(msg)),
                    Err(err) => return Err(err),
                };
                if template.is_json() != is_webhook {
                    return Err(AppError::bad_request(format!(
                        "template `{}` format `{}` is not valid for channel `{}`",
                        template.name, template.format, channel.code
                    )));
                }
            }
            targets.push(DefinitionTarget {
                target_id: target.id,
                template_id: item.template_id,
            });
        }
        if !is_webhook && targets.iter().all(|t| t.template_id.is_none()) {
            return Err(AppError::bad_request(format!(
                "channel `{}` requires a template_id on at least one target",
                channel.code
            )));
        }

        Ok(ResolvedLinks {
            source_id: source.id,
            channel_id: channel.id,
            targets,
        })
    }
}

#[async_trait]
impl DefinitionTrait for DefinitionService {
    async fn get_by_code(&self, code: &str) -> AppResult<DefinitionData> {
        match self.repo.find_by_code(code).await {
            Ok(d) => Ok(d),
            Err(err) => {
                tracing::error!(error = %err, code, "definition get_by_code err");
                Err(err)
            }
        }
    }

    async fn get_active_by_code(&self, code: &str) -> AppResult<DefinitionData> {
        let d = self.get_by_code(code).await?;
        if d.status != Status::Active {
            return Err(AppError::bad_request(format!(
                "definition `{code}` is not ACTIVE",
            )));
        }
        Ok(d)
    }

    async fn list(&self, page: PageRequest) -> AppResult<PageData<DefinitionData>> {
        match self.repo.list_page(&page).await {
            Ok(data) => Ok(data),
            Err(err) => {
                tracing::error!(error = %err, "list definitions failed");
                Err(err)
            }
        }
    }

    async fn create(&self, req: SaveDefinitionRequest) -> AppResult<DefinitionData> {
        let links = self.resolve(&req).await?;
        match self
            .repo
            .insert(&req, links.source_id, links.channel_id, &links.targets)
            .await
        {
            Ok(row) => Ok(row),
            Err(AppError::Conflict(_)) => Err(AppError::conflict(format!(
                "definition `{}` already exists",
                req.code
            ))),
            Err(err) => {
                tracing::error!(error = %err, code = %req.code, "insert definition failed");
                Err(err)
            }
        }
    }

    async fn update(&self, req: SaveDefinitionRequest) -> AppResult<DefinitionData> {
        let links = self.resolve(&req).await?;
        match self
            .repo
            .update(&req, links.source_id, links.channel_id, &links.targets)
            .await
        {
            Ok(row) => Ok(row),
            Err(AppError::NotFound(_)) => Err(AppError::not_found(format!(
                "definition `{}` not found",
                req.code
            ))),
            Err(err) => {
                tracing::error!(error = %err, code = %req.code, "update definition failed");
                Err(err)
            }
        }
    }

    async fn delete(&self, code: &str) -> AppResult<()> {
        match self.repo.delete(code).await {
            Ok(()) => Ok(()),
            Err(AppError::NotFound(_)) => Err(AppError::not_found(format!(
                "definition `{code}` not found"
            ))),
            Err(err) => {
                tracing::error!(error = %err, code, "delete definition failed");
                Err(err)
            }
        }
    }
}
