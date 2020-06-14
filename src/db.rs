use crate::{
    config::CryptoService,
    errors::AppError,
    models::{NewUser, UpdateProfile, User},
};
use actix_web::{web::Data, FromRequest};
use eyre::Result;
use futures::future::{ready, Ready};
use sqlx::postgres::PgQueryAs;
use sqlx::PgPool;
use std::{ops::Deref, sync::Arc};
use tracing::instrument;
use uuid::Uuid;

pub const UNIQUE_VIOLATION_CODE: &str = "23505";

pub struct UserRepository {
    pool: Arc<PgPool>,
}

impl UserRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    #[instrument(skip(self, new_user, hashing))]
    pub async fn create(&self, new_user: NewUser, hashing: &CryptoService) -> Result<User> {
        let password_hash = hashing.hash_password(new_user.password).await?;

        let user = sqlx::query_as::<_, User>(
            "insert into users (username, email, password_hash) values ($1, $2, $3) returning *",
        )
        .bind(new_user.username)
        .bind(new_user.email)
        .bind(password_hash)
        .fetch_one(&*self.pool)
        .await?;
        Ok(user)
    }

    pub async fn update_profile(&self, user_id: Uuid, profile: UpdateProfile) -> Result<User> {
        let user = sqlx::query_as::<_, User>(
            "update users set full_name = $2, bio = $3, image = $4 where id = $1 returning *",
        )
        .bind(user_id)
        .bind(profile.full_name)
        .bind(profile.bio)
        .bind(profile.image)
        .fetch_one(&*self.pool)
        .await?;
        Ok(user)
    }

    #[instrument(skip(self))]
    pub async fn find_by_username(&self, username: &str) -> Result<Option<User>> {
        let maybe_user = sqlx::query_as::<_, User>("select * from users where username = $1")
            .bind(username)
            .fetch_optional(&*self.pool)
            .await?;

        Ok(maybe_user)
    }

    #[instrument(skip(self))]
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<User>> {
        let maybe_user = sqlx::query_as::<_, User>("select * from users where id = $1")
            .bind(id)
            .fetch_optional(&*self.pool)
            .await?;

        Ok(maybe_user)
    }
}

impl FromRequest for UserRepository {
    type Error = AppError;
    type Future = Ready<Result<Self, Self::Error>>;
    type Config = ();
    #[instrument(skip(req, payload))]
    fn from_request(
        req: &actix_web::HttpRequest,
        payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let pool_result = Data::<PgPool>::from_request(req, payload).into_inner();

        match pool_result {
            Ok(pool) => ready(Ok(UserRepository::new(pool.deref().clone()))),
            _ => ready(Err(AppError::NOT_AUTHORIZED.default())),
        }
    }
}
