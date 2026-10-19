use crate::enums::{Channel, Status};
use crate::error::{AppError, AppResult};
use crate::models::{
    CreateDefinitionRequest, DefinitionData, DefinitionInternal, DefinitionTarget, PageData,
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
            Ok(source) => source,
            Err(AppError::NotFound(_)) => {
                return Err(AppError::bad_request(format!("unknown source `{source}`")))
            }
            Err(err) => return Err(err),
        };

        // channel
        let channel = match self.channels.get_by_code(channel_code).await {
            Ok(channel) => channel,
            Err(AppError::NotFound(_)) => {
                return Err(AppError::bad_request(format!(
                    "unknown channel `{channel_code}`"
                )))
            }
            Err(err) => return Err(err),
        };
        let is_webhook = channel.code == Channel::WEBHOOK;

        // get targets
        let codes: Vec<String> = items.iter().map(|i| i.target_code.clone()).collect();
        let found = self.targets.get_by_list_code(&codes).await?;
        let by_code: HashMap<&str, &TargetInternalData> =
            found.iter().map(|t| (t.code.as_str(), t)).collect();

        let mut links = Vec::with_capacity(items.len());
        let mut targets_map = HashMap::with_capacity(items.len());
        for item in items {
            let Some(&target) = by_code.get(item.target_code.as_str()) else {
                return Err(AppError::bad_request(format!(
                    "unknown target `{}`",
                    item.target_code
                )));
            };
            if target.channel_id != channel.id {
                return Err(AppError::bad_request(format!(
                    "target `{}` does not belong to channel `{}`",
                    target.code, channel.code
                )));
            }
            links.push(DefinitionTarget {
                target_id: target.id,
                template_id: item.template_id,
            });
            targets_map.insert(target.id, target.clone());
        }

        // templates: one query for every template_id the targets link to
        let mut template_ids: Vec<i32> = links.iter().filter_map(|l| l.template_id).collect();
        template_ids.sort_unstable();
        template_ids.dedup();
        let templates: HashMap<i32, TemplateData> = self
            .templates
            .get_by_ids(&template_ids)
            .await?
            .into_iter()
            .map(|t| (t.id, t))
            .collect();

        let mut template_map = HashMap::with_capacity(links.len());
        for link in &links {
            let template = match link.template_id {
                Some(template_id) => {
                    let Some(template) = templates.get(&template_id) else {
                        return Err(AppError::bad_request(format!(
                            "template id `{template_id}` has no ACTIVE version"
                        )));
                    };
                    if template.status != Status::Active {
                        return Err(AppError::bad_request(format!(
                            "template id `{template_id}` is not ACTIVE"
                        )));
                    }
                    if template.is_json() != is_webhook {
                        return Err(AppError::bad_request(format!(
                            "template `{}` format `{}` is not valid for channel `{}`",
                            template.name, template.format, channel.code
                        )));
                    }
                    Some(template.clone())
                }
                None => None,
            };
            template_map.insert(link.target_id, template);
        }

        if !is_webhook && links.iter().all(|t| t.template_id.is_none()) {
            return Err(AppError::bad_request(format!(
                "channel `{}` requires a template_id on at least one target",
                channel.code
            )));
        }

        Ok(DefinitionInternal {
            source,
            channel,
            targets: DefinitionTargetDataInternal {
                links,
                targets_map,
                template_map,
            },
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
                &resolved.targets.links,
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
                &resolved.targets.links,
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
