use crate::models::{PageData, PageRequest};
use crate::utils::TimeUtils;
use deadpool_postgres::Pool;

use crate::{
    enums::Status,
    models::{CreateTargetRequest, PutTargetRequest, TargetData},
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
                SELECT t.id, t.code, c.code AS channel, t.name, t.status,
                       t.created_at, t.updated_at
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
                SELECT t.id, t.code, c.code AS channel, t.name, t.status,
                       t.created_at, t.updated_at
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
                SELECT t.id, t.code, c.code AS channel, t.name, t.status,
                       t.created_at, t.updated_at
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

    pub async fn insert(&self, req: &CreateTargetRequest, channel_id: i32) -> AppResult<i32> {
        let client = self.pool.get().await?;
        let row = client
            .query_one(
                r#"
                INSERT INTO targets (code, channel_id, name, status, updated_at)
                VALUES ($1, $2, $3, $4, $5)
                RETURNING id
                "#,
                &[
                    &req.code,
                    &channel_id,
                    &req.name,
                    &Status::Active.as_str(),
                    &TimeUtils::now(),
                ],
            )
            .await?;
        Ok(row.get("id"))
    }

    /// The channel never changes; `name` replaces the stored value.
    pub async fn update(&self, code: &str, req: &PutTargetRequest) -> AppResult<()> {
        let client = self.pool.get().await?;
        let n = client
            .execute(
                r#"
                UPDATE targets
                SET name = $2,
                    updated_at = $3
                WHERE code = $1
                "#,
                &[&code, &req.name, &TimeUtils::now()],
            )
            .await?;

        if n == 0 {
            return Err(AppError::not_found(format!("target `{code}` not found")));
        }
        Ok(())
    }
}
