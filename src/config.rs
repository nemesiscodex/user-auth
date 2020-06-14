use actix_web::web::block;
use argonautica::{Hasher, Verifier};
use chrono::{Duration, Utc};
use dotenv::dotenv;
use eyre::{eyre, Result, WrapErr};
use futures::compat::Future01CompatExt;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, TokenData, Validation};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPool;
use sqlx::types::Uuid;
use std::sync::Arc;
use tracing::{info, instrument};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub host: String,
    pub port: i32,
    pub database_url: String,
    pub secret_key: String,
    pub jwt_secret: String,
}

#[derive(Debug, Clone)]
pub struct CryptoService {
    key: Arc<String>,
    jwt_secret: Arc<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    exp: i64,
}

impl CryptoService {
    #[instrument(skip(self, password))]
    pub async fn hash_password(&self, password: String) -> Result<String> {
        Hasher::default()
            .with_secret_key(&*self.key)
            .with_password(password)
            .hash_non_blocking()
            .compat()
            .await
            .map_err(|err| eyre!("Hashing error: {}", err))
    }

    #[instrument(skip(self, password, password_hash))]
    pub async fn verify_password(&self, password: &str, password_hash: &str) -> Result<bool> {
        Verifier::default()
            .with_secret_key(&*self.key)
            .with_hash(password_hash)
            .with_password(password)
            .verify_non_blocking()
            .compat()
            .await
            .map_err(|err| eyre!("Verifying error: {}", err))
    }

    #[instrument(skip(self))]
    pub async fn generate_jwt(&self, user_id: Uuid) -> Result<String> {
        let jwt_key = self.jwt_secret.clone();
        block(move || {
            let headers = Header::default();
            let encoding_key = EncodingKey::from_secret(jwt_key.as_bytes());
            let now = Utc::now() + Duration::days(1); // Expires in 1 day
            let claims = Claims {
                sub: user_id,
                exp: now.timestamp(),
            };
            encode(&headers, &claims, &encoding_key)
        })
        .await
        .map_err(|err| eyre!("Creating jwt token: {}", err))
    }

    #[instrument(skip(self, token))]
    pub async fn verify_jwt(&self, token: String) -> Result<TokenData<Claims>> {
        let jwt_key = self.jwt_secret.clone();
        block(move || {
            let decoding_key = DecodingKey::from_secret(jwt_key.as_bytes());
            let validation = Validation::default();
            decode::<Claims>(&token.clone(), &decoding_key, &validation)
        })
        .await
        .map_err(|err| eyre!("Verifying jwt token: {}", err))
    }
}

impl Config {
    #[instrument]
    pub fn from_env() -> Result<Config> {
        dotenv().ok();

        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .init();

        info!("Loading configuration");

        let mut c = config::Config::new();

        c.merge(config::Environment::default())?;

        c.try_into()
            .context("loading configuration from environment")
    }

    #[instrument(skip(self))]
    pub async fn db_pool(&self) -> Result<PgPool> {
        info!("Creating database connection pool.");
        PgPool::builder()
            .connect_timeout(std::time::Duration::from_secs(30))
            .build(&self.database_url)
            .await
            .context("creating database connection pool")
    }

    #[instrument(skip(self))]
    pub fn hashing(&self) -> CryptoService {
        CryptoService {
            key: Arc::new(self.secret_key.clone()),
            jwt_secret: Arc::new(self.jwt_secret.clone()),
        }
    }
}
