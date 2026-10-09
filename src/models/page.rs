use serde::Serialize;

/// Paginated payload: `Data` is `{ list, total }`.
#[derive(Debug, Clone, Serialize)]
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
