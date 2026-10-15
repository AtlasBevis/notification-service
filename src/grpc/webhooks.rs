use std::sync::Arc;

use tonic::{Request, Response, Status};

use super::convert::ProtoConvert;
use super::pb::webhooks_server::Webhooks;
use super::pb::{
    CodeRequest, SaveWebhookAuthRequest as SaveWebhookAuthRpcRequest,
    SaveWebhookRequest as SaveWebhookRpcRequest, Webhook, WebhookAuth, WebhookList,
};
use crate::error::{AppError, AppResult};
use crate::models::{SaveWebhookAuthRequest, SaveWebhookRequest, WebhookData};
use crate::services::WebhookServiceTrait;

/// gRPC counterpart of `WebhookController`; webhooks are addressed by target code.
pub struct WebhookGrpc {
    svc: Arc<dyn WebhookServiceTrait>,
}

impl WebhookGrpc {
    pub fn new(svc: Arc<dyn WebhookServiceTrait>) -> Self {
        Self { svc }
    }

    /// Omitted optional fields take the HTTP body's defaults.
    fn to_request(request: SaveWebhookRpcRequest) -> AppResult<SaveWebhookRequest> {
        let auth = request
            .auth
            .ok_or_else(|| AppError::bad_request("auth is required"))?;
        Ok(SaveWebhookRequest {
            target_code: request.target_code,
            domain: request.domain,
            path: request.path,
            method: request
                .method
                .unwrap_or_else(SaveWebhookRequest::default_method),
            timeout_seconds: request
                .timeout_seconds
                .unwrap_or_else(SaveWebhookRequest::default_timeout_seconds),
            max_retries: request
                .max_retries
                .unwrap_or_else(SaveWebhookRequest::default_max_retries),
            headers: ProtoConvert::map(request.headers),
            auth: Self::to_auth_request(auth),
        })
    }

    fn to_auth_request(auth: SaveWebhookAuthRpcRequest) -> SaveWebhookAuthRequest {
        SaveWebhookAuthRequest {
            auth_mode: auth
                .auth_mode
                .unwrap_or_else(SaveWebhookRequest::default_auth_mode),
            header_name: auth
                .header_name
                .unwrap_or_else(SaveWebhookRequest::default_header_name),
            header_value: auth.header_value,
            header_prefix: auth
                .header_prefix
                .unwrap_or_else(SaveWebhookRequest::default_header_prefix),
            token_url: auth.token_url,
            token_method: auth
                .token_method
                .unwrap_or_else(SaveWebhookRequest::default_method),
            token_body_format: auth
                .token_body_format
                .unwrap_or_else(SaveWebhookRequest::default_token_body_format),
            request_headers: ProtoConvert::map(auth.request_headers),
            request_body: ProtoConvert::map(auth.request_body),
            timeout_seconds: auth
                .timeout_seconds
                .unwrap_or_else(SaveWebhookRequest::default_timeout_seconds),
            token_path: auth
                .token_path
                .unwrap_or_else(SaveWebhookRequest::default_token_path),
            // An empty string clears it (`normalize()` drops blank paths).
            expires_in_path: auth
                .expires_in_path
                .or_else(SaveWebhookRequest::default_expires_in_path),
            default_ttl_seconds: auth
                .default_ttl_seconds
                .unwrap_or_else(SaveWebhookRequest::default_ttl_seconds),
        }
    }

    fn to_reply(data: WebhookData) -> Webhook {
        Webhook {
            id: data.id,
            target_id: data.target_id,
            target_code: data.target_code,
            domain: data.domain,
            path: data.path,
            method: data.method,
            timeout_seconds: data.timeout_seconds,
            max_retries: data.max_retries,
            headers: ProtoConvert::to_struct(&data.headers),
            auth: data.auth.map(|auth| WebhookAuth {
                id: auth.id,
                auth_mode: auth.auth_mode,
                header_name: auth.header_name,
                has_header_value: auth.has_header_value,
                header_prefix: auth.header_prefix,
                token_url: auth.token_url,
                token_method: auth.token_method,
                token_body_format: auth.token_body_format,
                token_path: auth.token_path,
                expires_in_path: auth.expires_in_path,
                timeout_seconds: auth.timeout_seconds,
                default_ttl_seconds: auth.default_ttl_seconds,
            }),
            created_at: Some(ProtoConvert::timestamp(data.created_at)),
            updated_at: data.updated_at.map(ProtoConvert::timestamp),
        }
    }
}

#[tonic::async_trait]
impl Webhooks for WebhookGrpc {
    async fn list_webhooks(&self, _request: Request<()>) -> Result<Response<WebhookList>, Status> {
        let data = self.svc.list().await.map_err(ProtoConvert::status)?;
        Ok(Response::new(WebhookList {
            items: data.into_iter().map(Self::to_reply).collect(),
        }))
    }

    async fn get_webhook(
        &self,
        request: Request<CodeRequest>,
    ) -> Result<Response<Webhook>, Status> {
        let key = ProtoConvert::code(&request.into_inner().code, "target_code")
            .map_err(ProtoConvert::status)?;
        let data = self
            .svc
            .get_by_target_code(&key)
            .await
            .map_err(ProtoConvert::status)?;
        Ok(Response::new(Self::to_reply(data)))
    }

    async fn create_webhook(
        &self,
        request: Request<SaveWebhookRpcRequest>,
    ) -> Result<Response<Webhook>, Status> {
        let mut payload = Self::to_request(request.into_inner()).map_err(ProtoConvert::status)?;
        payload.normalize();
        payload.validate().map_err(ProtoConvert::status)?;

        let data = self
            .svc
            .create(payload)
            .await
            .map_err(ProtoConvert::status)?;
        Ok(Response::new(Self::to_reply(data)))
    }

    async fn update_webhook(
        &self,
        request: Request<SaveWebhookRpcRequest>,
    ) -> Result<Response<Webhook>, Status> {
        let mut payload = Self::to_request(request.into_inner()).map_err(ProtoConvert::status)?;
        payload.normalize();
        payload.validate().map_err(ProtoConvert::status)?;

        let data = self
            .svc
            .update(payload)
            .await
            .map_err(ProtoConvert::status)?;
        Ok(Response::new(Self::to_reply(data)))
    }

    async fn delete_webhook(&self, request: Request<CodeRequest>) -> Result<Response<()>, Status> {
        let key = ProtoConvert::code(&request.into_inner().code, "target_code")
            .map_err(ProtoConvert::status)?;
        self.svc.delete(&key).await.map_err(ProtoConvert::status)?;
        Ok(Response::new(()))
    }
}
