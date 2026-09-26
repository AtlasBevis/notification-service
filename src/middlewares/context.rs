use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: String,
}

impl RequestContext {
    pub fn from_headers(request_id: Option<&str>) -> Self {
        let request_id = request_id
            .map(str::to_owned)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        Self { request_id }
    }
}
