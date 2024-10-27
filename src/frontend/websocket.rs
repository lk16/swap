use std::fmt::{self, Display};
use std::time::Duration;

use crate::bot::get_bot;
use crate::othello::game::Game;
use crate::{bot::Bot, othello::board::Board};
use axum::extract::ws::{Message, WebSocket};
use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
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

use tokio::time::sleep;
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
    ws_sender: SplitSink<WebSocket, Message>,
    ws_receiver: SplitStream<WebSocket>,

    game: Game,
    black_bot: Option<Box<dyn Bot>>,
    white_bot: Option<Box<dyn Bot>>,
}

impl GameSession {
    fn new(ws_sender: SplitSink<WebSocket, Message>, ws_receiver: SplitStream<WebSocket>) -> Self {
        Self {
            ws_sender,
            ws_receiver,
            game: Game::new(),
            black_bot: None,
            white_bot: None,
        }
    }

    async fn run(&mut self) -> Result<(), axum::Error> {
        self.send_current_board().await?;

        while let Some(msg) = self.ws_receiver.next().await {
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
        self.ws_sender.send(Message::Text(message)).await
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
            ("human_move", data) => self.handle_human_move((command, data)).await,
            ("new_game", data) => self.handle_new_game((command, data)).await,
            ("xot_game", data) => self.handle_xot_game((command, data)).await,
            ("set_black_player", data) => self.handle_set_black_player((command, data)).await,
            ("set_white_player", data) => self.handle_set_white_player((command, data)).await,
            _ => Err(UnknownCommand((command.clone(), data.to_string()))),
        }
    }

    async fn handle_undo(&mut self, _: (&String, &Value)) -> Result<(), HandlerError> {
        let black_is_human = self.black_bot.is_none();
        let white_is_human = self.white_bot.is_none();

        if !self.game.undo(black_is_human, white_is_human) {
            return Ok(());
        }

        self.send_current_board().await.map_err(WebSocketError)
    }

    async fn handle_redo(&mut self, _: (&String, &Value)) -> Result<(), HandlerError> {
        if !self.game.redo() {
            return Ok(());
        }

        self.send_current_board().await.map_err(WebSocketError)
    }

    async fn handle_human_move(
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
            return Err(HandlerValueError(
                (key.clone(), value.to_string()),
                "invalid move".to_string(),
            ));
        }

        self.game.do_move(index);

        self.send_current_board().await.map_err(WebSocketError)?;
        self.do_bot_move().await
    }

    async fn handle_new_game(&mut self, _: (&String, &Value)) -> Result<(), HandlerError> {
        self.game.reset(Board::new());
        self.send_current_board().await.map_err(WebSocketError)?;
        self.do_bot_move().await
    }

    async fn handle_xot_game(&mut self, _: (&String, &Value)) -> Result<(), HandlerError> {
        self.game.reset(Board::new_xot());
        self.send_current_board().await.map_err(WebSocketError)?;
        self.do_bot_move().await
    }

    async fn handle_set_black_player(
        &mut self,
        (key, value): (&String, &Value),
    ) -> Result<(), HandlerError> {
        let name = value.as_str().unwrap();

        if name == "human" {
            self.black_bot = None;
            return Ok(());
        }

        let Some(bot) = get_bot(name) else {
            return Err(HandlerValueError(
                (key.clone(), value.to_string()),
                "unknown bot".to_string(),
            ));
        };

        self.black_bot = Some(bot);
        self.do_bot_move().await?;
        Ok(())
    }

    async fn handle_set_white_player(
        &mut self,
        (key, value): (&String, &Value),
    ) -> Result<(), HandlerError> {
        let name = value.as_str().unwrap();

        if name == "human" {
            self.white_bot = None;
            return Ok(());
        }

        let Some(bot) = get_bot(name) else {
            return Err(HandlerValueError(
                (key.clone(), value.to_string()),
                "unknown bot".to_string(),
            ));
        };

        self.white_bot = Some(bot);
        self.do_bot_move().await?;
        Ok(())
    }

    async fn do_bot_move(&mut self) -> Result<(), HandlerError> {
        // Loop because a bot may have to move again
        loop {
            let bot = if self.current_board().black_to_move {
                self.black_bot.as_ref()
            } else {
                self.white_bot.as_ref()
            };

            let Some(bot) = bot else {
                return Ok(()); // No bot set for player to move
            };

            let board = self.current_board();

            if !board.has_moves() {
                return Ok(()); // Bot can't move
            }

            let move_index = bot.get_move(&board.position);
            self.game.do_move(move_index);

            self.send_current_board().await.map_err(WebSocketError)?;

            // Sleep to allow human player to see the move
            sleep(Duration::from_millis(100)).await;
        }
    }

    fn current_board(&self) -> &Board {
        self.game.current_board()
    }
}

pub async fn handle_socket(socket: WebSocket) {
    // split socket to facilitate testing
    let (ws_sender, ws_receiver) = socket.split();

    let mut session = GameSession::new(ws_sender, ws_receiver);

    if let Err(err) = session.run().await {
        eprintln!("WS error: {}", err);
    }
}
