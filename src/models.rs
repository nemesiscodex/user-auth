use uuid::Uuid;
use chrono::NaiveDateTime;
use serde::{Serialize, Deserialize};

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
    pub updated_at: NaiveDateTime
}

// enum Roles {
//     ADMIN,
//     USER
// }

// pub struct UserRoles {
//     id: Uuid,
//     role: Roles
// }

#[derive(Debug, Deserialize)]
pub struct NewUser {
    pub email: String,
    pub password: String
}

// #[derive(Deserialize)]
// pub struct UpdateProfile {
//     id: Uuid,
//     full_name: Option<String>,
//     bio: Option<String>
// }

// #[derive(Deserialize)]
// struct Login {
//     email: Option<String>,
//     username: Option<String>,
//     password: String
// }

#[derive(Serialize)]
struct Auth {
    token: String
}