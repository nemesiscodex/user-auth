use config;
use dotenv::dotenv;
use serde::Deserialize;
use eyre::{eyre, WrapErr, Result};
use tracing::{info, instrument, Level};
use sqlx::postgres::PgPool;
use std::sync::Arc;
use argonautica::{Verifier, Hasher};
use futures::compat::Future01CompatExt;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub host: String,
    pub port: i32,
    pub database_url: String,
    pub secret_key: String,
    pub jwt_secret: String
}

#[derive(Debug, Clone)]
pub struct HashingService {
    key: Arc<String>
}

impl HashingService {
    #[instrument(skip(self, password))]
    pub async fn hash(&self, password: String) -> Result<String> {
        Hasher::default()
            .with_secret_key(&*self.key)
            .with_password(password)
            .hash_non_blocking()
            .compat()
            .await
            .map_err(|err| eyre!("Hashing error: {}", err))
    }

    #[instrument(skip(self, password, password_hash))]
    pub async fn verify(&self, password: &str, password_hash: &str) -> Result<bool> {
        Verifier::default()
            .with_secret_key(&*self.key)
            .with_hash(password_hash)
            .with_password(password)
            .verify_non_blocking()
            .compat()
            .await
            .map_err(|err| eyre!("Verifying error: {}", err))
    }
}

impl Config {
    
    #[instrument]
    pub fn from_env() -> Result<Config> {
        dotenv().ok();
        
        tracing_subscriber::fmt()
            .with_max_level(Level::DEBUG)
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
            .build(&self.database_url)
            .await
            .context("creating database connection pool")
    }

    #[instrument(skip(self))]
    pub fn hashing(&self) -> HashingService {
        HashingService { key: Arc::new(self.secret_key.clone()) }
    }
}