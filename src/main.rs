use std::{env, process::exit};

use axum::{
    Json, Router,
    extract::ws::{Message, Utf8Bytes, WebSocket, WebSocketUpgrade},
    response::Response,
    routing::{any, get, post},
    serve,
};

use dotenv::dotenv;
use serde_json::{Value, json};

#[tokio::main]
async fn main() {
    dotenv().ok();

    let sending_key = env::var("WS_SENDING_KEY");

    match sending_key {
        Ok(value) => println!("KEY {:?}", value),
        Err(e) => {
            println!("Error ENV_KEY: {}", e);
            exit(404)
        }
    }

    let app = Router::new()
        .route("/", get(root))
        .route("/send", post(send_message))
        .route("/ws", any(web_socket));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:4399").await.unwrap();

    println!("[Server] starting service on port 4399");
    serve(listener, app).await.unwrap();
}

async fn root() -> Json<Value> {
    Json(json!({"test": "Hello world"}))
}

async fn send_message() -> Json<Value> {
    Json(json!({"test": "Hello world"}))
}

async fn web_socket(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    let sending_key = env::var("WS_SENDING_KEY").unwrap();

    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(content) => {
                    let v: Value = match serde_json::from_str(content.as_str()) {
                        Ok(value) => value,
                        Err(error) => {
                            println!("wrong format: {}", error);
                            return;
                        }
                    };

                    let mut send_message = Message::Text(content);

                    if v["token"] != sending_key {
                        send_message = Message::Text(Utf8Bytes::from("Nice try hacker!"));
                    }

                    if socket.send(send_message).await.is_err() {
                        return;
                    }
                }
                Message::Close(_) => {
                    let exit_message = Message::Text(Utf8Bytes::from("User exit"));

                    println!("User exit");

                    if socket.send(exit_message).await.is_err() {
                        return;
                    }
                }
                _ => (),
            }
        } else {
            return;
        };
    }
}
