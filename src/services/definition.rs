use crate::enums::{Channel, Status};
use crate::error::{AppError, AppResult};
use crate::models::{
    CreateDefinitionRequest, DefinitionData, DefinitionInternal, DefinitionTargetRequest, PageData,
    PageRequest, PutDefinitionRequest, TargetInternalData, TemplateData,
};
use crate::repository::DefinitionRepository;
use crate::services::{ChannelTrait, SourceTrait, TargetTrait, TemplateTrait};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

#[async_trait]
pub trait DefinitionTrait: Send + Sync {
    async fn get_by_code(&self, code: &str) -> AppResult<DefinitionData>;
    async fn get_active_by_code(&self, code: &str) -> AppResult<DefinitionData>;
    async fn list(&self, page: PageRequest) -> AppResult<PageData<DefinitionData>>;
    async fn create(&self, req: CreateDefinitionRequest) -> AppResult<DefinitionData>;
    async fn update(&self, code: &str, req: PutDefinitionRequest) -> AppResult<DefinitionData>;
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

    async fn validate_create(
        &self,
        req: &CreateDefinitionRequest,
    ) -> AppResult<DefinitionInternal> {
        self.validate_links(&req.source, req.channel, &req.targets)
            .await
    }

    async fn validate_put(&self, req: &PutDefinitionRequest) -> AppResult<DefinitionInternal> {
        self.validate_links(&req.source, req.channel, &req.targets)
            .await
    }

    async fn validate_links(
        &self,
        source: &str,
        channel: Channel,
        items: &[DefinitionTargetRequest],
    ) -> AppResult<DefinitionInternal> {
        // source
        let source = match self.sources.get_by_code(source).await {
            Ok(data) => data,
            Err(AppError::NotFound(_)) => {
                return Err(AppError::bad_request(format!("unknown source `{source}`")))
            }
            Err(err) => return Err(err),
        };

        // channel
        let channel = match self.channels.get_by_code(channel).await {
            Ok(data) => data,
            Err(AppError::NotFound(_)) => {
                return Err(AppError::bad_request(format!(
                    "unknown channel `{channel}`"
                )))
            }
            Err(err) => return Err(err),
        };
        let is_webhook = channel.code == Channel::WEBHOOK;

        // get targets
        let codes: Vec<String> = items.iter().map(|i| i.code.clone()).collect();
        // get list code
        let found = self.targets.get_by_list_code(&codes).await?;
        // map
        let target_map: HashMap<&str, &TargetInternalData> =
            found.iter().map(|t| (t.code.as_str(), t)).collect();

        // get templates
        let mut template_codes: Vec<String> = items
            .iter()
            .filter_map(|i| i.template_code.clone())
            .collect();
        template_codes.dedup();

        // get templates by codes
        let templates = self.templates.get_by_codes(&template_codes).await?;
        let template_map: HashMap<&str, &TemplateData> =
            templates.iter().map(|t| (t.code.as_str(), t)).collect();

        let mut target_ids = Vec::with_capacity(items.len());
        let mut template_ids = Vec::with_capacity(items.len());
        for item in items {
            let Some(&target) = target_map.get(item.code.as_str()) else {
                return Err(AppError::bad_request(format!(
                    "unknown target `{}`",
                    item.code
                )));
            };
            if target.channel_id != channel.id {
                return Err(AppError::bad_request(format!(
                    "target `{}` does not belong to channel `{}`",
                    target.code, channel.code
                )));
            }

            let template_id = match &item.template_code {
                Some(template_code) => {
                    let Some(&template) = template_map.get(template_code.as_str()) else {
                        return Err(AppError::bad_request(format!(
                            "template `{template_code}` not found or has no ACTIVE version"
                        )));
                    };
                    if template.status != Status::Active {
                        return Err(AppError::bad_request(format!(
                            "template `{template_code}` is not ACTIVE"
                        )));
                    }
                    if template.is_json() != is_webhook {
                        return Err(AppError::bad_request(format!(
                            "template `{}` format `{}` is not valid for channel `{}`",
                            template.code, template.format, channel.code
                        )));
                    }
                    Some(template.id)
                }
                None => None,
            };

            target_ids.push(target.id);
            template_ids.push(template_id);
        }

        if !is_webhook && template_ids.iter().all(Option::is_none) {
            return Err(AppError::bad_request(format!(
                "channel `{}` requires a template_code on at least one target",
                channel.code
            )));
        }

        Ok(DefinitionInternal {
            source,
            channel,
            target_ids,
            template_ids,
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

    async fn create(&self, req: CreateDefinitionRequest) -> AppResult<DefinitionData> {
        let resolved = self.validate_create(&req).await?;
        match self
            .repo
            .insert(
                &req,
                resolved.source.id,
                resolved.channel.id,
                &resolved.target_ids,
                &resolved.template_ids,
            )
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

    async fn update(&self, code: &str, req: PutDefinitionRequest) -> AppResult<DefinitionData> {
        let resolved = self.validate_put(&req).await?;
        match self
            .repo
            .update(
                code,
                &req,
                resolved.source.id,
                resolved.channel.id,
                &resolved.target_ids,
                &resolved.template_ids,
            )
            .await
        {
            Ok(row) => Ok(row),
            Err(AppError::NotFound(_)) => Err(AppError::not_found(format!(
                "definition `{code}` not found"
            ))),
            Err(err) => {
                tracing::error!(error = %err, code, "update definition failed");
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
