use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::error::{AppError, AppResult};
use crate::infras::http::HttpClient;
use crate::models::{NotifyValue, TargetData};
use crate::services::template::wrap_teams_message;
use crate::services::{ChannelTrait, TargetTrait};
use crate::utils::JsonUtils;

#[async_trait]
pub trait MSTrait: Send + Sync {
    async fn send_teams(&self, request: &NotifyValue) -> AppResult<()>;
}

#[derive(Clone)]
pub struct MSService {
    http: Arc<HttpClient>,
    channels: Arc<dyn ChannelTrait>,
    targets: Arc<dyn TargetTrait>,
}

impl MSService {
    /// MSTEAMS webhook URLs are not stored yet, so every target fails here.
    fn teams_url(target: &TargetData) -> AppResult<String> {
        Err(AppError::bad_request(format!(
            "target `{}` has no Teams webhook URL (MSTEAMS is not supported yet)",
            target.code
        )))
    }

    pub fn new(
        http: Arc<HttpClient>,
        channels: Arc<dyn ChannelTrait>,
        targets: Arc<dyn TargetTrait>,
    ) -> Self {
        Self {
            http,
            channels,
            targets,
        }
    }
}

#[async_trait]
impl MSTrait for MSService {
    async fn send_teams(&self, request: &NotifyValue) -> AppResult<()> {
        self.channels.get_active_by_id(request.channel_id).await?;
        let raw = request
            .message
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| AppError::bad_request("rendered Teams card is empty"))?;
        let card: Value = JsonUtils::str_to_json(raw)
            .map_err(|_| AppError::bad_request("rendered Teams template is not valid JSON"))?;
        let mut failures = Vec::new();
        for target_id in &request.target_ids {
            let result = async {
                let target = self.targets.get_active_by_id(*target_id).await?;
                let url = Self::teams_url(&target)?;
                self.http
                    .post_json(&url, &HashMap::new(), &wrap_teams_message(card.clone()))
                    .await
                    .map_err(|err| AppError::internal(err.to_string()))?;
                Ok::<(), AppError>(())
            }
            .await;
            if let Err(error) = result {
                failures.push(error.to_string());
            }
        }
        if !failures.is_empty() {
            return Err(AppError::internal(format!(
                "Teams delivery failed: {}",
                failures.join("; ")
            )));
        }
        Ok(())
    }
}
