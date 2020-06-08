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

        let password_hash = hashing.hash(new_user.password.clone()).await?;

        let user = sqlx::query_as::<_, User>("insert into users (email, password_hash) values ($1, $2) returning *")
             .bind(new_user.email.clone())
             .bind(password_hash)
             .fetch_one(&*self.pool)
             .await?;
        Ok(user)
    }
}