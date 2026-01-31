use crate::{
    schemas::user_schema::{
        Pagination, UserQuery, UserResponse, UserStoreRequest, UserStoreResponse, UserUpdateRequest,
    },
    utils::response::ApiResponse,
};

use super::*;
use axum::http::HeaderMap;
use chrono::Utc;
use reqwest::StatusCode;
use serde_json::{Value, json};
use validator::Validate;

// Helper function to create test headers
pub fn create_test_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("CF-Connecting-IP", "127.0.0.1".parse().unwrap());
    headers
}

// Helper function to create test user store request
pub fn create_test_user_store_request() -> UserStoreRequest {
    UserStoreRequest {
        name: "Test User".to_string(),
        email: "test@example.com".to_string(),
        password: "password123".to_string(),
        image: Some("test.jpg".to_string()),
    }
}

// Helper function to create test user update request
pub fn create_test_user_update_request() -> UserUpdateRequest {
    UserUpdateRequest {
        name: "Updated User".to_string(),
        email: "updated@example.com".to_string(),
        password: Some("newpassword123".to_string()),
        image: Some("updated.jpg".to_string()),
    }
}

#[tokio::test]
async fn test_user_query_deserialization() {
    // Test UserQuery struct with various inputs
    let query_json = json!({
        "page": 2,
        "limit": 5,
        "keyword": "test"
    });

    let query: UserQuery = serde_json::from_value(query_json).unwrap();
    assert_eq!(query.page, Some(2));
    assert_eq!(query.limit, Some(5));
    assert_eq!(query.keyword, Some("test".to_string()));
}

#[tokio::test]
async fn test_user_query_defaults() {
    // Test UserQuery with empty values
    let query_json = json!({});

    let query: UserQuery = serde_json::from_value(query_json).unwrap();
    assert_eq!(query.page, None);
    assert_eq!(query.limit, None);
    assert_eq!(query.keyword, None);
}

#[tokio::test]
async fn test_user_store_request_validation() {
    // Test valid user store request
    let valid_request = UserStoreRequest {
        name: "Valid Name".to_string(),
        email: "valid@example.com".to_string(),
        password: "password123".to_string(),
        image: Some("image.jpg".to_string()),
    };

    assert!(valid_request.validate().is_ok());

    // Test invalid email
    let invalid_email_request = UserStoreRequest {
        name: "Valid Name".to_string(),
        email: "invalid-email".to_string(),
        password: "password123".to_string(),
        image: None,
    };

    assert!(invalid_email_request.validate().is_err());

    // Test short password
    let short_password_request = UserStoreRequest {
        name: "Valid Name".to_string(),
        email: "valid@example.com".to_string(),
        password: "123".to_string(),
        image: None,
    };

    assert!(short_password_request.validate().is_err());

    // Test empty name
    let empty_name_request = UserStoreRequest {
        name: "".to_string(),
        email: "valid@example.com".to_string(),
        password: "password123".to_string(),
        image: None,
    };

    assert!(empty_name_request.validate().is_err());
}

#[tokio::test]
async fn test_user_update_request_validation() {
    // Test valid user update request
    let valid_request = UserUpdateRequest {
        name: "Updated Name".to_string(),
        email: "updated@example.com".to_string(),
        password: Some("newpassword123".to_string()),
        image: Some("updated.jpg".to_string()),
    };

    assert!(valid_request.validate().is_ok());

    // Test valid request without password
    let no_password_request = UserUpdateRequest {
        name: "Updated Name".to_string(),
        email: "updated@example.com".to_string(),
        password: None,
        image: None,
    };

    assert!(no_password_request.validate().is_ok());

    // Test invalid email
    let invalid_email_request = UserUpdateRequest {
        name: "Updated Name".to_string(),
        email: "invalid-email".to_string(),
        password: None,
        image: None,
    };

    assert!(invalid_email_request.validate().is_err());
}

#[tokio::test]
async fn test_pagination_calculation() {
    // Test pagination logic
    let page = 2;
    let limit = 10;
    let offset = if page > 1 { (page - 1) * limit } else { 0 };

    assert_eq!(offset, 10);

    let page = 1;
    let limit = 10;
    let offset = if page > 1 { (page - 1) * limit } else { 0 };

    assert_eq!(offset, 0);
}

#[tokio::test]
async fn test_user_response_serialization() {
    let user_response = UserResponse {
        data: vec![UserStoreResponse {
            id: 1,
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            image: Some("image.jpg".to_string()),
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
        }],
        pagination: Pagination {
            page: 1,
            limit: 10,
            total: 1,
            total_page: 1,
        },
    };

    let json = serde_json::to_string(&user_response).unwrap();
    assert!(json.contains("Test User"));
    assert!(json.contains("test@example.com"));
}

#[tokio::test]
async fn test_api_response_success() {
    let success_response = ApiResponse::success("Success message", json!({"key": "value"}));

    let json = serde_json::to_string(&success_response).unwrap();
    assert!(json.contains("\"status\":true"));
    assert!(json.contains("Success message"));
    assert!(json.contains("key"));
}

#[tokio::test]
async fn test_api_response_error() {
    let error_response: ApiResponse<Value> = ApiResponse::error("Error message");

    let json = serde_json::to_string(&error_response).unwrap();
    assert!(json.contains("\"status\":false"));
    assert!(json.contains("Error message"));
}

