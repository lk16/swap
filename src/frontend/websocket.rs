use crate::board::Board;
use axum::extract::ws::{Message, WebSocket};

pub async fn handle_socket(mut socket: WebSocket) {
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
