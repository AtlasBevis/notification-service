use serde::Serialize;
use utoipa::ToSchema;

/// Paginated payload: `Data` is `{ list, total }`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PageData<T> {
    pub list: Vec<T>,
    pub total: i64,
}

impl<T> PageData<T> {
    pub fn from_items(list: Vec<T>) -> Self {
        let total = list.len() as i64;
        Self { list, total }
    }
}
