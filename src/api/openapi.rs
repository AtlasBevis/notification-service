use crate::api::{notification, notify, sources, targets, templates};
use crate::enums::NotificationStatus;
use crate::models::{
    CreateNotificationRequest, CreateSourceRequest, CreateTargetRequest, CreateTemplateRequest,
    NotificationData, NotifyRequest, PatchMetadataRequest, Recipients, SourceData, TargetData,
    TemplateData, UpdateNotificationRequest,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Data Notification",
        version = "0.1.0",
        description = "Data Platform Notification"
    ),
    servers(
        (url = "/", description = "localhost")
    ),
    paths(
        notification::list_notifications,
        notification::create_notification,
        notification::get_notification,
        notification::update_notification,
        notification::delete_notification,
        notify::notify,
        sources::get_source,
        sources::create_source,
        sources::patch_metadata,
        targets::get_target,
        targets::create_target,
        targets::patch_metadata,
        templates::create_template,
    ),
    components(
        schemas(
            NotifyRequest,
            CreateNotificationRequest,
            UpdateNotificationRequest,
            NotificationData,
            Recipients,
            CreateSourceRequest,
            CreateTargetRequest,
            PatchMetadataRequest,
            CreateTemplateRequest,
            SourceData,
            TargetData,
            TemplateData,
            NotificationStatus,
        )
    ),
    tags(
        (name = "Notification", description = "Notification configs"),
        (name = "Notify", description = "Trigger a configured notification"),
        (name = "Sources", description = "Notification sources (calling entries)"),
        (name = "Targets", description = "Delivery destinations"),
        (name = "Templates", description = "Notification templates"),
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        for header in ["api-key"] {
            components.add_security_scheme(
                header,
                utoipa::openapi::security::SecurityScheme::ApiKey(
                    utoipa::openapi::security::ApiKey::Header(
                        utoipa::openapi::security::ApiKeyValue::new(header),
                    ),
                ),
            );
        }
    }
}
