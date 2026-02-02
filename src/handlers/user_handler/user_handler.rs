use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;

use crate::handlers::upload_handler::upload_cloudinary;
use crate::models::user::User;
use crate::schemas::user_schema::{
    Pagination, UserQuery, UserResponse, UserStoreRequest, UserStoreResponse, UserUpdateRequest,
};
use crate::utils::password::hash_password;
use crate::utils::response::ApiResponse;
use axum::extract::{Path, Query};
use axum::http::{HeaderMap, StatusCode};
use axum::{Extension, Json, body::Body, response::Response};
use csv::Writer;
use reqwest::Client;
use reqwest::header::{
    ACCEPT, ACCEPT_LANGUAGE, CONNECTION, CONTENT_TYPE, COOKIE, UPGRADE_INSECURE_REQUESTS,
    USER_AGENT,
};
use rust_xlsxwriter::workbook::Workbook;
use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder};
use serde_json::{Value, json};
use sqlx::MySqlPool;
use validator::Validate;

#[path = "./tests.rs"]
mod tests;

pub async fn index(
    Extension(db): Extension<MySqlPool>,
    Query(query): Query<UserQuery>,
) -> (StatusCode, Json<ApiResponse<Value>>) {
    let page: i64 = query.page.unwrap_or(1);
    let limit: i64 = query.limit.unwrap_or(10);
    let keyword: String = query.keyword.unwrap_or("".to_string());
    let offset = if page > 1 { (page - 1) * limit } else { 0 };

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
        offset,
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

    // upload image base64 to cloudinary
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

    let hashed = hash_password(&payload.password);

    // insert data user to database
    let result = sqlx::query!(
        "INSERT INTO users (name, email, image, password) VALUES (?, ?, ?, ?)",
        payload.name,
        payload.email,
        image_cloudinary,
        hashed.unwrap()
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
        SELECT id, image
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

    //check if image is not empty
    let image_cloudinary: Option<String> = if let Some(image) = &payload.image {
        if !image.is_empty() && !image.contains("http") {
            let image_path = upload_cloudinary(image.clone()).await.unwrap();
            println!("Image path: {:#?}", image_path);
            Some(image_path.secure_url.clone())
        } else {
            user_exist.image
        }
    } else {
        user_exist.image
    };

    // update user
    let result = match &payload.password {
        Some(password) if !password.is_empty() => {
            // Hash password with bcrypt
            let hashed = hash_password(password);
            // Update user with hashed password
            sqlx::query!(
                "
                UPDATE users
                SET name = ?, email = ?, password = ?, image = ?
                WHERE id = ?
                ",
                payload.name,
                payload.email,
                hashed.unwrap(),
                image_cloudinary,
                id
            )
            .execute(&db)
            .await
        }
        _ => {
            // Update user tanpa password
            sqlx::query!(
                "UPDATE users SET name = ?, email = ?, image = ? WHERE id = ?",
                payload.name,
                payload.email,
                image_cloudinary,
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

pub const TOKEN: &str = "GESY7idTDHN5MV7QNzuffrXVfI8ZE3lzLzV7XZbz";
pub const XSRF_TOKEN: &str = "eyJpdiI6IkduZ1NBcXpUVlhMaXdMVFRJMXN4eUE9PSIsInZhbHVlIjoiM1JBWDgxMVcyYnptcWYwTWZhR0NmaVlUUnV4VWVmU3pJU0NZQnBlaUx6MXVxXC9LMnFuR0psejMrRHBKR0dXeWJtNFUxc2RZK0RYdWxLSFVvelh3XC9OQT09IiwibWFjIjoiNzQ5NDNjMjQ2YTVmMDFiOGVjNTFiOGIxYzEzYWZlMTYxMmRkYjkxMTVhYWU0YTA0NmYzMTM4MzRkODMzOGQ3MiJ9";
pub const COOKIE_HEADER: &str = "_tt_enable_cookie=1; _ttp=01KF15YYP1KMQZY1JMTB010KF1_.tt.1; _hjSessionUser_1121314=eyJpZCI6IjIwMjZhNGU3LTYxNzYtNTdjYi05ZDE1LWE2NDkyMzdhYjM5ZSIsImNyZWF0ZWQiOjE3Njg0OTI1OTU5OTEsImV4aXN0aW5nIjp0cnVlfQ==; _fbp=fb.1.1768909651234.661177438798122095; _gid=GA1.2.1912491256.1769503912; _ga_EGQK4VRL7Y=GS2.1.s1769585850$o6$g0$t1769585852$j58$l0$h0; remember_web_59ba36addc2b2f9401580f014c7f58ea4e30989d=eyJpdiI6IkFacktZWFZuZTlXY3IxWmwrZjRSYUE9PSIsInZhbHVlIjoidzVNUHU3U1JFeWF1UDJ2a2FETzBocVRIbHk0bWN2UGdMVTFUb2krc3Z0NUpMQjJnZ2xOM2lVT2psalFucExaMTRRMVlyTjJyS0ExMkdMenU3MHN6bnA2WWo3dVwvSUFMSk53Q0R1RW14VE9GMlkzc3VLUFE1RDEzUXArY2NlYmY0MzQ5eGRcL0pzaXZCMXpyUG1Xd0d2SEYxcmdcL3FPUEx2MlQzVEU3XC91NytSV3Fmb0dXa0hvTE9sZjU4T2hyYWpCUCIsIm1hYyI6ImM4MTE4MWNiYzExMGNjNGU0OTg1YjNkZjE5YmE5OTg3ZmMzOGIwYjcxNjVlODExYzQwMzE5OTMxMTYzZDU5OTQifQ%3D%3D; _hjSession_1121314=eyJpZCI6IjgyOTU2MjZjLWMwMDMtNDE0YS1hZDNlLTU2MjMwNDI0NzIwYSIsImMiOjE3Njk2NDU4NDcyMDQsInMiOjAsInIiOjAsInNiIjowLCJzciI6MCwic2UiOjAsImZzIjowLCJzcCI6MH0=; logammulia_session=VTgwMZYQDCPUyX7faXLYR5R46bk2yrPv5ns9Oncv; cf_clearance=4fC9m1lCQr78KB4Xkz6LQyLsCgAiAz9OR2avS1Lhf0M-1769647885-1.2.1.1-ZAc7IT4Ahh_5qC3k6VP1N1ud..F__pPTcagWv9cGUGmhGE5emPnfK0Wlk6otOsSnXi4_iKX.G4vCwc.hhOQ2Bcv3RWd23rnj97z0zVt7ZYfXm5Snj_5BJUnZWe0hIHlkFgzVUkNU0hOvFqxNNSawHf2rYlxOlrviZ9WS_SyFwETg9KGGRQjEsBxGIojFrea0B2PeWwQVTJ9tT978Q..TRyaNgtsP2X6xE_Ua9hVRYbk; _gat_gtag_UA_117676598_1=1; _gat_UA-117676598-1=1; _ga=GA1.1.1969594460.1768492596; _ga_8XC1TTYW3C=GS2.1.s1769645846$o12$g1$t1769648263$j59$l0$h892778770; XSRF-TOKEN=eyJpdiI6ImNaXC9ueGs2NFB1MkdjN3NCeEZGd0tBPT0iLCJ2YWx1ZSI6InZ4QUQ2XC9TU0JEekphV0FlNE0rSnZieThpdXBFamRZcVVpYXFXM01LeFB3SE9YZURiMkpXODJoM1pOekZxa1NJc3oxWk1cL21obHp4MWkwQWxJbEREaFE9PSIsIm1hYyI6ImJmOTliMjFjOTE4MWI3YjBkNmM5OGI0Y2U5MjNhOGRhYzllM2M4YzJiYmUzY2RiNGVkN2E2YjRkMzcwNDIwODYifQ%3D%3D; _gcl_au=1.1.516091693.1768492596.448682973.1769646129.1769648276; ttcsid_CR39QRJC77U85A2HF4A0=1769645847125::DjTjT0I_tHzzdUUELOD7.10.1769648276291.1; ttcsid=1769645847126::Zs8-WMM4fmjPIz2KFf0J.10.1769648276292.0";

pub async fn api_multiple_order(
    headers: HeaderMap,
    Json(_payload): Json<UserStoreRequest>,
) -> (StatusCode, Json<ApiResponse<Value>>) {
    let client_ip = headers
        .get("CF-Connecting-IP")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_else(|| "");

    println!("Client IP: {}", client_ip);

    let client = Client::new();
    let url = "https://logammulia.com/add-to-cart-multiple";
    let payload = json!({
      "token": TOKEN,
      "id_variant": [
      11, 12, 13, 15, 17, 18, 19, 20, 38, 57, 58, 59
      ],
      "qty": [
      0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0
      ],
      "grand_total": 0,
      "tax_type": "PPH22",
      "tax_rate_npwp": 0,
      "tax_rate_non_npwp": 0,
      "tax_number": "on",
      "ppn_rate": 12,
      "dpp_rate": 0.91666666666667,
      "hemat_brankas": 10,
      "current_url": "https://logammulia.com/id/purchase/gold"
    });

    let res = match client
        .post(url)
        .header("XSRF-TOKEN", XSRF_TOKEN)
        .header("X-CSRF-TOKEN", XSRF_TOKEN)
        .header("CF-Connecting-IP", client_ip)
        .header(USER_AGENT, "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
        .header(ACCEPT, "application/json, text/plain, */*")
        .header(ACCEPT_LANGUAGE, "en-US,en;q=0.9,id;q=0.8")
        .header(CONNECTION, "keep-alive")
        .header(UPGRADE_INSECURE_REQUESTS, "1")
        .header("sec-fetch-dest", "empty")
        .header("sec-fetch-mode", "cors")
        .header("sec-fetch-site", "same-origin")
        .header("sec-fetch-user", "?1")
        .header("sec-ch-ua", "\"Google Chrome\";v=\"131\", \"Chromium\";v=\"131\", \"Not_A Brand\";v=\"24\"")
        .header("sec-ch-ua-mobile", "?0")
        .header("sec-ch-ua-platform", "\"macOS\"")
        .header(CONTENT_TYPE, "application/json")
        .header(COOKIE, COOKIE_HEADER)
        .json(&payload)
        .send()
        .await
    {
        Ok(res) => {
            println!("✅ Request successful!");
            println!("Status: {}", res.status());
            println!("Headers: {:?}", res.headers());
            res
        },
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&format!(
                    "Failed to make request: {}",
                    e
                ))),
            );
        }
    };

    let body = match res.text().await {
        Ok(body) => body,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&format!(
                    "Failed to read response: {}",
                    e
                ))),
            );
        }
    };

    println!("📄 Response Body: {}", body);

    (
        StatusCode::OK,
        Json(ApiResponse::success(
            "External API called successfully",
            json!(body),
        )),
    )
}

