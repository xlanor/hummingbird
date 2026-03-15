mod library;
mod playlist;
mod scanner;
mod user;

use sqlx::sqlite::SqlitePool;

pub struct SqliteDatabase {
    pub(crate) pool: SqlitePool,
}

impl SqliteDatabase {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn run_migrations(&self) -> anyhow::Result<()> {
        let sql = include_str!("../../../../migrations/sqlite/001_initial_schema.sql");
        sqlx::raw_sql(sql).execute(&self.pool).await?;
        let sql2 = include_str!("../../../../migrations/sqlite/002_add_users.sql");
        sqlx::raw_sql(sql2).execute(&self.pool).await?;
        Ok(())
    }
}
