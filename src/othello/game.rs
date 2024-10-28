use super::{board::Board, position::GameState};
use crate::bot::{get_bot, Bot};

pub struct Game {
    history: Vec<Board>,
    undone_moves: Vec<Board>, // For undo/redo
    black_bot: Option<Box<dyn Bot>>,
    white_bot: Option<Box<dyn Bot>>,
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl Game {
    pub fn new() -> Self {
        Self {
            history: vec![Board::new()],
            undone_moves: vec![],
            black_bot: None,
            white_bot: None,
        }
    }

    pub fn set_black_player(&mut self, bot_name: &str) {
        self.black_bot = if bot_name == "human" {
            None
        } else {
            get_bot(bot_name)
        };
    }

    pub fn set_white_player(&mut self, bot_name: &str) {
        self.white_bot = if bot_name == "human" {
            None
        } else {
            get_bot(bot_name)
        };
    }

    #[allow(clippy::borrowed_box)]
    pub fn get_current_bot(&self) -> Option<&Box<dyn Bot>> {
        if self.current_board().black_to_move {
            self.black_bot.as_ref()
        } else {
            self.white_bot.as_ref()
        }
    }

    /// Returns true if the undo was successful
    pub fn undo(&mut self) -> bool {
        let black_is_human = self.black_bot.is_none();
        let white_is_human = self.white_bot.is_none();

        // If both players are bots, don't undo
        if !black_is_human && !white_is_human {
            return false;
        }

        // Find the index of the last human turn in the history
        let last_human_turn = self
            .history
            .iter()
            .rev()
            .skip(1) // Skip current position
            .position(|board| {
                // TODO factor out
                ((board.black_to_move && black_is_human)
                    || (!board.black_to_move && white_is_human))
                    && board.game_state() != GameState::Passed
            });

        // If no human turn is found, don't undo
        let Some(last_human_index) = last_human_turn else {
            return false;
        };

        // Convert the reverse index to a forward index
        let last_human_index = self.history.len() - 2 - last_human_index;

        // Undo moves until we reach the last human turn
        while self.history.len() > last_human_index + 1 {
            let undone_move = self.history.pop().unwrap();
            self.undone_moves.push(undone_move);
        }

        true
    }

    pub fn redo(&mut self) -> bool {
        if let Some(redone_move) = self.undone_moves.pop() {
            self.history.push(redone_move);
            return true;
        }

        false
    }

    pub fn do_move(&mut self, move_index: usize) {
        let mut board = self.current_board().do_move_cloned(move_index);
        self.history.push(board);

        if board.game_state() == GameState::Passed {
            board.pass();
            self.history.push(board);
        }

        self.undone_moves.clear();
    }

    pub fn reset(&mut self, board: Board) {
        self.history = vec![board];
        self.undone_moves.clear();
    }

    pub fn current_board(&self) -> &Board {
        self.history.last().unwrap()
    }
}

// TODO add tests
