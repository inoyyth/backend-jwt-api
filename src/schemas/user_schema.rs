use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use validator::Validate;

fn deserialize_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    Ok(opt.filter(|s| !s.is_empty()))
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UserStoreRequest {
    #[validate(length(
        min = 1,
        max = 255,
        message = "Nama minimal 1 karakter dan maksimal 255 karakter"
    ))]
    pub name: String,
    #[validate(email(message = "Email tidak valid"))]
    pub email: String,
    #[validate(length(min = 6, message = "Password minimal 6 karakter"))]
    pub password: String,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub image: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UserUpdateRequest {
    #[validate(length(
        min = 1,
        max = 255,
        message = "Nama minimal 1 karakter dan maksimal 255 karakter"
    ))]
    pub name: String,
    #[validate(email(message = "Email tidak valid"))]
    pub email: String,
    pub password: Option<String>,
    pub image: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserStoreResponse {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub image: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Pagination {
    pub page: i64,
    pub limit: i64,
    pub total: i64,
    pub total_page: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserResponse {
    pub data: Vec<UserStoreResponse>,
    pub pagination: Pagination,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub keyword: Option<String>,
}
