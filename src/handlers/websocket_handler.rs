use axum::extract::ws::{Message, WebSocket};
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use tokio::sync::broadcast;

use crate::schemas::message_schema::{ClientMessage, ServerMessage};

pub async fn handle_socket(
    socket: WebSocket,
    tx: broadcast::Sender<ServerMessage>,
) {
    println!("Client connected");

    let (mut sender, mut receiver) = socket.split();
    let mut rx = tx.subscribe();

    let mut username = String::new();

    loop {
        tokio::select! {

            // Receive broadcast messages → send to client
            result = rx.recv() => {
                match result {
                    Ok(msg) => {
                        if let Ok(json) = serde_json::to_string(&msg) {
                            if sender.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(_) => break,
                }
            }

            // Receive client messages
            result = receiver.next() => {
                match result {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<ClientMessage>(&text) {
                            Ok(msg) => match msg {
                                ClientMessage::Join { username: name } => {
                                    username = name.clone();
                                    let _ = tx.send(ServerMessage::UserJoined { username: name, time: Some(Utc::now()) });
                                }
                                ClientMessage::Chat { message } => {
                                    let _ = tx.send(ServerMessage::Chat {
                                        username: username.clone(),
                                        message,
                                        time: Some(Utc::now()),
                                    });
                                }
                            },
                            Err(e) => {
                                println!("JSON parse error: {:?}", e);
                            }
                        }
                    }

                    Some(Ok(Message::Close(_))) => {
                        println!("Client disconnected");
                        break;
                    }

                    Some(Ok(_)) => {}

                    Some(Err(e)) => {
                        println!("WebSocket error: {:?}", e);
                        break;
                    }

                    None => break,
                }
            }
        }
    }

    println!("Connection fully closed");
}

