use std::fmt::{self, Display};

use crate::othello::board::{Board, State};
use axum::extract::ws::{Message, WebSocket};
use serde_json::Value;

enum HandlerError {
    WebSocketError(axum::Error),
    UnexpectedMessage(Message),
    InvalidJson(serde_json::Error),
    NonObjectJson(String),
    MultipleKeyJson(String),
    MissingKeyJson(String),
    HandlerValueError((String, String), String),
    UnknownCommand((String, String)),
}

use HandlerError::*;

impl Display for HandlerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WebSocketError(e) => write!(f, "WebSocket error: {}", e),
            Self::UnexpectedMessage(m) => write!(f, "Unexpected message: {:?}", m),
            Self::InvalidJson(e) => write!(f, "Invalid JSON: {}", e),
            Self::NonObjectJson(text) => write!(f, "Non-object JSON: {}", text),
            Self::MultipleKeyJson(text) => write!(f, "Multiple keys in JSON: {}", text),
            Self::MissingKeyJson(text) => write!(f, "Missing key in JSON: {}", text),
            Self::HandlerValueError((command, value), e) => {
                write!(
                    f,
                    "Error in handler for {} with value {}: {}",
                    command, value, e
                )
            }
            Self::UnknownCommand((command, value)) => {
                write!(f, "Unknown command {} with value {}", command, value)
            }
        }
    }
}

struct GameSession {
    socket: WebSocket,
    history: Vec<Board>,
    undone_moves: Vec<Board>, // For undo/redo
}

impl GameSession {
    async fn new(socket: WebSocket) -> Self {
        Self {
            socket,
            history: vec![Board::new()],
            undone_moves: vec![],
        }
    }

    async fn run(&mut self) -> Result<(), axum::Error> {
        self.send_current_board().await?;

        while let Some(msg) = self.socket.recv().await {
            if let Err(e) = self.handle_message(msg).await {
                match e {
                    WebSocketError(e) => return Err(e),
                    _ => eprintln!("{}", e),
                }
            }
        }

        Ok(())
    }

    async fn send_current_board(&mut self) -> Result<(), axum::Error> {
        let message = self.current_board().as_ws_message();
        self.socket.send(Message::Text(message)).await
    }

    async fn handle_message(
        &mut self,
        msg: Result<Message, axum::Error>,
    ) -> Result<(), HandlerError> {
        let text = match msg {
            Ok(Message::Text(text)) => text,
            Ok(Message::Close(_)) => return Ok(()),
            Ok(other) => return Err(UnexpectedMessage(other)),
            Err(e) => return Err(WebSocketError(e)),
        };

        let parsed: Value = serde_json::from_str(&text).map_err(InvalidJson)?;
        let object = parsed.as_object().ok_or(NonObjectJson(text.clone()))?;

        if object.len() > 1 {
            return Err(MultipleKeyJson(text));
        }

        let (command, data) = object.iter().next().ok_or(MissingKeyJson(text))?;

        match (command.as_str(), data) {
            ("undo", data) => self.handle_undo((command, data)).await,
            ("redo", data) => self.handle_redo((command, data)).await,
            ("do_move", data) => self.handle_do_move((command, data)).await,
            ("new_game", data) => self.handle_new_game((command, data)).await,
            ("xot_game", data) => self.handle_xot_game((command, data)).await,
            _ => Err(UnknownCommand((command.clone(), data.to_string()))),
        }
    }

    async fn handle_undo(&mut self, _: (&String, &Value)) -> Result<(), HandlerError> {
        if self.history.len() > 1 {
            let undone_move = self.history.pop().unwrap();
            self.undone_moves.push(undone_move);
        }

        self.send_current_board().await.map_err(WebSocketError)
    }

    async fn handle_redo(&mut self, _: (&String, &Value)) -> Result<(), HandlerError> {
        if let Some(redone_move) = self.undone_moves.pop() {
            self.history.push(redone_move);
        }

        self.send_current_board().await.map_err(WebSocketError)
    }

    async fn handle_do_move(
        &mut self,
        (key, value): (&String, &Value),
    ) -> Result<(), HandlerError> {
        let index = match value.as_u64() {
            Some(index) => index as usize,
            None => {
                return Err(HandlerValueError(
                    (key.clone(), value.to_string()),
                    "index is not a number".to_string(),
                ))
            }
        };

        if !self.current_board().is_valid_move(index) {
            // TODO prevent sending invalid moves from client
            return Err(HandlerValueError(
                (key.clone(), value.to_string()),
                "invalid move".to_string(),
            ));
        }

        self.undone_moves.clear();

        let mut board = *self.current_board();
        board.do_move(index);

        match board.as_state() {
            State::HasMoves(board) => self.history.push(board),
            State::Finished(board) => self.history.push(board),
            State::Passed(board) => self.history.push(board),
        }

        self.send_current_board().await.map_err(WebSocketError)
    }

    async fn handle_new_game(&mut self, _: (&String, &Value)) -> Result<(), HandlerError> {
        self.history = vec![Board::new()];
        self.undone_moves.clear();
        self.send_current_board().await.map_err(WebSocketError)
    }

    async fn handle_xot_game(&mut self, _: (&String, &Value)) -> Result<(), HandlerError> {
        self.history = vec![Board::new_xot()];
        self.undone_moves.clear();
        self.send_current_board().await.map_err(WebSocketError)
    }

    fn current_board(&self) -> &Board {
        self.history.last().unwrap()
    }
}

pub async fn handle_socket(socket: WebSocket) {
    let mut session = GameSession::new(socket).await;

    if let Err(err) = session.run().await {
        eprintln!("WS error: {}", err);
    }
}

// TODO add tests
