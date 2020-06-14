use actix_web::error::ResponseError;
use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use eyre::{DefaultHandler, Report};
use serde::export::Formatter;
use serde::{Serialize, Serializer};
use tracing::error;

#[derive(Debug, Serialize)]
pub struct AppError {
    message: String,
    code: AppErrorCode,
}

#[derive(Debug, PartialEq, Eq)]
pub struct AppErrorCode(i32);

impl AppErrorCode {
    pub fn message(self, _message: String) -> AppError {
        AppError {
            message: _message,
            code: self,
        }
    }

    pub fn default(self) -> AppError {
        let message = match self {
            AppError::INVALID_INPUT => "Invalid input.",
            AppError::INVALID_CREDENTIALS => "Invalid username or password provided",
            AppError::NOT_AUTHORIZED => "Not authorized.",
            AppError::NOT_FOUND => "Item not found.",
            _ => "An unexpected error has occurred.",
        };
        AppError {
            message: message.to_string(),
            code: self,
        }
    }
}

impl AppError {
    pub const INTERNAL_ERROR: AppErrorCode = AppErrorCode(1001);
    pub const INVALID_INPUT: AppErrorCode = AppErrorCode(2001);
    pub const INVALID_CREDENTIALS: AppErrorCode = AppErrorCode(3001);
    pub const NOT_AUTHORIZED: AppErrorCode = AppErrorCode(3002);
    pub const NOT_FOUND: AppErrorCode = AppErrorCode(4001);
}

impl Serialize for AppErrorCode {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i32(self.0)
    }
}

impl std::convert::From<eyre::Report> for AppError {
    fn from(e: Report<DefaultHandler>) -> Self {
        error!("{:?}", e);
        Self::INTERNAL_ERROR.message("An unexpected error ocurred.".to_string())
    }
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self.code {
            AppError::INVALID_INPUT => StatusCode::BAD_REQUEST,
            AppError::NOT_FOUND => StatusCode::NOT_FOUND,
            AppError::INVALID_CREDENTIALS => StatusCode::UNAUTHORIZED,
            AppError::NOT_AUTHORIZED => StatusCode::UNAUTHORIZED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).json(self)
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{:?}", self)
    }
}
