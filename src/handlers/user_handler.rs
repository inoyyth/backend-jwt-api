use std::collections::HashMap;

use axum::extract::{Path, Query};
use axum::{Extension, Json, http::StatusCode};
use bcrypt::hash;
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::MySqlPool;

use crate::handlers::upload_handler::upload_cloudinary;
use crate::models::user::User;
use crate::schemas::user_schema::{
    Pagination, UserResponse, UserStoreRequest, UserStoreResponse, UserUpdateRequest,
};
use crate::utils::response::ApiResponse;
use validator::Validate;

#[derive(Deserialize, Debug)]
pub struct UserQuery {
    page: Option<i64>,
    limit: Option<i64>,
    keyword: Option<String>,
}

pub async fn index(
    Extension(db): Extension<MySqlPool>,
    Query(query): Query<UserQuery>,
) -> (StatusCode, Json<ApiResponse<Value>>) {
    println!("Query: {:#?}", query);
    let page: i64 = query.page.unwrap_or(1);
    let limit: i64 = query.limit.unwrap_or(10);
    let keyword: String = query.keyword.unwrap_or("".to_string());
    let offset = if page > 1 { (page - 1) * limit } else { 0 };
    println!(
        "page: {} | Limit: {} | Keyword: {} | Offset: {}",
        page, limit, keyword, offset
    );
    let total_count =
        match sqlx::query!("SELECT COUNT(*) as total FROM users WHERE deleted_at IS NULL")
            .fetch_one(&db)
            .await
        {
            Ok(result) => result.total,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error(&format!("Failed to count users: {}", e))),
                );
            }
        };

    // get all users data
    let users = match sqlx::query_as!(
        User,
        "
        SELECT id, name, email, password, image, created_at, updated_at, deleted_at
        FROM users
        WHERE name LIKE ?
        AND deleted_at IS NULL
        ORDER BY id DESC
        LIMIT ? OFFSET ?
        ",
        format!("%{}%", keyword),
        limit,
        offset
    )
    .fetch_all(&db)
    .await
    {
        Ok(users) => users,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&format!("Failed to fetch users: {}", e))),
            );
        }
    };

    // reponse user with pagination
    let user_response: UserResponse = UserResponse {
        data: users
            .into_iter()
            .map(|user| UserStoreResponse {
                id: user.id,
                name: user.name,
                email: user.email,
                image: user.image,
                created_at: user.created_at,
                updated_at: user.updated_at,
            })
            .collect(),
        pagination: Pagination {
            page: page,
            limit: limit,
            total: total_count,
            total_page: (total_count as f64 / limit as f64).ceil() as i64,
        },
    };

    (
        StatusCode::OK,
        Json(ApiResponse::success("List Users", json!(user_response))),
    )
}

pub async fn store(
    Extension(db): Extension<MySqlPool>,
    Json(payload): Json<UserStoreRequest>,
) -> (StatusCode, Json<ApiResponse<Value>>) {
    // Request Validation
    if let Err(errors) = payload.validate() {
        let mut field_errors: HashMap<String, Vec<String>> = HashMap::new();

        println!("payload {:#?}", payload);

        // Collect All Validation Errors
        for (field, errors) in errors.field_errors() {
            let message = errors
                .iter()
                .filter_map(|e| e.message.as_ref())
                .map(|m| m.to_string())
                .collect::<Vec<String>>();

            field_errors.insert(field.to_string(), message);
        }

        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiResponse {
                status: false,
                message: "Validation failed".to_string(),
                data: Some(json!(field_errors)),
            }),
        );
    }

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

    // upload image to cloudinary
    let image_cloudinary: Option<String> = if let Some(image) = &payload.image {
        if !image.is_empty() {
            let image_path = upload_cloudinary(image.clone()).await.unwrap();
            println!("Image path: {:#?}", image_path);
            Some(image_path.secure_url.clone())
        } else {
            None
        }
    } else {
        None
    };

    // // upload image base6 to folder
    // let image_path = if let Some(image) = &payload.image {
    //     if !image.is_empty() {
    //         let (mime, image_data) = decode_image(image);
    //         let image_path = format!(
    //             "./uploads/{}.{}",
    //             Utc::now().timestamp(),
    //             mime.split('/').last().unwrap()
    //         );
    //         println!("Image path: {}", image_path);
    //         std::fs::create_dir_all("./uploads").unwrap();
    //         std::fs::write(&image_path, image_data).unwrap();
    //         image_path
    //     } else {
    //         "".to_string()
    //     }
    // } else {
    //     "".to_string()
    // };

    // println!("Image path: {}", image_path);

    // insert data user to database
    let result = sqlx::query!(
        "INSERT INTO users (name, email, image, password) VALUES (?, ?, ?, ?)",
        payload.name,
        payload.email,
        image_cloudinary,
        payload.password
    )
    .execute(&db)
    .await;

    match result {
        Ok(result) => {
            // Get the last insert ID
            let user_id = result.last_insert_id() as i64;

            // Get the user data based on the ID
            let user = sqlx::query!(
                r#"SELECT id, name, email, image, created_at, updated_at FROM users WHERE id = ?"#,
                user_id
            )
            .fetch_one(&db)
            .await;

            match user {
                Ok(user) => {
                    let response = UserStoreResponse {
                        id: user.id,
                        name: user.name,
                        email: user.email,
                        image: user.image,
                        created_at: user.created_at,
                        updated_at: user.updated_at,
                    };

                    (
                        // send HTTP 201 created
                        StatusCode::CREATED,
                        Json(ApiResponse::success(
                            "Tambah Data user Berhasil!",
                            json!(response),
                        )),
                    )
                }
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::error(&format!(
                            "Failed to get user data: {}",
                            e
                        ))),
                    );
                }
            }
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&format!("Failed to create user: {}", e))),
            );
        }
    }
}

