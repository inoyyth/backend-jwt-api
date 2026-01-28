use crate::handlers::{login_handler::login, register_handler::register};
use axum::{Router, routing::post};

pub fn auth_routes() -> Router {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
}
