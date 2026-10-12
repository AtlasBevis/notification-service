use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::error::{AppError, AppResult};
use crate::infras::http::HttpClient;
use crate::models::NotifyValue;
use crate::services::template::wrap_teams_message;
use crate::services::{ChannelTrait, TargetTrait};

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
        let channel = self
            .channels
            .must_active(&request.channel)
            .await
            .map_err(AppError::bad_request)?;
        let target = self
            .targets
            .must_active(&request.target)
            .await
            .map_err(AppError::bad_request)?;
        let url = target
            .teams_webhook_url()
            .or_else(|| channel.teams_webhook_url().ok())
            .ok_or_else(|| {
                AppError::bad_request(format!(
                    "Teams webhook url missing on target `{}` and channel `{}`",
                    request.target, request.channel
                ))
            })?;

        let raw = request
            .message
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| AppError::bad_request("rendered Teams card is empty"))?;
        let card: Value = serde_json::from_str(raw)
            .map_err(|_| AppError::bad_request("rendered Teams template is not valid JSON"))?;
        self.http
            .post_json(&url, &HashMap::new(), &wrap_teams_message(card))
            .await
            .map_err(|err| AppError::internal(err.to_string()))?;
        Ok(())
    }
}
