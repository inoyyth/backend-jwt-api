use crate::models::user::User;
use crate::schemas::login_schema::{LoginRequest, LoginResponse, UserResponse};
use crate::utils::{jwt::generate_token, response::ApiResponse};
use axum::{Extension, Json, http::StatusCode};
use bcrypt::verify;
use serde_json::{Value, json};
use sqlx::MySqlPool;
use std::collections::HashMap;
use validator::Validate;

pub async fn login(
    Extension(db): Extension<MySqlPool>,
    Json(payload): Json<LoginRequest>,
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

    // Get user by email
    let user = sqlx::query_as!(
        User,
        "SELECT id, name, email, password, created_at, updated_at, image, deleted_at FROM users WHERE email = ?",
        payload.email
    )
    .fetch_one(&db)
    .await;

    let user_result = match user {
        Ok(user) => user,
        Err(sqlx::Error::RowNotFound) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::error("Invalid credentials")),
            );
        }
        Err(e) => {
            eprintln!("Database Error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Internal server error")),
            );
        }
    };

    // Verication password with Bycrpt
    match verify(payload.password, &user_result.password) {
        Ok(true) => {
            //Generate token JWT
            match generate_token(user_result.id) {
                Ok(token) => {
                    let response = LoginResponse {
                        token,
                        user: UserResponse {
                            id: user_result.id,
                            email: user_result.email,
                            name: user_result.name,
                        },
                    };

                    // Return success response
                    (
                        StatusCode::OK,
                        Json(ApiResponse::success("Login successful", json!(response))),
                    )
                }
                Err(e) => {
                    println!("Token Generation Error: {}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::error("Failed to generate token")),
                    );
                }
            }
        }
        Ok(false) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::error("Invalid credentials")),
            );
        }
        Err(e) => {
            println!("Password Verification Error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Internal server error")),
            );
        }
    }
}
