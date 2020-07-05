use super::AppResponse;
use crate::{
    config::crypto::{Auth, CryptoService},
    db::user::UserRepository,
    errors::AppError,
};
use actix_web::{web::Data, FromRequest, HttpResponse};
use actix_web_httpauth::extractors::{basic::BasicAuth, bearer::BearerAuth};
use futures::future::{ready, BoxFuture};
use tracing::{debug, instrument};
use uuid::Uuid;

#[derive(Debug)]
pub struct AuthenticatedUser(pub Uuid);

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
                let future = async move {
                    let user_id: Uuid = crypto_service
                        .verify_jwt(bearer.token().to_string())
                        .await
                        .map(|data| data.claims.sub)
                        .map_err(|err| {
                            debug!("Cannot verify jwt. {:?}", err);
                            AppError::NOT_AUTHORIZED
                        })?;

                    repository.find_by_id(user_id).await?.ok_or_else(|| {
                        debug!("User {} not found", user_id);
                        AppError::NOT_AUTHORIZED
                    })?;

                    Ok(AuthenticatedUser(user_id))
                };
                Box::pin(future)
            }
            _ => {
                let error = ready(Err(AppError::NOT_AUTHORIZED.into()));
                Box::pin(error)
            }
        }
    }
}

#[instrument(skip(basic, repository, hashing))]
pub async fn auth(
    basic: BasicAuth,
    repository: UserRepository,
    hashing: Data<CryptoService>,
) -> AppResponse {
    let username = basic.user_id();
    let password = basic
        .password()
        .ok_or_else(|| {
            debug!("Invalid request. Missing Basic Auth.");
            AppError::INVALID_CREDENTIALS
        })?;

    let user = repository
        .find_by_username(username)
        .await?
        .ok_or_else(|| {
            debug!("User doesn't exist.");
            AppError::INVALID_CREDENTIALS
        })?;

    let valid = hashing
        .verify_password(password, &user.password_hash)
        .await?;

    if valid {
        let token = hashing.generate_jwt(user.id).await?;
        Ok(HttpResponse::Ok().json(Auth { token }))
    } else {
        debug!("Invalid password.");
        Err(AppError::INVALID_CREDENTIALS.into())
    }
}
