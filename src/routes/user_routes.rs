use crate::{handlers::user_handler, middlewares::auth_middleware::auth};
use axum::{
    Router, middleware,
    routing::{delete, get, post, put},
};

pub fn user_routes() -> Router {
    Router::new()
        .route("/user", get(user_handler::index))
        .route("/user", post(user_handler::store))
        .route("/user/{id}", get(user_handler::show))
        .route("/user/{id}", put(user_handler::update))
        .route("/user/{id}", delete(user_handler::delete))
        .route("/user/api-external", post(user_handler::api_multiple_order))
        .route(
            "/user/api-change-profile",
            post(user_handler::api_change_profile),
        )
        .route("/user/export-excel", get(user_handler::export_excel))
        .route("/user/export-csv", get(user_handler::export_csv))
        .layer(middleware::from_fn(auth))
}
