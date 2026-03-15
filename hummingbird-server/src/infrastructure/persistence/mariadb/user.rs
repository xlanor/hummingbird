use async_trait::async_trait;

use super::MariaDbDatabase;
use crate::domain::user::dao::UserDao;
use crate::domain::user::User;
use crate::errors::AppError;

type Result<T> = std::result::Result<T, AppError>;

#[async_trait]
impl UserDao for MariaDbDatabase {
    async fn create_user(
        &self,
        username: &str,
        display_name: Option<&str>,
        password_hash: Option<&str>,
        role: &str,
    ) -> Result<i64> {
        let result = sqlx::query(
            "INSERT INTO app_user (username, display_name, password_hash, role) VALUES (?, ?, ?, ?)",
        )
        .bind(username)
        .bind(display_name)
        .bind(password_hash)
        .bind(role)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_id() as i64)
    }

    async fn get_user_by_id(&self, id: i64) -> Result<User> {
        let user = sqlx::query_as::<_, User>(
            "SELECT id, username, display_name, password_hash, oidc_issuer, oidc_subject, role, created_at \
             FROM app_user WHERE id = ?",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok(user)
    }

    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            "SELECT id, username, display_name, password_hash, oidc_issuer, oidc_subject, role, created_at \
             FROM app_user WHERE username = ?",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;
        Ok(user)
    }

    async fn get_user_by_oidc(&self, issuer: &str, subject: &str) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            "SELECT id, username, display_name, password_hash, oidc_issuer, oidc_subject, role, created_at \
             FROM app_user WHERE oidc_issuer = ? AND oidc_subject = ?",
        )
        .bind(issuer)
        .bind(subject)
        .fetch_optional(&self.pool)
        .await?;
        Ok(user)
    }

    async fn create_or_get_oidc_user(
        &self,
        issuer: &str,
        subject: &str,
        username: &str,
        display_name: Option<&str>,
    ) -> Result<User> {
        if let Some(user) = self.get_user_by_oidc(issuer, subject).await? {
            return Ok(user);
        }

        let unique_username = match self.get_user_by_username(username).await? {
            None => username.to_string(),
            Some(_) => format!("{username}_{}", &subject[..8.min(subject.len())]),
        };

        sqlx::query(
            "INSERT INTO app_user (username, display_name, oidc_issuer, oidc_subject, role) \
             VALUES (?, ?, ?, ?, 'user')",
        )
        .bind(&unique_username)
        .bind(display_name)
        .bind(issuer)
        .bind(subject)
        .execute(&self.pool)
        .await?;

        self.get_user_by_oidc(issuer, subject)
            .await?
            .ok_or(AppError::Internal(anyhow::anyhow!("failed to create OIDC user")))
    }

    async fn list_users(&self) -> Result<Vec<User>> {
        let users = sqlx::query_as::<_, User>(
            "SELECT id, username, display_name, password_hash, oidc_issuer, oidc_subject, role, created_at \
             FROM app_user ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(users)
    }

    async fn delete_user(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM playlist_item WHERE playlist_id IN (SELECT id FROM playlist WHERE user_id = ?)")
            .bind(id)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM playlist WHERE user_id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM app_user WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn update_user_password(&self, id: i64, password_hash: &str) -> Result<()> {
        sqlx::query("UPDATE app_user SET password_hash = ? WHERE id = ?")
            .bind(password_hash)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn update_user_role(&self, id: i64, role: &str) -> Result<()> {
        sqlx::query("UPDATE app_user SET role = ? WHERE id = ?")
            .bind(role)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