#[tokio::test]
async fn test_header_extraction() {
    let mut headers = HeaderMap::new();
    headers.insert("CF-Connecting-IP", "192.168.1.1".parse().unwrap());
    headers.insert("User-Agent", "Test-Agent".parse().unwrap());

    let client_ip = headers
        .get("CF-Connecting-IP")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_else(|| "");

    assert_eq!(client_ip, "192.168.1.1");
}

#[tokio::test]
async fn test_header_extraction_missing() {
    let headers = HeaderMap::new();

    let client_ip = headers
        .get("CF-Connecting-IP")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_else(|| "");

    assert_eq!(client_ip, "");
}

#[tokio::test]
async fn test_json_payload_creation() {
    let payload = json!({
        "token": "test_token",
        "id_variant": [1, 2, 3],
        "qty": [1, 2, 3],
        "grand_total": 100,
        "tax_type": "PPH22",
        "tax_rate_npwp": 0,
        "tax_rate_non_npwp": 0,
        "tax_number": "on",
        "ppn_rate": 12,
        "dpp_rate": 0.91666666666667,
        "hemat_brankas": 10,
        "current_url": "https://example.com"
    });

    let json_str = payload.to_string();
    assert!(json_str.contains("test_token"));
    assert!(json_str.contains("PPH22"));
    assert!(json_str.contains("https://example.com"));
}

#[tokio::test]
async fn test_status_code_handling() {
    // Test various status codes
    let success_status = StatusCode::OK;
    let error_status = StatusCode::INTERNAL_SERVER_ERROR;
    let not_found_status = StatusCode::NOT_FOUND;

    assert_eq!(success_status.as_u16(), 200);
    assert_eq!(error_status.as_u16(), 500);
    assert_eq!(not_found_status.as_u16(), 404);
}

#[tokio::test]
async fn test_datetime_handling() {
    let now = Utc::now();
    let user_response = UserStoreResponse {
        id: 1,
        name: "Test User".to_string(),
        email: "test@example.com".to_string(),
        image: None,
        created_at: Some(now),
        updated_at: Some(now),
    };

    let json = serde_json::to_string(&user_response).unwrap();
    assert!(json.contains("created_at"));
    assert!(json.contains("updated_at"));
}

#[tokio::test]
async fn test_external_api_headers() {
    let headers = create_test_headers();
    let client_ip = headers
        .get("CF-Connecting-IP")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_else(|| "");

    assert!(!client_ip.is_empty());
    assert_eq!(client_ip, "127.0.0.1");
}

#[tokio::test]
async fn test_request_payload_validation() {
    let payload = create_test_user_store_request();

    // Test that the payload is valid
    assert!(payload.validate().is_ok());
    assert_eq!(payload.name, "Test User");
    assert_eq!(payload.email, "test@example.com");
    assert_eq!(payload.password, "password123");
    assert!(payload.image.is_some());
}

#[tokio::test]
async fn test_update_payload_validation() {
    let payload = create_test_user_update_request();

    // Test that the update payload is valid
    assert!(payload.validate().is_ok());
    assert_eq!(payload.name, "Updated User");
    assert_eq!(payload.email, "updated@example.com");
    assert!(payload.password.is_some());
    assert!(payload.image.is_some());
}

// Test edge cases
#[tokio::test]
async fn test_empty_payload_handling() {
    let empty_payload = json!({});

    // Should be able to serialize empty JSON
    let json_str = empty_payload.to_string();
    assert_eq!(json_str, "{}");
}

#[tokio::test]
async fn test_large_payload_handling() {
    let large_name = "a".repeat(300); // Exceeds 255 character limit
    let invalid_request = UserStoreRequest {
        name: large_name.clone(),
        email: "test@example.com".to_string(),
        password: "password123".to_string(),
        image: None,
    };

    // Should fail validation due to name length
    assert!(invalid_request.validate().is_err());
}

#[tokio::test]
async fn test_special_characters_in_email() {
    let special_emails = vec![
        "test+alias@example.com",
        "user.name@example.com",
        "user123@example-domain.com",
        "invalid-email", // This should fail
    ];

    for email in special_emails {
        let request = UserStoreRequest {
            name: "Test User".to_string(),
            email: email.to_string(),
            password: "password123".to_string(),
            image: None,
        };

        if email == "invalid-email" {
            assert!(request.validate().is_err());
        } else {
            assert!(request.validate().is_ok());
        }
    }
}

#[tokio::test]
async fn test_optional_image_field() {
    // Test with image
    let with_image = UserStoreRequest {
        name: "Test User".to_string(),
        email: "test@example.com".to_string(),
        password: "password123".to_string(),
        image: Some("image.jpg".to_string()),
    };
    assert!(with_image.validate().is_ok());

    // Test without image
    let without_image = UserStoreRequest {
        name: "Test User".to_string(),
        email: "test@example.com".to_string(),
        password: "password123".to_string(),
        image: None,
    };
    assert!(without_image.validate().is_ok());
}

#[tokio::test]
async fn test_password_complexity() {
    let test_cases = vec![
        ("12345", false),              // Too short
        ("123456", true),              // Valid length
        ("password", true),            // Valid length
        ("verylongpassword123", true), // Valid
        ("𐍈𐍈𐍈𐍈𐍈𐍈", true),              // Unicode characters
    ];

    for (password, should_be_valid) in test_cases {
        let request = UserStoreRequest {
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            password: password.to_string(),
            image: None,
        };

        if should_be_valid {
            assert!(
                request.validate().is_ok(),
                "Password '{}' should be valid",
                password
            );
        } else {
            assert!(
                request.validate().is_err(),
                "Password '{}' should be invalid",
                password
            );
        }
    }
}
