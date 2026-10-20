use crate::models::{PageData, PageRequest};
use crate::utils::TimeUtils;
use deadpool_postgres::{Pool, Transaction};

use crate::enums::{Level, Status};
use crate::error::{AppError, AppResult};
use crate::models::{CreateDefinitionRequest, DefinitionData, PutDefinitionRequest};

#[derive(Clone)]
pub struct DefinitionRepository {
    pool: Pool,
}

impl DefinitionRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn find_by_code(&self, code: &str) -> AppResult<DefinitionData> {
        let client = self.pool.get().await?;
        let sql = r#"
            SELECT d.id, d.code, d.name, d.status, d.description, d.level,
                   d.source_id, d.channel_id, d.created_at, d.updated_at,
                   (
                       SELECT COALESCE(
                           jsonb_agg(
                               jsonb_build_object(
                                   'target_id', t.id,
                                   'code', t.code,
                                   'template_id', dt.template_id,
                                   'template_code', tp.code
                               )
                               ORDER BY dt.id
                           ),
                           '[]'::jsonb
                       )
                       FROM definition_targets dt
                       JOIN targets t ON t.id = dt.target_id
                       LEFT JOIN templates tp ON tp.id = dt.template_id
                       WHERE dt.definition_id = d.id
                   ) AS targets
            FROM definitions d
            WHERE d.code = $1
            "#;
        match client.query_opt(sql, &[&code]).await? {
            Some(row) => DefinitionData::map_row(&row),
            None => Err(AppError::not_found(format!("code {code} not found"))),
        }
    }

    pub async fn find_by_id(&self, id: i32) -> AppResult<DefinitionData> {
        let client = self.pool.get().await?;
        let sql = r#"
            SELECT d.id, d.code, d.name, d.status, d.description, d.level,
                   d.source_id, d.channel_id, d.created_at, d.updated_at,
                   (
                       SELECT COALESCE(
                           jsonb_agg(
                               jsonb_build_object(
                                   'target_id', t.id,
                                   'code', t.code,
                                   'template_id', dt.template_id,
                                   'template_code', tp.code
                               )
                               ORDER BY dt.id
                           ),
                           '[]'::jsonb
                       )
                       FROM definition_targets dt
                       JOIN targets t ON t.id = dt.target_id
                       LEFT JOIN templates tp ON tp.id = dt.template_id
                       WHERE dt.definition_id = d.id
                   ) AS targets
            FROM definitions d
            WHERE d.id = $1
            "#;
        match client.query_opt(sql, &[&id]).await? {
            Some(row) => DefinitionData::map_row(&row),
            None => Err(AppError::not_found(format!("id {id} not found"))),
        }
    }

    pub async fn list_page(&self, page: &PageRequest) -> AppResult<PageData<DefinitionData>> {
        let client = self.pool.get().await?;
        let total: i64 = client
            .query_one(
                r#"
                SELECT COUNT(*)
                FROM definitions
                "#,
                &[],
            )
            .await?
            .get(0);
        let rows = client
            .query(
                r#"
                SELECT d.id, d.code, d.name, d.status, d.description, d.level,
                       d.source_id, d.channel_id, d.created_at, d.updated_at,
                       (
                           SELECT COALESCE(
                               jsonb_agg(
                                   jsonb_build_object(
                                   'target_id', t.id,
                                   'code', t.code,
                                   'template_id', dt.template_id,
                                   'template_code', tp.code
                               )
                                   ORDER BY dt.id
                               ),
                               '[]'::jsonb
                           )
                           FROM definition_targets dt
                           JOIN targets t ON t.id = dt.target_id
                           LEFT JOIN templates tp ON tp.id = dt.template_id
                           WHERE dt.definition_id = d.id
                       ) AS targets
                FROM definitions d
                ORDER BY d.updated_at, d.created_at
                LIMIT $1
                OFFSET $2
                "#,
                &[&page.limit(), &page.offset()],
            )
            .await?;
        let items = rows
            .iter()
            .map(DefinitionData::map_row)
            .collect::<AppResult<Vec<_>>>()?;
        Ok(PageData::new(items, total))
    }

    pub async fn insert(
        &self,
        req: &CreateDefinitionRequest,
        source_id: i32,
        channel_id: i32,
        target_ids: &[i32],
        template_ids: &[Option<i32>],
    ) -> AppResult<i32> {
        let mut client = self.pool.get().await?;
        let tx = client.transaction().await?;

        let row = tx
            .query_one(
                r#"
                INSERT INTO definitions (code, name, description, status, level, source_id, channel_id)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                RETURNING id
                "#,
                &[
                    &req.code,
                    &req.name,
                    &req.description,
                    &req.status.as_str(),
                    &req.level.as_str(),
                    &source_id,
                    &channel_id,
                ],
            )
            .await?;
        let id: i32 = row.get("id");

        Self::insert_targets(&tx, id, target_ids, template_ids).await?;
        tx.commit().await?;
        Ok(id)
    }

    /// Replaces the definition's fields and its linked targets. An omitted
    /// `status` / `level` keeps the stored value.
    pub async fn update(
        &self,
        code: &str,
        req: &PutDefinitionRequest,
        source_id: i32,
        channel_id: i32,
        target_ids: &[i32],
        template_ids: &[Option<i32>],
    ) -> AppResult<i32> {
        let mut client = self.pool.get().await?;
        let tx = client.transaction().await?;

        let row = tx
            .query_opt(
                r#"
                UPDATE definitions
                SET name = $2,
                    description = $3,
                    status = COALESCE($4, status),
                    level = COALESCE($5, level),
                    source_id = $6,
                    channel_id = $7,
                    updated_at = $8
                WHERE code = $1
                RETURNING id
                "#,
                &[
                    &code,
                    &req.name,
                    &req.description,
                    &req.status.map(Status::as_str),
                    &req.level.map(Level::as_str),
                    &source_id,
                    &channel_id,
                    &TimeUtils::now(),
                ],
            )
            .await?;
        let row = row.ok_or_else(|| AppError::not_found(format!("code {code} not found")))?;
        let id: i32 = row.get("id");

        tx.execute(
            r#"
            DELETE FROM definition_targets
            WHERE definition_id = $1
            "#,
            &[&id],
        )
        .await?;
        Self::insert_targets(&tx, id, target_ids, template_ids).await?;
        tx.commit().await?;
        Ok(id)
    }

    /// `definition_targets` rows go with it (`ON DELETE CASCADE`).
    pub async fn delete(&self, code: &str) -> AppResult<()> {
        let client = self.pool.get().await?;
        let n = client
            .execute(
                r#"
                DELETE FROM definitions
                WHERE code = $1
                "#,
                &[&code],
            )
            .await?;

        if n == 0 {
            return Err(AppError::not_found(format!("code {code} not found")));
        }
        Ok(())
    }

    /// Inserts `target_ids[i]` with `template_ids[i]`, in slice order:
    /// `definition_targets.id` order decides which template EMAIL / MSTEAMS render.
    async fn insert_targets(
        tx: &Transaction<'_>,
        definition_id: i32,
        target_ids: &[i32],
        template_ids: &[Option<i32>],
    ) -> AppResult<()> {
        if target_ids.len() != template_ids.len() {
            return Err(AppError::internal(
                "definition target_ids and template_ids differ in length",
            ));
        }
        tx.execute(
            r#"
            INSERT INTO definition_targets (
                definition_id, 
                target_id, 
                template_id
            )
            SELECT 
                $1, 
                t.target_id, 
                t.template_id
            FROM unnest(
                $2::int[], 
                $3::int[]
            ) AS t (target_id, template_id)
            "#,
            &[&definition_id, &target_ids, &template_ids],
        )
        .await?;
        Ok(())
    }
}