pub async fn api_change_profile(
    headers: HeaderMap,
    Json(_payload): Json<UserStoreRequest>,
) -> (StatusCode, Json<ApiResponse<Value>>) {
    let client_ip = headers
        .get("CF-Connecting-IP")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_else(|| "");

    println!("Client IP: {}", client_ip);

    let client = Client::new();
    let url = "https://logammulia.com/my-account";
    let payload = json!({
      "field_post": "all_field",
      "go_checkout": 0,
      "_token": TOKEN,
      "full_name": "Supriyadin",
      "tax_name_profile": "SUPRIYADIN",
      "email": "supri170845@gmail.com",
      "mobile_phone": "087889911369",
      "identity_number": "3172030907880011",
      "birth_place": "Jakartax",
      "birth_date": "1988-07-09",
      "sumber_dana": "Usaha",
      "tujuan_transaksi": "Investasi",
      "jobs": "KARYAWAN SWASTA",
      "code_bank": "008",
      "bank_code": "BANK MANDIRI (PERSERO)",
      "rekening_number": "1260007117582",
      "rekening_name": "SUPRIYADIN",
      "income_value": "10 Juta - 25 Juta Rupiah",
      "id_country": 1,
      "full_name_billing_address": "Supriyadin",
      "mobile_phone_billing_address": "087889911369",
      "province": 11,
      "city": 8783,
      "zip_code": "10640",
      "address": "Jl intanbaiduri no.17 rt:001/003 Sumur batu kec. Kemayoran Jakarta pusat"
    });

    let res = match client
        .post(url)
        .header("XSRF-TOKEN", XSRF_TOKEN)
        .header("X-CSRF-TOKEN", XSRF_TOKEN)
        .header("CF-Connecting-IP", client_ip)
        .header(USER_AGENT, "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
        .header(ACCEPT, "application/json, text/plain, */*")
        .header(ACCEPT_LANGUAGE, "en-US,en;q=0.9,id;q=0.8")
        .header(CONNECTION, "keep-alive")
        .header(UPGRADE_INSECURE_REQUESTS, "1")
        .header("sec-fetch-dest", "empty")
        .header("sec-fetch-mode", "cors")
        .header("sec-fetch-site", "same-origin")
        .header("sec-fetch-user", "?1")
        .header("sec-ch-ua", "\"Google Chrome\";v=\"131\", \"Chromium\";v=\"131\", \"Not_A Brand\";v=\"24\"")
        .header("sec-ch-ua-mobile", "?0")
        .header("sec-ch-ua-platform", "\"macOS\"")
        .header(CONTENT_TYPE, "application/json")
        .header(COOKIE, COOKIE_HEADER)
        .json(&payload)
        .send()
        .await
    {
        Ok(res) => {
            println!("✅ Request successful!");
            println!("Status: {}", res.status());
            println!("Headers: {:?}", res.headers());
            res
        },
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&format!(
                    "Failed to make request: {}",
                    e
                ))),
            );
        }
    };

    let body = match res.text().await {
        Ok(body) => body,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&format!(
                    "Failed to read response: {}",
                    e
                ))),
            );
        }
    };

    println!("📄 Response Body: {}", body);

    (
        StatusCode::OK,
        Json(ApiResponse::success(
            "External API called successfully",
            json!(body),
        )),
    )
}

