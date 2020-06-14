use crate::db::{UserRepository, UNIQUE_VIOLATION_CODE};
use crate::errors::AppError;
use crate::{
    config::CryptoService,
    models::{Auth, NewUser, UpdateProfile, User},
};
use actix_web::{web, FromRequest, HttpResponse};
use actix_web_httpauth::extractors::{basic::BasicAuth, bearer::BearerAuth};
use eyre::Result;
use futures::future::{ready, BoxFuture};
use sqlx::error::DatabaseError;
use tracing::{debug, instrument};
use uuid::Uuid;
use validator::Validate;
use web::{Data, Json};

type AppResult<T> = Result<T, AppError>;
type AppResponse = AppResult<HttpResponse>;

pub fn app_config(config: &mut web::ServiceConfig) {
    let signup = web::resource("/signup")
        .route(web::post().to(create_user));

    let auth = web::resource("/auth")
        .route(web::post().to(auth));

    let me = web::resource("/me")
        .route(web::get().to(me))
        .route(web::post().to(update_profile));

    config
        .service(signup)
        .service(auth)
        .service(me);
}

#[instrument(skip(user, repository, crypto_service))]
async fn create_user(
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
            let pg_error = error
                .root_cause()
                .downcast_ref::<sqlx::postgres::PgError>()
                .ok_or_else(|| {
                    debug!("Error creating user. {:?}", error);
                    AppError::INTERNAL_ERROR.default()
                })?;

            let error = match (pg_error.code(), pg_error.column_name()) {
                (Some(UNIQUE_VIOLATION_CODE), Some("email")) => {
                    AppError::INVALID_INPUT.message("Email address already exists.".to_string())
                }

                (Some(UNIQUE_VIOLATION_CODE), Some("username")) => {
                    AppError::INVALID_INPUT.message("Username already exists.".to_string())
                }

                _ => {
                    debug!("Error creating user. {:?}", pg_error);
                    AppError::INTERNAL_ERROR.default()
                }
            };
            Err(error)
        }
    }
}

#[instrument(skip(basic, repository, hashing))]
async fn auth(
    basic: BasicAuth,
    repository: UserRepository,
    hashing: Data<CryptoService>,
) -> AppResponse {
    let username = basic.user_id();
    let password = basic
        .password()
        .ok_or_else(|| AppError::INVALID_CREDENTIALS.default())?;

    let user = repository
        .find_by_username(username)
        .await?
        .ok_or_else(|| AppError::INVALID_CREDENTIALS.default())?;

    let valid = hashing
        .verify_password(password, &user.password_hash)
        .await?;

    if valid {
        let token = hashing.generate_jwt(user.id).await?;
        Ok(HttpResponse::Ok().json(Auth { token }))
    } else {
        Err(AppError::INVALID_CREDENTIALS.default())
    }
}

#[derive(Debug)]
pub struct AuthenticatedUser(Uuid);

impl FromRequest for AuthenticatedUser {
    type Error = AppError;
    type Future = BoxFuture<'static, Result<Self, Self::Error>>;
    type Config = ();
    #[instrument(skip(req, payload))]
    fn from_request(
        req: &actix_web::HttpRequest,
        payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let bearer_result = BearerAuth::from_request(req, payload).into_inner();
        let repository_result = UserRepository::from_request(req, payload).into_inner();
        let crypto_service_result = Data::<CryptoService>::from_request(req, payload).into_inner();

        match (bearer_result, repository_result, crypto_service_result) {
            (Ok(bearer), Ok(repository), Ok(crypto_service)) => {
                let future = async { validate_jwt(bearer, repository, crypto_service).await };
                Box::pin(future)
            }
            _ => {
                let error = ready(Err(AppError::NOT_AUTHORIZED.default()));
                Box::pin(error)
            }
        }
    }
}

#[instrument(skip(bearer, repository, crypto_service))]
pub async fn validate_jwt(
    bearer: BearerAuth,
    repository: UserRepository,
    crypto_service: Data<CryptoService>,
) -> AppResult<AuthenticatedUser> {
    let user_id = crypto_service
        .verify_jwt(bearer.token().to_string())
        .await
        .map(|data| data.claims.sub)
        .map_err(|err| {
            debug!("Cannot verify jwt. {:?}", err);
            AppError::NOT_AUTHORIZED.default()
        })?;

    repository.find_by_id(user_id).await?.ok_or_else(|| {
        debug!("User {} not found", user_id);
        AppError::NOT_AUTHORIZED.default()
    })?;

    Ok(AuthenticatedUser(user_id))
}

#[instrument[skip(repository)]]
async fn me(user: AuthenticatedUser, repository: UserRepository) -> AppResponse {
    let user = repository
        .find_by_id(user.0)
        .await?
        .ok_or(AppError::INTERNAL_ERROR.default())?;

    Ok(HttpResponse::Ok().json(user))
}

#[instrument(skip(repository))]
async fn update_profile(
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
