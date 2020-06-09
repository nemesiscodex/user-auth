use actix_web::{web, HttpResponse};
use actix_web_httpauth::extractors::basic::BasicAuth;
use tracing::{error, instrument, debug};
use web::{Json, Data};
use sqlx::{PgPool, error::DatabaseError};
use std::ops::Deref;
use eyre::Result;
use crate::db::UserRepository;
use crate::{models::{User, NewUser, Auth}, config::HashingService};
use validator::{Validate, ValidationErrors};

pub fn app_config(config: &mut web::ServiceConfig) {
    let signup = web::resource("/signup")
        .route(web::post().to(create_user));

    let auth = web::resource("/auth")
        .route(web::post().to(auth));

    config
        .service(signup)
        .service(auth);
}

// TODO: Better error response
#[instrument(skip(user, pool, hashing))]
async fn create_user(user: Json<NewUser>, pool: Data<PgPool>, hashing: Data<HashingService>) -> HttpResponse {

    let result: Result<(), ValidationErrors> = user.validate();

    match result {
        Ok(_) => (),
        Err(errors) => {
            let error_map = errors.field_errors();

            if error_map.contains_key("username") {
                debug!("Invalid username. \"{}\" is too short.", user.username);
            }

            if error_map.contains_key("email") {
                debug!("Invalid email address \"{}\"", user.email);
            }

            if error_map.contains_key("password") {
                debug!("Invalid password. Too short");
            }
            return HttpResponse::BadRequest().finish();
        }
    }

    let repository = UserRepository::new(pool.deref().clone());

    let result: Result<User> = repository.create_user(user.0, hashing.as_ref()).await;

    match result {
        Ok(user) => HttpResponse::Ok().json(user),
        Err(error) => match error.root_cause().downcast_ref::<sqlx::postgres::PgError>() {
            Some(pg_error) if pg_error.code() == Some("23505") => {
                debug!("Email address already exists. {:?}", error);
                HttpResponse::BadRequest().finish()
            },
            _ => {
                error!("An error ocurred creating the user. {:?}", error);
                HttpResponse::InternalServerError().finish()
            }
        }
    }

}

// TODO: Better error handling. 
// TODO: Remove nested match
// TODO: Implement jwt token
#[instrument(skip(basic, pool, hashing))]
async fn auth(basic: BasicAuth, pool: Data<PgPool>, hashing: Data<HashingService>) -> HttpResponse {
    let username = basic.user_id();
    let maybe_password = basic.password();

    match maybe_password {
        Some(password) => {
            let repository = UserRepository::new(pool.deref().clone());

            let result: Result<Option<User>> = repository.find_by_username(username).await;

            match result {
                Ok(Some(user)) => {
                    let verify: Result<bool> = hashing.verify(password, &user.password_hash).await;

                    match verify {
                        Ok(true) => {
                            debug!("Correct username and password. Creating token.");
                            HttpResponse::Ok().json(Auth { token: String::from("mytoken") })
                        },
                        Ok(false) => {
                            debug!("Invalid password for user {}.", username);
                            HttpResponse::Unauthorized().finish()
                        },
                        Err(err) => {
                            debug!("Unexpected error occured. {:?}", err);
                            HttpResponse::InternalServerError().finish()
                        }
                    }

                },
                Ok(None) => {
                    debug!("User with username \"{}\" not found.", username);
                    HttpResponse::Unauthorized().finish()
                },
                Err(err) => {
                    debug!("Unexpected error occured. {:?}", err);
                    HttpResponse::InternalServerError().finish()
                }
            }
        },
        None => {
            debug!("Must provide a password.");
            HttpResponse::Unauthorized().finish()
        }
    }

}