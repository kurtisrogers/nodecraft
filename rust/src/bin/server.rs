//! Nodecraft multiplayer server — block-change overlay + player sync.
//! Run: cargo run --bin nodecraft-server

#[cfg(feature = "server")]
mod server_impl {
    use axum::{
        extract::ws::{Message, WebSocket, WebSocketUpgrade},
        response::IntoResponse,
        routing::get,
        Router,
    };
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::net::SocketAddr;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    use tokio::sync::{broadcast, RwLock};

    #[derive(Clone, Serialize, Deserialize, Debug)]
    #[serde(tag = "type", rename_all = "camelCase")]
    enum ClientMessage {
        #[serde(rename = "join")]
        Join { name: String },
        #[serde(rename = "move")]
        Move { x: f32, y: f32, z: f32, yaw: f32, pitch: f32 },
        #[serde(rename = "breakBlock")]
        BreakBlock { x: i32, y: i32, z: i32 },
        #[serde(rename = "placeBlock")]
        PlaceBlock { x: i32, y: i32, z: i32, block_id: u8 },
    }

    #[derive(Clone, Serialize, Deserialize, Debug)]
    #[serde(tag = "type", rename_all = "camelCase")]
    enum ServerMessage {
        #[serde(rename = "welcome")]
        Welcome {
            player_id: u32,
            seed: u32,
            block_changes: Vec<BlockChange>,
            day_time: f32,
        },
        #[serde(rename = "blockChange")]
        BlockChangeMsg { x: i32, y: i32, z: i32, block_id: u8 },
        #[serde(rename = "playerMove")]
        PlayerMove { id: u32, x: f32, y: f32, z: f32, yaw: f32, pitch: f32 },
    }

    #[derive(Clone, Serialize, Deserialize, Debug)]
    struct BlockChange {
        x: i32,
        y: i32,
        z: i32,
        block_id: u8,
    }

    struct GameState {
        seed: u32,
        block_changes: HashMap<(i32, i32, i32), u8>,
        day_time: f32,
    }

    static NEXT_ID: AtomicU32 = AtomicU32::new(1);

    pub async fn run() {
        let state = Arc::new(RwLock::new(GameState {
            seed: rand::random(),
            block_changes: HashMap::new(),
            day_time: 0.0,
        }));
        let (tx, _) = broadcast::channel::<String>(64);

        let app = Router::new()
            .route("/ws", get(ws_handler))
            .with_state((state, tx));

        let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
        println!("Nodecraft server listening on http://{addr}");
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    }

    async fn ws_handler(
        ws: WebSocketUpgrade,
        axum::extract::State((state, tx)): axum::extract::State<(
            Arc<RwLock<GameState>>,
            broadcast::Sender<String>,
        )>,
    ) -> impl IntoResponse {
        ws.on_upgrade(move |socket| handle_socket(socket, state, tx))
    }

    async fn handle_socket(
        mut socket: WebSocket,
        state: Arc<RwLock<GameState>>,
        tx: broadcast::Sender<String>,
    ) {
        let player_id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let mut rx = tx.subscribe();

        {
            let gs = state.read().await;
            let welcome = ServerMessage::Welcome {
                player_id,
                seed: gs.seed,
                block_changes: gs
                    .block_changes
                    .iter()
                    .map(|(&(x, y, z), &block_id)| BlockChange { x, y, z, block_id })
                    .collect(),
                day_time: gs.day_time,
            };
            let _ = socket
                .send(Message::Text(serde_json::to_string(&welcome).unwrap()))
                .await;
        }

        loop {
            tokio::select! {
                msg = socket.recv() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                                handle_client_message(client_msg, &state, &tx, player_id).await;
                            }
                        }
                        Some(Ok(Message::Close(_))) | None => break,
                        _ => {}
                    }
                }
                msg = rx.recv() => {
                    if let Ok(text) = msg {
                        let _ = socket.send(Message::Text(text)).await;
                    }
                }
            }
        }
    }

    async fn handle_client_message(
        msg: ClientMessage,
        state: &Arc<RwLock<GameState>>,
        tx: &broadcast::Sender<String>,
        player_id: u32,
    ) {
        match msg {
            ClientMessage::Join { .. } => {}
            ClientMessage::Move { x, y, z, yaw, pitch } => {
                let out = ServerMessage::PlayerMove { id: player_id, x, y, z, yaw, pitch };
                let _ = tx.send(serde_json::to_string(&out).unwrap());
            }
            ClientMessage::BreakBlock { x, y, z } => {
                if y == 0 {
                    return;
                }
                let mut gs = state.write().await;
                gs.block_changes.insert((x, y, z), 0);
                let out = ServerMessage::BlockChangeMsg { x, y, z, block_id: 0 };
                let _ = tx.send(serde_json::to_string(&out).unwrap());
            }
            ClientMessage::PlaceBlock { x, y, z, block_id } => {
                let mut gs = state.write().await;
                gs.block_changes.insert((x, y, z), block_id);
                let out = ServerMessage::BlockChangeMsg { x, y, z, block_id };
                let _ = tx.send(serde_json::to_string(&out).unwrap());
            }
        }
    }
}

#[cfg(feature = "server")]
#[tokio::main]
async fn main() {
    server_impl::run().await;
}

#[cfg(not(feature = "server"))]
fn main() {
    eprintln!("Build with --features server to run nodecraft-server");
}
