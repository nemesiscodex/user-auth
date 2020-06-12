use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(sqlx::FromRow, Serialize)]
pub struct User {
    pub id: Uuid,
    pub username: Option<String>,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub full_name: Option<String>,
    pub bio: Option<String>,
    pub image: Option<String>,
    pub email_verified: bool,
    pub active: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// enum Roles {
//     ADMIN,
//     USER
// }

// pub struct UserRoles {
//     id: Uuid,
//     role: Roles
// }

#[derive(Debug, Deserialize, Validate)]
pub struct NewUser {
    #[validate(length(min = 3))]
    pub username: String,
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 3))]
    pub password: String,
}

// #[derive(Deserialize)]
// pub struct UpdateProfile {
//     id: Uuid,
//     full_name: Option<String>,
//     bio: Option<String>
// }

#[derive(Serialize)]
pub struct Auth {
    pub token: String,
}
