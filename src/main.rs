use axum::response::Html;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Router,
};
use board::Board;
use std::fs;
use std::net::SocketAddr;

pub mod board;
pub mod position;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(serve_index))
        .route("/ws", get(ws_handler));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn serve_index() -> impl IntoResponse {
    let html_content = fs::read_to_string("static/index.html").expect("Failed to read index.html");
    Html(html_content)
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    let mut history = vec![Board::new()];

    // Send initial board state
    let initial_state = history.last().unwrap().as_ws_message();
    if socket.send(Message::Text(initial_state)).await.is_err() {
        return;
    }

    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            if let Message::Text(text) = msg {
                let mut board = history.last().unwrap().clone();
                if text == "undo" {
                    if history.len() > 1 {
                        history.pop();
                        board = history.last().unwrap().clone();
                    }
                } else if let Ok(index) = text.parse::<usize>() {
                    if index < 64 {
                        // Check if the move is valid before applying it
                        let valid_moves = board.get_moves();
                        if valid_moves & (1u64 << index) != 0 {
                            board.do_move(index);
                            history.push(board.clone());
                        }
                    }
                }
                let response = board.as_ws_message();
                if socket.send(Message::Text(response)).await.is_err() {
                    break;
                }
            }
        } else {
            break;
        }
    }
}
