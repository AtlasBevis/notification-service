use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

/// Paging input shared by every list API (HTTP query `?page=&size=`, gRPC `PageRequest`).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PageRequest {
    pub page: u32,
    pub size: u32,
}

impl PageRequest {
    pub const DEFAULT_PAGE: u32 = 1;
    pub const DEFAULT_SIZE: u32 = 10;
    pub const MAX_SIZE: u32 = 100;

    pub fn validate(&self) -> AppResult<()> {
        if self.size() > Self::MAX_SIZE {
            return Err(AppError::bad_request(format!(
                "size must be between 1 and {}",
                Self::MAX_SIZE
            )));
        }
        Ok(())
    }

    pub fn page(&self) -> u32 {
        if self.page == 0 {
            Self::DEFAULT_PAGE
        } else {
            self.page
        }
    }

    pub fn size(&self) -> u32 {
        if self.size == 0 {
            Self::DEFAULT_SIZE
        } else {
            self.size
        }
    }

    /// SQL `LIMIT`.
    pub fn limit(&self) -> i64 {
        i64::from(self.size())
    }

    /// SQL `OFFSET`.
    pub fn offset(&self) -> i64 {
        i64::from(self.page() - 1) * i64::from(self.size())
    }
}

/// Response of every list API. `total` is the row count before paging.
#[derive(Debug, Serialize)]
pub struct PageData<T> {
    pub items: Vec<T>,
    pub total: i64,
}

impl<T> PageData<T> {
    pub fn new(items: Vec<T>, total: i64) -> Self {
        Self { items, total }
    }
}
