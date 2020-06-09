#[macro_use]
extern crate validator_derive;

mod config;
mod handlers;
mod models;
mod db;

use crate::config::Config;
use tracing::{info, instrument};
use eyre::Result;
use actix_web::{App, HttpServer};
use handlers::app_config;


#[actix_rt::main]
#[instrument]
async fn main() -> Result<()> {

    let config = Config::from_env()
        .expect("Server configuration");

    let pool = config.db_pool().await
        .expect("Database configuration");

    let hashing = config.hashing();

    info!("Starting server at http://{}:{}/", config.host, config.port);

    HttpServer::new(move || {
        App::new()
            .data(pool.clone())
            .data(hashing.clone())
            .configure(app_config)
    })
        .bind(format!("{}:{}", config.host, config.port))?
        .run()
        .await?;

    Ok(())
}
