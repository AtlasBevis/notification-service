use crate::utils::TimeUtils;
use deadpool_postgres::{Pool, Transaction};

use crate::enums::{Level, Status};
use crate::error::{AppError, AppResult};
use crate::models::{DefinitionData, DefinitionTarget, SaveDefinitionRequest};

const COLUMNS: &str = r#"
    d.id, d.code, d.name, d.status, d.description, d.level,
    d.source_id, d.channel_id, d.created_at, d.updated_at,
    (
        SELECT COALESCE(
            jsonb_agg(
                jsonb_build_object('target_id', dt.target_id, 'template_id', dt.template_id)
                ORDER BY dt.id
            ),
            '[]'::jsonb
        )
        FROM definition_targets dt
        WHERE dt.definition_id = d.id
    ) AS targets
"#;

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
        let sql = format!(
            r#"
            SELECT {COLUMNS}
            FROM definitions d
            WHERE d.code = $1
            "#
        );
        match client.query_opt(sql.as_str(), &[&code]).await? {
            Some(row) => DefinitionData::map_row(&row),
            None => Err(AppError::not_found(format!("code {code} not found"))),
        }
    }

    pub async fn find_by_id(&self, id: i64) -> AppResult<DefinitionData> {
        let client = self.pool.get().await?;
        let sql = format!(
            r#"
            SELECT {COLUMNS}
            FROM definitions d
            WHERE d.id = $1
            "#
        );
        match client.query_opt(sql.as_str(), &[&id]).await? {
            Some(row) => DefinitionData::map_row(&row),
            None => Err(AppError::not_found(format!("id {id} not found"))),
        }
    }

    pub async fn list(&self) -> AppResult<Vec<DefinitionData>> {
        let client = self.pool.get().await?;
        let sql = format!(
            r#"
            SELECT {COLUMNS}
            FROM definitions d
            ORDER BY d.code
            "#
        );
        let rows = client.query(sql.as_str(), &[]).await?;
        rows.iter().map(DefinitionData::map_row).collect()
    }

    pub async fn insert(
        &self,
        req: &SaveDefinitionRequest,
        source_id: i32,
        channel_id: i32,
        targets: &[DefinitionTarget],
    ) -> AppResult<DefinitionData> {
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
                    &req.status.as_deref().unwrap_or(Status::Active.as_str()),
                    &req.level.as_deref().unwrap_or(Level::Info.as_str()),
                    &source_id,
                    &channel_id,
                ],
            )
            .await?;
        let id: i64 = row.get("id");

        insert_targets(&tx, id, targets).await?;
        let data = find_in_tx(&tx, id).await?;
        tx.commit().await?;
        Ok(data)
    }

    /// Replaces the definition's fields and its linked targets. An omitted
    /// `status` / `level` keeps the stored value.
    pub async fn update(
        &self,
        req: &SaveDefinitionRequest,
        source_id: i32,
        channel_id: i32,
        targets: &[DefinitionTarget],
    ) -> AppResult<DefinitionData> {
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
                    &req.code,
                    &req.name,
                    &req.description,
                    &req.status,
                    &req.level,
                    &source_id,
                    &channel_id,
                    &TimeUtils::now(),
                ],
            )
            .await?;
        let row = row.ok_or_else(|| AppError::not_found(format!("code {} not found", req.code)))?;
        let id: i64 = row.get("id");

        tx.execute(
            r#"
            DELETE FROM definition_targets
            WHERE definition_id = $1
            "#,
            &[&id],
        )
        .await?;
        insert_targets(&tx, id, targets).await?;
        let data = find_in_tx(&tx, id).await?;
        tx.commit().await?;
        Ok(data)
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
}

/// Inserts in slice order: `definition_targets.id` order decides which
/// template EMAIL / MSTEAMS render.
async fn insert_targets(
    tx: &Transaction<'_>,
    definition_id: i64,
    targets: &[DefinitionTarget],
) -> AppResult<()> {
    let target_ids: Vec<i32> = targets.iter().map(|t| t.target_id).collect();
    let template_ids: Vec<Option<i32>> = targets.iter().map(|t| t.template_id).collect();
    tx.execute(
        r#"
        INSERT INTO definition_targets (definition_id, target_id, template_id)
        SELECT $1, t.target_id, t.template_id
        FROM unnest($2::int[], $3::int[]) WITH ORDINALITY AS t (target_id, template_id, ord)
        ORDER BY t.ord
        "#,
        &[&definition_id, &target_ids, &template_ids],
    )
    .await?;
    Ok(())
}

async fn find_in_tx(tx: &Transaction<'_>, id: i64) -> AppResult<DefinitionData> {
    let sql = format!(
        r#"
        SELECT {COLUMNS}
        FROM definitions d
        WHERE d.id = $1
        "#
    );
    let row = tx.query_one(sql.as_str(), &[&id]).await?;
    DefinitionData::map_row(&row)
}
