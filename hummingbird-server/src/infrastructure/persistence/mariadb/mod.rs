mod library;
mod playlist;
mod scanner;
mod user;

use sqlx::mysql::MySqlPool;

use crate::errors::AppError;

type Result<T> = std::result::Result<T, AppError>;

pub struct MariaDbDatabase {
    pub(crate) pool: MySqlPool,
}

impl MariaDbDatabase {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub async fn run_migrations(&self) -> anyhow::Result<()> {
        let sql = include_str!("../../../../migrations/mariadb/001_initial_schema.sql");
        for stmt in sql.split(';') {
            let trimmed = stmt.trim();
            if !trimmed.is_empty() && !trimmed.starts_with("--") {
                sqlx::raw_sql(&format!("{trimmed};"))
                    .execute(&self.pool)
                    .await?;
            }
        }

        let sql2 = include_str!("../../../../migrations/mariadb/002_add_users.sql");
        for stmt in sql2.split(';') {
            let trimmed = stmt.trim();
            if !trimmed.is_empty() && !trimmed.starts_with("--") {
                sqlx::raw_sql(&format!("{trimmed};"))
                    .execute(&self.pool)
                    .await?;
            }
        }

        Ok(())
    }

    async fn cleanup_orphans_after_track_delete(
        &self,
        album_id: Option<i64>,
        folder: Option<&str>,
        disc_number: Option<i32>,
    ) -> Result<()> {
        if let (Some(aid), Some(f)) = (album_id, folder) {
            let disc = disc_number.unwrap_or(-1);
            sqlx::query(
                "DELETE FROM album_path \
                 WHERE path = ? AND disc_num = ? AND album_id = ? \
                 AND NOT EXISTS (SELECT 1 FROM track WHERE folder = ? AND disc_number <=> ? AND album_id = ?)",
            )
            .bind(f)
            .bind(disc)
            .bind(aid)
            .bind(f)
            .bind(disc_number)
            .bind(aid)
            .execute(&self.pool)
            .await?;
        }

        if let Some(aid) = album_id {
            let deleted = sqlx::query(
                "DELETE FROM album WHERE id = ? AND NOT EXISTS (SELECT 1 FROM track WHERE album_id = ?)",
            )
            .bind(aid)
            .bind(aid)
            .execute(&self.pool)
            .await?;

            if deleted.rows_affected() > 0 {
                sqlx::query("DELETE FROM album_path WHERE album_id = ?")
                    .bind(aid)
                    .execute(&self.pool)
                    .await
                    .ok();
            }
        }

        sqlx::query("DELETE FROM artist WHERE NOT EXISTS (SELECT 1 FROM album WHERE artist_id = artist.id)")
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
