use async_trait::async_trait;

use super::User;
use crate::errors::AppError;

type Result<T> = std::result::Result<T, AppError>;

#[async_trait]
pub trait UserDao: Send + Sync {
    async fn create_user(
        &self,
        username: &str,
        display_name: Option<&str>,
        password_hash: Option<&str>,
        role: &str,
    ) -> Result<i64>;
    async fn get_user_by_id(&self, id: i64) -> Result<User>;
    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>>;
    async fn get_user_by_oidc(&self, issuer: &str, subject: &str) -> Result<Option<User>>;
    async fn create_or_get_oidc_user(
        &self,
        issuer: &str,
        subject: &str,
        username: &str,
        display_name: Option<&str>,
    ) -> Result<User>;
    async fn list_users(&self) -> Result<Vec<User>>;
    async fn delete_user(&self, id: i64) -> Result<()>;
    async fn update_user_password(&self, id: i64, password_hash: &str) -> Result<()>;
    async fn update_user_role(&self, id: i64, role: &str) -> Result<()>;
}
