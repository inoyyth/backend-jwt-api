use crate::utils::response::ApiResponse;
use axum::Json;
use bcrypt::hash;
use serde_json::Value;

pub fn hash_password(password: &str) -> Result<String, Json<ApiResponse<Value>>> {
    let hashed = match hash(password, 10) {
        Ok(hashed) => hashed,
        Err(e) => {
            return Err(Json(ApiResponse::error(&format!(
                "Failed to hash password: {}",
                e
            ))));
        }
    };
    Ok(hashed)
}
