use axum::{
    Extension, Router,
    http::{HeaderValue, Method, header},
};
use dotenvy::dotenv;
use std::{env, net::SocketAddr};
use tower_http::cors::CorsLayer;

mod config;
mod handlers;
mod middlewares;
mod models;
mod routes;
mod schemas;
mod utils;

#[tokio::main]
async fn main() {
    // Load environment variables from .env file
    dotenv().ok();

    // Connect to database
    let db = config::database::connect().await;

    // Cors Configuration
    let cors = CorsLayer::new()
        .allow_origin("http://localhost:5173".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]);

    // Create a base router
    let app = Router::new()
        .merge(routes::auth_routes::auth_routes())
        .merge(routes::user_routes::user_routes())
        .merge(routes::document_routes::document_routes())
        .merge(routes::websocket_routes::websocket_routes())
        .layer(Extension(db))
        .layer(cors);

    let port = env::var("APP_PORT")
        .ok()
        .and_then(|port| port.parse::<u16>().ok())
        .unwrap_or(3000);

    // sever address
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    // show server address on console
    println!("Server running on http://{}", addr);

    // run server
    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app)
        .await
        .unwrap();
}
