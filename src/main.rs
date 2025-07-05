use axum::{
    Json, Router,
    extract::ws::{Message, Utf8Bytes, WebSocket, WebSocketUpgrade},
    response::Response,
    routing::{any, get},
    serve,
};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(root))
        .route("/ws", any(web_socket));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:4399").await.unwrap();

    println!("[Server] starting service on port 4399");
    serve(listener, app).await.unwrap();
}

async fn root() -> Json<Value> {
    Json(json!({"Hello": "world"}))
}

async fn web_socket(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_socket)
}

#[derive(Serialize, Deserialize)]
struct SendMessage {
    content: String,
    from: String,
    time: String,
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(msg) = socket.recv().await {
        let msg = match msg {
            Ok(value) => value,
            Err(err) => {
                println!("[Server] Error: {}", err);
                return;
            }
        };

        match msg {
            Message::Text(content) => {
                let v: SendMessage = match serde_json::from_str(content.as_str()) {
                    Ok(value) => value,
                    Err(error) => {
                        println!("wrong format: {}", error);
                        return;
                    }
                };

                let json_str = serde_json::to_string(&v).unwrap();

                let send_message = Message::Text(Utf8Bytes::from(&json_str));

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
    }
}
