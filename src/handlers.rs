use actix_web::{web, HttpResponse};
use tracing::{error, instrument};
use web::{Json, Data};
use sqlx::{PgPool, error::DatabaseError};
use std::ops::Deref;
use eyre::Result;
use crate::db::UserRepository;
use crate::{models::{User, NewUser}, config::HashingService};

pub fn app_config(config: &mut web::ServiceConfig) {
    let signup = web::resource("/signup")
        .route(web::post().to(create_user));

    config
        .service(signup);
}

#[instrument(skip(user, pool, hashing))]
async fn create_user(user: Json<NewUser>, pool: Data<PgPool>, hashing: Data<HashingService>) -> HttpResponse {
    let repository = UserRepository::new(pool.deref().clone());

    let result: Result<User> = repository.create_user(user.0, hashing.as_ref()).await;

    match result {
        Ok(user) => HttpResponse::Ok().json(user),
        // TODO: Better error response
        Err(error) => match error.root_cause().downcast_ref::<sqlx::postgres::PgError>() {
            Some(pg_error) if pg_error.code() == Some("23505") => {
                error!("Email address already exists. {:?}", error);
                HttpResponse::BadRequest().finish()
            },
            _ => {
                error!("An error ocurred creating the user. {:?}", error);
                HttpResponse::InternalServerError().finish()
            }
        }
    }

}
