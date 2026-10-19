use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::enums::Status;
use crate::error::{AppError, AppResult};
use crate::models::{PageData, PageRequest, SourceData, SourceRequest};
use crate::repository::SourceRepository;

#[async_trait]
pub trait SourceTrait: Send + Sync {
    async fn list(&self, page: PageRequest) -> AppResult<PageData<SourceData>>;
    async fn get_by_code(&self, code: &str) -> AppResult<SourceData>;
    async fn get_active_by_code(&self, code: &str) -> AppResult<SourceData>;
    async fn get_active_by_id(&self, id: i32) -> AppResult<SourceData>;
    async fn create(&self, req: SourceRequest) -> AppResult<()>;
    async fn update(&self, req: SourceRequest) -> AppResult<()>;
}

#[derive(Clone)]
pub struct SourceService {
    repo: SourceRepository,
    cache: Arc<RwLock<HashMap<String, SourceData>>>,
}

impl SourceService {
    /// Loads every source into the memory cache before returning.
    pub async fn new(repo: SourceRepository) -> AppResult<Self> {
        let rows = repo.list().await?;

        let mut map = HashMap::new();
        for row in rows {
            map.insert(row.code.clone(), row);
        }

        Ok(Self {
            repo,
            cache: Arc::new(RwLock::new(map)),
        })
    }

    async fn get(&self, code: &str) -> Option<SourceData> {
        self.cache.read().await.get(code).cloned()
    }
}

#[async_trait]
impl SourceTrait for SourceService {
    async fn list(&self, page: PageRequest) -> AppResult<PageData<SourceData>> {
        self.repo.list_page(&page).await
    }

    async fn get_by_code(&self, code: &str) -> AppResult<SourceData> {
        if let Some(s) = self.get(code).await {
            return Ok(s);
        }

        match self.repo.find_by_code(code).await {
            Ok(s) => {
                self.cache.write().await.insert(code.to_owned(), s.clone());
                Ok(s)
            }
            Err(err) => {
                tracing::error!(error = %err, code, "find source by code failed");
                Err(err)
            }
        }
    }

    async fn get_active_by_code(&self, code: &str) -> AppResult<SourceData> {
        let s = self.get_by_code(code).await?;
        if s.status != Status::Active {
            return Err(AppError::bad_request(format!(
                "source `{code}` is not ACTIVE",
            )));
        }
        Ok(s)
    }

    async fn get_active_by_id(&self, id: i32) -> AppResult<SourceData> {
        let s = self.repo.find_by_id(id).await?;
        if s.status != Status::Active {
            return Err(AppError::bad_request(format!(
                "source `{id}` is not ACTIVE",
            )));
        }
        Ok(s)
    }

    async fn create(&self, req: SourceRequest) -> AppResult<()> {
        if self.get_by_code(&req.code).await.is_ok() {
            return Err(AppError::conflict(format!(
                "source `{}` already exists",
                req.code
            )));
        }

        match self.repo.insert(&req).await {
            Ok(_) => Ok(()),
            Err(err) => {
                tracing::error!(error = %err, code = %req.code, "insert source failed");
                Err(err)
            }
        }
    }

    async fn update(&self, req: SourceRequest) -> AppResult<()> {
        match self.repo.update(&req).await {
            Ok(()) => {
                self.cache.write().await.remove(&req.code);
                Ok(())
            }
            Err(AppError::NotFound(_)) => Err(AppError::not_found(format!(
                "source `{}` not found",
                req.code
            ))),
            Err(err) => {
                tracing::error!(error = %err, code = %req.code, "update source failed");
                Err(err)
            }
        }
    }
}
