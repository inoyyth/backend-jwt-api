// import util response API
use crate::schemas::register_schema::{RegisterRequest, RegisterResponse};
// import util response API
use crate::utils::response::ApiResponse;
use axum::{Extension, Json, http::StatusCode};
use bcrypt::hash;
use serde_json::{Value, json};
use sqlx::MySqlPool;
use std::collections::HashMap;
use validator::Validate;

pub async fn register(
    Extension(db): Extension<MySqlPool>,
    Json(payload): Json<RegisterRequest>,
) -> (StatusCode, Json<ApiResponse<Value>>) {
    // Request Validation
    if let Err(errors) = payload.validate() {
        let mut field_errors: HashMap<String, Vec<String>> = HashMap::new();

        // Collect All Validation Errors
        for (field, errors) in errors.field_errors() {
            let message = errors
                .iter() // Iterate over ValidationErrors
                .filter_map(|e| e.message.as_ref()) // Extract error messages
                .map(|m| m.to_string()) // Convert to String
                .collect::<Vec<String>>(); // Collect into Vec<String>

            field_errors.insert(field.to_string(), message); // Insert into HashMap
        }

        return (
            // Send HTTP 422 unprocessable entity
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiResponse {
                status: false,
                message: "Validation failed".to_string(),
                data: Some(json!(field_errors)),
            }),
        );
    }

    // Hash Password with bcrypt
    let password = match hash(payload.password, 10) {
        Ok(hashed) => hashed,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse {
                    status: false,
                    message: "Failed to hash password".to_string(),
                    data: None,
                }),
            );
        }
    };

    // Check if email already exists
    let user = sqlx::query!("SELECT id FROM users WHERE email = ?", payload.email)
        .fetch_optional(&db)
        .await;

    match user {
        Ok(Some(_)) => {
            return (
                StatusCode::CONFLICT,
                Json(ApiResponse {
                    status: false,
                    message: "Email already exists".to_string(),
                    data: None,
                }),
            );
        }
        Ok(None) => {}
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse {
                    status: false,
                    message: "Failed to check email".to_string(),
                    data: None,
                }),
            );
        }
    }

    // Insert Data User To Database
    let result = sqlx::query!(
        "INSERT INTO users (name, email, password) VALUES (?, ?, ?)",
        payload.name,
        payload.email,
        password
    )
    .execute(&db)
    .await;

    match result {
        Ok(result) => {
            // Get the last insert ID
            let user_id = result.last_insert_id() as i64;

            // Get the user data based on the ID
            let user = sqlx::query!(
                r#"SELECT id, name, email, created_at, updated_at FROM users WHERE id = ?"#,
                user_id
            )
            .fetch_one(&db)
            .await;

            match user {
                Ok(user) => {
                    let response = RegisterResponse {
                        id: user.id,
                        name: user.name,
                        email: user.email,
                        created_at: user.created_at,
                        updated_at: user.updated_at,
                    };

                    (
                        // send HTTP 201 created
                        StatusCode::CREATED,
                        Json(ApiResponse::success("Register Berhasil!", json!(response))),
                    )
                }
                Err(_) => {
                    return (
                        // send HTTP 500 internal server error
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::error("Failed to get user data")),
                    );
                }
            }
        }
        Err(e) => {
            if e.to_string().contains("Duplicate entry") {
                (
                    // kirim response 409 conflict
                    StatusCode::CONFLICT,
                    Json(ApiResponse::error("Email already exists")),
                )
            } else {
                (
                    // kirim response 500 internal server error
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error("Failed to register user")),
                )
            }
        }
    }
}