pub async fn export_excel(
    Extension(db): Extension<MySqlPool>,
    Query(query): Query<UserQuery>,
) -> Result<Response<Body>, (StatusCode, Json<ApiResponse<Value>>)> {
    let page: i64 = query.page.unwrap_or(1);
    let limit: i64 = query.limit.unwrap_or(10);
    let keyword: String = query.keyword.unwrap_or("".to_string());
    let offset = if page > 1 { (page - 1) * limit } else { 0 };

    // get all users data (no pagination for export)
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
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&format!("Failed to fetch users: {}", e))),
            ));
        }
    };

    let mut workbook = Workbook::new();

    // Create formats
    let header_format = Format::new()
        .set_bold()
        .set_border(FormatBorder::Thin)
        .set_background_color(Color::RGB(0xE6E6FA))
        .set_align(FormatAlign::Center);

    // Add worksheet
    let worksheet = workbook.add_worksheet();

    // Set column widths
    let _ = worksheet.set_column_width(0, 10); // ID
    let _ = worksheet.set_column_width(1, 30); // Name
    let _ = worksheet.set_column_width(2, 40); // Email
    let _ = worksheet.set_column_width(3, 20); // Created At
    let _ = worksheet.set_column_width(4, 20); // Updated At

    // Write headers
    worksheet
        .write_with_format(0, 0, "ID", &header_format)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&format!(
                    "Failed to write headers: {}",
                    e
                ))),
            )
        })?;
    worksheet
        .write_with_format(0, 1, "Name", &header_format)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&format!(
                    "Failed to write headers: {}",
                    e
                ))),
            )
        })?;
    worksheet
        .write_with_format(0, 2, "Email", &header_format)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&format!(
                    "Failed to write headers: {}",
                    e
                ))),
            )
        })?;
    worksheet
        .write_with_format(0, 3, "Created At", &header_format)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&format!(
                    "Failed to write headers: {}",
                    e
                ))),
            )
        })?;
    worksheet
        .write_with_format(0, 4, "Updated At", &header_format)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&format!(
                    "Failed to write headers: {}",
                    e
                ))),
            )
        })?;

    // Write data
    for (row, user) in users.iter().enumerate() {
        let row_num: u32 = (row + 1) as u32;
        worksheet.write(row_num, 0, user.id).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&format!(
                    "Failed to write user data: {}",
                    e
                ))),
            )
        })?;
        worksheet.write(row_num, 1, &user.name).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&format!(
                    "Failed to write user data: {}",
                    e
                ))),
            )
        })?;
        worksheet.write(row_num, 2, &user.email).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&format!(
                    "Failed to write user data: {}",
                    e
                ))),
            )
        })?;

        let created_at_str = user
            .created_at
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "N/A".to_string());
        worksheet.write(row_num, 3, &created_at_str).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&format!(
                    "Failed to write user data: {}",
                    e
                ))),
            )
        })?;

        let updated_at_str = user
            .updated_at
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "N/A".to_string());
        worksheet.write(row_num, 4, &updated_at_str).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&format!(
                    "Failed to write user data: {}",
                    e
                ))),
            )
        })?;
    }

    // workbook.save("test.xlsx").map_err(|e| {
    //     (
    //         StatusCode::INTERNAL_SERVER_ERROR,
    //         Json(ApiResponse::error(&format!(
    //             "Failed to save Excel file: {}",
    //             e
    //         ))),
    //     )
    // })?;

    // Generate Excel file in memory
    let buffer = workbook.save_to_buffer().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&format!(
                "Failed to generate Excel file: {}",
                e
            ))),
        )
    })?;

    // Create response with proper headers for file download
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(
            "Content-Type",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        )
        .header(
            "Content-Disposition",
            "attachment; filename=\"users_export.xlsx\"",
        )
        .header("Content-Length", buffer.len())
        .body(Body::from(buffer))
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&format!(
                    "Failed to create response: {}",
                    e
                ))),
            )
        })?;

    Ok(response)
}

