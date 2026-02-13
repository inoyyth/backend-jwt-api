use axum::http::HeaderMap;
use axum::{
    Router,
    extract::WebSocketUpgrade,
    routing::get,
};
use tokio::sync::broadcast;
use crate::handlers::websocket_handler::handle_socket;
use crate::routes::message::ServerMessage;

pub fn websocket_routes() -> Router {
    let (tx, _rx): (broadcast::Sender<ServerMessage>, broadcast::Receiver<ServerMessage>) =
        // Create a broadcast channel with capacity 100
        broadcast::channel::<ServerMessage>(100);
    let tx_state = tx.clone();
    Router::new().route(
        "/ws",
        get(move |ws: WebSocketUpgrade, header: HeaderMap| {
            let tx = tx.clone();
            println!("Incoming WS request");
            println!("Origin: {:?}", header.get("origin"));
            // Clone tx for each connection
            async move { ws.on_upgrade(move |socket| handle_socket(socket, tx)) }
        }),
    )
    .with_state(tx_state)
}
