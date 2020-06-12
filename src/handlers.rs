use crate::db::{UserRepository, UNIQUE_VIOLATION_CODE};
use crate::errors::AppError;
use crate::{
    config::CryptoService,
    models::{Auth, NewUser, User},
};
use actix_web::{web, HttpResponse};
use actix_web_httpauth::extractors::basic::BasicAuth;
use eyre::Result;
use sqlx::{error::DatabaseError, PgPool};
use std::ops::Deref;
use tracing::instrument;
use validator::Validate;
use web::{Data, Json};

pub fn app_config(config: &mut web::ServiceConfig) {
    let signup = web::resource("/signup").route(web::post().to(create_user));

    let auth = web::resource("/auth").route(web::post().to(auth));

    config.service(signup).service(auth);
}

#[instrument(skip(user, pool, hashing))]
async fn create_user(
    user: Json<NewUser>,
    pool: Data<PgPool>,
    hashing: Data<CryptoService>,
) -> Result<HttpResponse, AppError> {
    match user.validate() {
        Ok(_) => Ok(()),
        Err(errors) => {
            let error_map = errors.field_errors();

            let message = if error_map.contains_key("username") {
                format!("Invalid username. \"{}\" is too short.", user.username)
            } else if error_map.contains_key("email") {
                format!("Invalid email address \"{}\"", user.email)
            } else if error_map.contains_key("password") {
                "Invalid password. Too short".to_string()
            } else {
                "Invalid input.".to_string()
            };

            Err(AppError::INVALID_INPUT.message(message))
        }
    }?;

    let repository = UserRepository::new(pool.deref().clone());

    let result: Result<User> = repository.create_user(user.0, hashing.as_ref()).await;

    match result {
        Ok(user) => Ok(HttpResponse::Ok().json(user)),
        Err(error) => {
            let pg_error = error
                .root_cause()
                .downcast_ref::<sqlx::postgres::PgError>()
                .ok_or_else(|| AppError::INTERNAL_ERROR.default())?;

            let error = match (pg_error.code(), pg_error.column_name()) {
                (Some(UNIQUE_VIOLATION_CODE), Some("email")) => {
                    AppError::INVALID_INPUT.message("Email address already exists.".to_string())
                }

                (Some(UNIQUE_VIOLATION_CODE), Some("username")) => {
                    AppError::INVALID_INPUT.message("Username already exists.".to_string())
                }

                _ => AppError::INTERNAL_ERROR.default(),
            };
            Err(error)
        }
    }
}

// TODO: Implement jwt token
#[instrument(skip(basic, pool, hashing))]
async fn auth(
    basic: BasicAuth,
    pool: Data<PgPool>,
    hashing: Data<CryptoService>,
) -> Result<HttpResponse, AppError> {
    let username = basic.user_id();
    let password = basic
        .password()
        .ok_or_else(|| AppError::INVALID_CREDENTIALS.default())?;

    let repository = UserRepository::new(pool.deref().clone());

    let user = repository
        .find_by_username(username)
        .await?
        .ok_or_else(|| AppError::INVALID_CREDENTIALS.default())?;

    let valid = hashing.verify_password(password, &user.password_hash).await?;

    if valid {
        let token = hashing
            .generate_jwt(user.id)
            .await?;
        Ok(HttpResponse::Ok().json(Auth { token }))
    } else {
        Err(AppError::INVALID_CREDENTIALS.default())
    }
}
