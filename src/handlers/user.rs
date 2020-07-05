use super::{auth::AuthenticatedUser, AppResponse};
use crate::{
    db, 
    config::crypto::CryptoService,
    db::user::UserRepository,
    errors::AppError,
    models::user::{NewUser, UpdateProfile, User},
};
use actix_web::{
    web::{Data, Json},
    HttpResponse,
};
use color_eyre::Result;
use sqlx::{error::DatabaseError, postgres::PgError};
use tracing::{debug, instrument};
use validator::Validate;

#[instrument(skip(user, repository, crypto_service))]
pub async fn create_user(
    user: Json<NewUser>,
    repository: UserRepository,
    crypto_service: Data<CryptoService>,
) -> AppResponse {
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

    let result: Result<User> = repository.create(user.0, crypto_service.as_ref()).await;

    match result {
        Ok(user) => Ok(HttpResponse::Ok().json(user)),
        Err(error) => {
            let pg_error: &PgError =
                error
                    .root_cause()
                    .downcast_ref::<PgError>()
                    .ok_or_else(|| {
                        debug!("Error creating user. {:?}", error);
                        AppError::INTERNAL_ERROR
                    })?;

            let error = match (pg_error.code(), pg_error.column_name()) {
                (Some(db::UNIQUE_VIOLATION_CODE), Some("email")) => {
                    AppError::INVALID_INPUT.message("Email address already exists.".to_string())
                },
                (Some(db::UNIQUE_VIOLATION_CODE), Some("username")) => {
                    AppError::INVALID_INPUT.message("Username already exists.".to_string())
                },
                (Some(db::UNIQUE_VIOLATION_CODE), None) => {
                    AppError::INVALID_INPUT.message("Username or email already exists.".to_string())
                },
                _ => {
                    debug!("Error creating user. {:?}", pg_error);
                    AppError::INTERNAL_ERROR.into()
                }
            };
            Err(error)
        }
    }
}

#[instrument(skip(repository))]
pub async fn update_profile(
    user: AuthenticatedUser,
    repository: UserRepository,
    profile: Json<UpdateProfile>,
) -> AppResponse {
    match profile.validate() {
        Ok(_) => Ok(()),
        Err(errors) => {
            let error_map = errors.field_errors();

            let message = if error_map.contains_key("image") {
                format!(
                    "Invalid image. \"{}\" is not a valid url.",
                    profile.image.as_deref().unwrap()
                )
            } else {
                "Invalid input.".to_string()
            };

            Err(AppError::INVALID_INPUT.message(message))
        }
    }?;

    let user = repository.update_profile(user.0, profile.0).await?;

    Ok(HttpResponse::Ok().json(user))
}

#[instrument[skip(repository)]]
pub async fn me(user: AuthenticatedUser, repository: UserRepository) -> AppResponse {
    let user = repository
        .find_by_id(user.0)
        .await?
        .ok_or(AppError::INTERNAL_ERROR)?;

    Ok(HttpResponse::Ok().json(user))
}