pub async fn export_csv(
    Extension(db): Extension<MySqlPool>,
    Query(query): Query<UserQuery>,
) -> Result<Response<Body>, (StatusCode, Json<ApiResponse<Value>>)> {
    let page: i64 = query.page.unwrap_or(1);
    let limit: i64 = query.limit.unwrap_or(10);
    let keyword: String = query.keyword.unwrap_or("".to_string());
    let offset = if page > 1 { (page - 1) * limit } else { 0 };

    // get all users data (no pagination for export)
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
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&format!("Failed to fetch users: {}", e))),
            ));
        }
    };

    let file = File::create("users.csv").map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&format!(
                "Failed to create CSV file: {}",
                e
            ))),
        )
    })?;
    let mut wtr = Writer::from_writer(file);

    // Write CSV header
    wtr.write_record(&["ID", "Name", "Email", "Password", "Image"])
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&format!(
                    "Failed to write CSV header: {}",
                    e
                ))),
            )
        })?;

    // Write CSV data
    for user in users {
        wtr.write_record(&[
            user.id.to_string(),
            user.name.clone(),
            user.email.clone(),
            user.password.clone(),
            user.image.clone().unwrap_or_default(),
        ])
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&format!(
                    "Failed to write CSV data: {}",
                    e
                ))),
            )
        })?;
    }

    wtr.flush().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&format!(
                "Failed to flush CSV writer: {}",
                e
            ))),
        )
    })?;

    // Read the CSV file and return as download
    let csv_content = std::fs::read("users.csv").map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&format!(
                "Failed to read CSV file: {}",
                e
            ))),
        )
    })?;

    // Return the CSV file as response
    let response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/csv")
        .header(
            "Content-Disposition",
            "attachment; filename=\"users_export.csv\"",
        )
        .body(Body::from(csv_content))
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&format!(
                    "Failed to create response: {}",
                    e
                ))),
            )
        })?;

    Ok(response)
}
