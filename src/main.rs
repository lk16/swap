use axum::{extract::ws::WebSocketUpgrade, response::IntoResponse, routing::get, Router};
use frontend::websocket::handle_socket;
use std::net::SocketAddr;
use tower_http::services::ServeDir;

pub mod board;
pub mod frontend;
pub mod position;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .nest_service("/", ServeDir::new("src/frontend/static"));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}
