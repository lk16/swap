use super::{board::Board, position::GameState};

pub struct Game {
    history: Vec<Board>,
    undone_moves: Vec<Board>, // For undo/redo
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
        }
    }

    /// Returns true if the undo was successful
    pub fn undo(&mut self, black_is_human: bool, white_is_human: bool) -> bool {
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
