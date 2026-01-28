use crate::utils::jwt::verify_token;
use crate::utils::response::ApiResponse;

use axum::{
    Json,
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};

// type alias for error response
type AuthError = (StatusCode, Json<ApiResponse<()>>);

// Middleware authentication
pub async fn auth(headers: HeaderMap, mut req: Request, next: Next) -> Result<Response, AuthError> {
    // TODO: Implement JWT validation logic here
    // 1. Check if Authorization header exists
    // 2. Extract Bearer token
    // 3. Validate JWT token
    // 4. Extract user info from token
    // 5. Add user info to request extensions if valid

    // Extract authorization header
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::<()>::error(
                    "Missing or invalid Authorization header",
                )),
            )
        });

    // Token Verification
    let claims = verify_token(token?).map_err(|e| {
        println!("JWT Verification Error: {:?}", e);
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::<()>::error("Invalid or expired token")),
        )
    })?;

    // Add user info to request extensions
    req.extensions_mut().insert(claims);

    // For now, just pass through (will be implemented later)
    Ok(next.run(req).await)
}
