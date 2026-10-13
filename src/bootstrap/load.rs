use crate::error::{AppError, AppResult};
use crate::services::{ChannelTrait, SourceTrait};

/// Load channels and sources from Postgres into each service's memory cache.
/// Targets are not cached; they are read from Postgres on demand.
pub async fn load_memory(channels: &dyn ChannelTrait, sources: &dyn SourceTrait) -> AppResult<()> {
    channels
        .load()
        .await
        .map_err(|e| AppError::internal_with("Failed to load channels", e))?;
    sources
        .load()
        .await
        .map_err(|e| AppError::internal_with("Failed to load sources", e))?;
    Ok(())
}
