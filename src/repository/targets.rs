use crate::models::{PageData, PageRequest};
use crate::utils::TimeUtils;
use deadpool_postgres::Pool;
use serde_json::Value;

use crate::{
    enums::Status,
    models::{TargetData, TargetRequest},
};

use crate::error::{AppError, AppResult};

#[derive(Clone)]
pub struct TargetsRepository {
    pool: Pool,
}

impl TargetsRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn list_page(&self, page: &PageRequest) -> AppResult<PageData<TargetData>> {
        let client = self.pool.get().await?;
        let total: i64 = client
            .query_one(
                r#"
                SELECT COUNT(*)
                FROM targets
                "#,
                &[],
            )
            .await?
            .get(0);
        let rows = client
            .query(
                r#"
                SELECT t.id, t.code, c.code AS channel_code, t.name, t.status,
                       t.metadata, t.created_at, t.updated_at
                FROM targets t
                JOIN channels c ON c.id = t.channel_id
                ORDER BY t.updated_at, t.created_at
                LIMIT $1
                OFFSET $2
                "#,
                &[&page.limit(), &page.offset()],
            )
            .await?;
        let items = rows
            .iter()
            .map(TargetData::map_row)
            .collect::<AppResult<Vec<_>>>()?;
        Ok(PageData::new(items, total))
    }

    pub async fn find_by_code(&self, code: &str) -> AppResult<TargetData> {
        let client = self.pool.get().await?;
        let row = client
            .query_opt(
                r#"
                SELECT t.id, t.code, c.code AS channel_code, t.name, t.status,
                       t.metadata, t.created_at, t.updated_at
                FROM targets t
                JOIN channels c ON c.id = t.channel_id
                WHERE t.code = $1
                "#,
                &[&code],
            )
            .await?;
        let row = row.ok_or_else(|| AppError::not_found(format!("unknown target `{code}`")))?;
        TargetData::map_row(&row)
    }

    pub async fn find_by_id(&self, id: i32) -> AppResult<TargetData> {
        let client = self.pool.get().await?;
        let row = client
            .query_opt(
                r#"
                SELECT t.id, t.code, c.code AS channel_code, t.name, t.status,
                       t.metadata, t.created_at, t.updated_at
                FROM targets t
                JOIN channels c ON c.id = t.channel_id
                WHERE t.id = $1
                "#,
                &[&id],
            )
            .await?;
        let row = row.ok_or_else(|| AppError::not_found(format!("unknown target id `{id}`")))?;
        TargetData::map_row(&row)
    }

    pub async fn insert(&self, req: &TargetRequest, channel_id: i32) -> AppResult<i32> {
        let client = self.pool.get().await?;
        let row = client
            .query_one(
                r#"
                INSERT INTO targets (code, channel_id, name, status, metadata)
                VALUES ($1, $2, $3, $4, $5)
                RETURNING id
                "#,
                &[
                    &req.code,
                    &channel_id,
                    &req.name,
                    &Status::Active.as_str(),
                    &Value::Object(req.metadata.clone()),
                ],
            )
            .await?;
        Ok(row.get("id"))
    }

    /// The channel never changes; `name` and `metadata` replace the stored values.
    pub async fn update(&self, req: &TargetRequest) -> AppResult<()> {
        let client = self.pool.get().await?;
        let n = client
            .execute(
                r#"
                UPDATE targets
                SET name = $2,
                    metadata = $3,
                    updated_at = $4
                WHERE code = $1
                "#,
                &[
                    &req.code,
                    &req.name,
                    &Value::Object(req.metadata.clone()),
                    &TimeUtils::now(),
                ],
            )
            .await?;

        if n == 0 {
            return Err(AppError::not_found(format!(
                "target `{}` not found",
                req.code
            )));
        }
        Ok(())
    }
}
