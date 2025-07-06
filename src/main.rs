use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use axum::{
    Json, Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
    routing::{any, get},
    serve,
};

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::{select, spawn, sync::broadcast};

struct AppState {
    user_set: Mutex<HashSet<String>>,
    tx: broadcast::Sender<String>,
}

#[derive(Serialize, Deserialize)]
struct SendMessage {
    content: String,
    from: String,
    time: String,
}

#[tokio::main]
async fn main() {
    let user_set = Mutex::new(HashSet::new());
    let (tx, _rx) = broadcast::channel(100);

    let app_state = Arc::new(AppState { user_set, tx });

    let app = Router::new()
        .route("/", get(root))
        .route("/ws", any(web_socket))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:4399").await.unwrap();

    println!("[Server] starting service on port 4399");
    serve(listener, app).await.unwrap();
}

async fn root() -> Json<Value> {
    Json(json!({"Hello": "world"}))
}

async fn web_socket(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(stream: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = stream.split();

    let mut username = String::new();

    while let Some(Ok(message)) = receiver.next().await {
        if let Message::Text(welcome_msg) = message {
            let v: SendMessage = match serde_json::from_str(welcome_msg.as_str()) {
                Ok(value) => value,
                Err(error) => {
                    let msg = Message::Text("Wrong format data. ".into());
                    sender.send(msg).await.unwrap();
                    println!("[Server] wrong format: {}", error);
                    return;
                }
            };

            check_username(&state, &mut username, v.from.as_str());

            if !username.is_empty() {
                break;
            } else {
                let msg = Message::Text(v.content.into());
                sender.send(msg).await.unwrap();
            }
        }
    }

    let mut rx = state.tx.subscribe();

    let msg = format!("{username} joined.");
    state.tx.send(msg).unwrap();

    let mut send_task = spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    let tx = state.tx.clone();
    let name = username.clone();

    let mut recv_task = spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            tx.send(format!("{name}: {text}")).unwrap();
        }
    });

    select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }

    let msg = format!("{username} left. ");

    state.tx.send(msg).unwrap();

    state.user_set.lock().unwrap().remove(&username);
}

fn check_username(state: &AppState, string: &mut String, name: &str) {
    let mut user_set = state.user_set.lock().unwrap();

    if !user_set.contains(name) {
        user_set.insert(name.to_owned());

        string.push_str(name);
    }
}
