use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(length(min = 3, message = "Nama minimal 3 karakter"))]
    pub name: String,
    #[validate(email(message = "Email tidak valid"))]
    pub email: String,
    #[validate(length(min = 6, message = "Password minimal 6 karakter"))]
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterResponse {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}
