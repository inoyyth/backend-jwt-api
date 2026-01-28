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
        .layer(middleware::from_fn(auth))
}
