use sqlx::PgPool;
use std::sync::Arc;
use eyre::Result;
use crate::{config::HashingService, models::{User, NewUser}};
use sqlx::postgres::PgQueryAs;
use tracing::instrument;

pub struct UserRepository {
    pool: Arc<PgPool>
}

impl UserRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    #[instrument(skip(self, new_user, hashing))]
    pub async fn create_user(&self, new_user: NewUser, hashing: &HashingService) -> Result<User> {
        let password_hash = hashing.hash(new_user.password).await?;

        let user = sqlx::query_as::<_, User>("insert into users (username, email, password_hash) values ($1, $2, $3) returning *")
            .bind(new_user.username)
            .bind(new_user.email)
            .bind(password_hash)
            .fetch_one(&*self.pool)
            .await?;
        Ok(user)
    }

    #[instrument(skip(self))]
    pub async fn find_by_username(&self, username: &str) -> Result<Option<User>> {
        let maybe_user = 
            sqlx::query_as::<_, User>("select * from users where email = $1 or username = $1")
            .bind(username)
            .fetch_optional(&*self.pool)
            .await?;
        
        Ok(maybe_user)
    }
}