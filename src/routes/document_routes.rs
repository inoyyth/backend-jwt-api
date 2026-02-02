use crate::{handlers::document_handler, middlewares::auth_middleware::auth};
use axum::{Router, middleware, routing::post};

pub fn document_routes() -> Router {
    Router::new()
        .route(
            "/document/upload-chunk",
            post(document_handler::upload_chunk),
        )
        .route(
            "/document/complete-upload",
            post(document_handler::complete_upload),
        )
        .layer(middleware::from_fn(auth))
}