pub async fn show(
    Path(id): Path<i64>,
    Extension(db): Extension<MySqlPool>,
) -> (StatusCode, Json<ApiResponse<Value>>) {
    // get all users data
    let users = match sqlx::query!(
        "
        SELECT id, name, email, password, image, created_at, updated_at
        FROM users
        WHERE id = ?
        AND deleted_at IS NULL
        ",
        id
    )
    .fetch_one(&db)
    .await
    {
        Ok(users) => users,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&format!("Failed to fetch users: {}", e))),
            );
        }
    };

    let user = UserStoreResponse {
        id: users.id,
        name: users.name,
        email: users.email,
        image: users.image,
        created_at: users.created_at,
        updated_at: users.updated_at,
    };

    (
        StatusCode::OK,
        Json(ApiResponse::success("User", json!(user))),
    )
}

pub async fn update(
    Path(id): Path<i64>,
    Extension(db): Extension<MySqlPool>,
    Json(payload): Json<UserUpdateRequest>,
) -> (StatusCode, Json<ApiResponse<Value>>) {
    // Validation payload
    if let Err(errors) = payload.validate() {
        let mut field_errors: HashMap<String, Vec<String>> = HashMap::new();

        // Collect All Validation Errors
        for (field, errors) in errors.field_errors() {
            let message = errors
                .iter()
                .filter_map(|e| e.message.as_ref())
                .map(|m| m.to_string())
                .collect::<Vec<String>>();

            field_errors.insert(field.to_string(), message);
        }

        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiResponse {
                status: false,
                message: "Validation failed".to_string(),
                data: Some(json!(field_errors)),
            }),
        );
    }

    // Validasi password optional
    if let Some(password) = &payload.password {
        if !password.is_empty() && password.len() < 6 {
            let mut errors = HashMap::new();
            errors.insert(
                "password".to_string(),
                vec!["Password must be at least 6 characters".to_string()],
            );
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(ApiResponse {
                    status: false,
                    message: "Validasi Gagal".to_string(),
                    data: Some(json!(errors)),
                }),
            );
        }
    }

    // check if is user is exist
    let user_exist = match sqlx::query!(
        "
        SELECT id
        FROM users
        WHERE id = ?
        AND deleted_at IS NULL
        ",
        id
    )
    .fetch_one(&db)
    .await
    {
        Ok(user) => user,
        Err(sqlx::Error::RowNotFound) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("User not found")),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&format!("Failed to fetch user: {}", e))),
            );
        }
    };

    // Check if email is already used by another user
    let email_exists = sqlx::query!(
        "
        SELECT id
        FROM users
        WHERE email = ?
        AND id != ?
        AND deleted_at IS NULL
        ",
        payload.email,
        user_exist.id
    )
    .fetch_optional(&db)
    .await;

    if let Ok(Some(_)) = email_exists {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::error("Email already used by another user")),
        );
    }

    // update user
    let result = match &payload.password {
        Some(password) if !password.is_empty() => {
            // Hash password with bcrypt
            let hashed = match hash(password, 10) {
                Ok(hashed) => hashed,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::error(&format!(
                            "Failed to hash password: {}",
                            e
                        ))),
                    );
                }
            };
            // Update user with hashed password
            sqlx::query!(
                "
                UPDATE users
                SET name = ?, email = ?, password = ?
                WHERE id = ?
                ",
                payload.name,
                payload.email,
                hashed,
                id
            )
            .execute(&db)
            .await
        }
        _ => {
            // Update user tanpa password
            sqlx::query!(
                "UPDATE users SET name = ?, email = ? WHERE id = ?",
                payload.name,
                payload.email,
                id
            )
            .execute(&db)
            .await
        }
    };

    if let Err(_) = result {
        return (
            // kirim response 500 Internal Server Error
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("Gagal memperbarui data user")),
        );
    }

    // Ambil data terbaru
    let user = sqlx::query!(
        r#"
        SELECT id, name, email, image, created_at, updated_at
        FROM users
        WHERE id = ?
        "#,
        id
    )
    .fetch_one(&db)
    .await
    .unwrap();

    let response = UserStoreResponse {
        id: user.id,
        name: user.name,
        email: user.email,
        image: user.image,
        created_at: user.created_at,
        updated_at: user.updated_at,
    };

    (
        // kirim response 200 OK
        StatusCode::OK,
        Json(ApiResponse::success(
            "User berhasil diperbarui",
            json!(response),
        )),
    )
}

pub async fn delete(
    Path(id): Path<i32>,
    Extension(db): Extension<MySqlPool>,
) -> (StatusCode, Json<ApiResponse<Value>>) {
    // check if is user is exist
    let user_exist = match sqlx::query!(
        "
        SELECT id
        FROM users
        WHERE id = ?
        AND deleted_at IS NULL
        ",
        id
    )
    .fetch_one(&db)
    .await
    {
        Ok(user) => user,
        Err(sqlx::Error::RowNotFound) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("User not found")),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&format!("Failed to fetch user: {}", e))),
            );
        }
    };

    // soft delete user
    let result = sqlx::query!(
        "
        UPDATE users
        SET deleted_at = NOW()
        WHERE id = ?
        ",
        user_exist.id
    )
    .execute(&db)
    .await;

    if let Err(_) = result {
        return (
            // kirim response 500 Internal Server Error
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("Gagal menghapus data user")),
        );
    }

    (
        // kirim response 200 OK
        StatusCode::OK,
        Json(ApiResponse::success("User berhasil dihapus", json!(null))),
    )
}
