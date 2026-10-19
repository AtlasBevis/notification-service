use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct PageData<T> {
    pub items: Vec<T>,
    pub total: i64,
}

impl<T> PageData<T> {
    pub fn new(items: Vec<T>) -> Self {
        let total = items.len() as i64;
        Self { items, total }
    }
}
