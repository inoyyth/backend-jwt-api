use serde::{Deserialize, Serialize};
use validator::Validate;

use chrono::NaiveDateTime;

#[derive(Debug, Serialize, Deserialize)]
pub struct Document {
    pub id: i64,
    pub name: String,
    pub file_id: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub deleted_at: Option<NaiveDateTime>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Pagination {
    pub page: i64,
    pub limit: i64,
    pub total: i64,
    pub total_page: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentResponse {
    pub data: Vec<Document>,
    pub pagination: Pagination,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DocumentRequest {
    #[validate(length(min = 1, max = 255, message = "Nama wajib diisi"))]
    pub name: String,
    pub file_id: String,
    #[validate(length(min = 1, message = "File wajib diisi"))]
    pub file: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub keyword: Option<String>,
}

#[derive(Deserialize, Validate)]
pub struct CompletePayload {
    #[validate(length(min = 1, max = 255, message = "ID wajib diisi"))]
    pub file_id: String,
    #[validate(length(min = 1, max = 255, message = "Nama wajib diisi"))]
    pub name: String,
    pub extention: String,
}
