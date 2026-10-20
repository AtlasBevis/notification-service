use std::sync::Arc;

use async_trait::async_trait;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde_json::Value;

use crate::config::Config;
use crate::enums::Channel;
use crate::error::{AppError, AppResult};
use crate::models::NotifyValue;
use crate::services::{ChannelTrait, NotifierTrait};

#[derive(Clone)]
pub struct EmailService {
    channels: Arc<dyn ChannelTrait>,
}

impl EmailService {
    pub fn new(channels: Arc<dyn ChannelTrait>) -> Self {
        Self { channels }
    }
}

#[async_trait]
impl NotifierTrait for EmailService {
    fn channel(&self) -> Channel {
        Channel::EMAIL
    }

    async fn notify(&self, request: &NotifyValue) -> AppResult<()> {
        self.channels.get_active_by_id(request.channel_id).await?;
        let smtp = Config::load().smtp.clone();
        if request.recipients.to.is_empty() {
            return Err(AppError::bad_request("email.to is required for EMAIL"));
        }

        let from: lettre::message::Mailbox = smtp
            .username
            .parse()
            .map_err(|_| AppError::internal("config smtp.username is not a valid mailbox"))?;
        let title = request
            .title
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(request.level.as_str());
        let subject = format!("[{}][{}] {}", request.source, request.level, title);
        let text = request
            .message
            .clone()
            .unwrap_or_else(|| Value::Object(request.variables.clone()).to_string());

        let mut builder = Message::builder().from(from).subject(subject);
        for addr in &request.recipients.to {
            let mailbox: lettre::message::Mailbox = addr
                .parse()
                .map_err(|_| AppError::bad_request(format!("invalid email recipient `{addr}`")))?;
            builder = builder.to(mailbox);
        }
        for addr in &request.recipients.cc {
            let mailbox: lettre::message::Mailbox = addr
                .parse()
                .map_err(|_| AppError::bad_request(format!("invalid email cc `{addr}`")))?;
            builder = builder.cc(mailbox);
        }
        for addr in &request.recipients.bcc {
            let mailbox: lettre::message::Mailbox = addr
                .parse()
                .map_err(|_| AppError::bad_request(format!("invalid email bcc `{addr}`")))?;
            builder = builder.bcc(mailbox);
        }
        let email = builder
            .body(text)
            .map_err(|_| AppError::internal("failed to build email message"))?;

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&smtp.host)
            .map_err(|_| AppError::internal(format!("invalid SMTP host `{}`", smtp.host)))?
            .credentials(Credentials::new(smtp.username, smtp.password))
            .build();
        mailer
            .send(email)
            .await
            .map_err(|_| AppError::internal(format!("SMTP send via {} failed", smtp.host)))?;
        tracing::debug!(source = %request.source, "Email sent");
        Ok(())
    }
}
