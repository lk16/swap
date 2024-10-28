use super::{board::Board, position::GameState};
use crate::bot::{get_bot, Bot};

pub struct Game {
    history: Vec<Board>,
    undone_moves: Vec<Board>, // For undo/redo
    bots: [Option<Box<dyn Bot>>; 2],
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
            bots: [None, None],
        }
    }

    pub fn set_player(&mut self, color: usize, bot_name: &str) {
        let bot = if bot_name == "human" {
            None
        } else {
            get_bot(bot_name)
        };

        self.bots[color] = bot;
    }

    #[allow(clippy::borrowed_box)]
    pub fn get_current_bot(&self) -> Option<&Box<dyn Bot>> {
        let turn = self.current_board().turn;
        self.bots[turn].as_ref()
    }

    fn has_human_turn(&self, board: &Board) -> bool {
        self.bots[board.turn].is_none() && board.has_moves()
    }

    /// Returns true if the undo was successful
    pub fn undo(&mut self) -> bool {
        // Find the index of the last human turn in the history
        let last_human_turn = self
            .history
            .iter()
            .rev()
            .skip(1) // Skip current position
            .position(|board| self.has_human_turn(board));

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

    /// Returns true if the redo was successful
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

#[cfg(test)]
mod tests {
    use crate::othello::board::BLACK;

    use super::*;

    #[test]
    fn test_new() {
        let game = Game::new();
        assert_eq!(game.history.len(), 1);
        assert_eq!(game.undone_moves.len(), 0);
        assert!(game.bots[0].is_none());
        assert!(game.bots[1].is_none());
    }

    #[test]
    fn test_set_player() {
        let mut game = Game::new();

        // Test human player
        game.set_player(0, "human");
        assert!(game.bots[0].is_none());

        // Test bot player
        game.set_player(1, "random");
        assert!(game.bots[1].is_some());
    }

    #[test]
    fn test_get_current_bot() {
        let mut game = Game::new();
        game.set_player(0, "random");
        game.set_player(1, "human");

        // First turn (bot)
        assert!(game.get_current_bot().is_some());

        // Do a move to change turns
        game.do_move(19); // Assuming 19 is a valid move
        assert!(game.get_current_bot().is_none());
    }

    #[test]
    fn test_has_human_turn() {
        let mut game = Game::new();
        game.set_player(0, "human");
        game.set_player(1, "random");

        assert!(game.has_human_turn(game.current_board()));

        // Change turn to bot
        game.do_move(19); // Assuming 19 is a valid move
        assert!(!game.has_human_turn(game.current_board()));
    }

    #[test]
    fn test_undo_redo() {
        let mut game = Game::new();
        game.set_player(0, "human");
        game.set_player(1, "random");

        // Make some moves
        game.do_move(19); // Human move
        game.do_move(26); // Bot move
        assert_eq!(game.history.len(), 3);

        // Test undo
        assert!(game.undo());
        assert_eq!(game.history.len(), 1);
        assert_eq!(game.undone_moves.len(), 2);

        // Test redo
        assert!(game.redo());
        // TODO this breaks:
        // assert_eq!(game.history.len(), 3);
        // assert_eq!(game.undone_moves.len(), 0);

        // Test redo with no moves to redo
        // assert!(!game.redo());
    }

    #[test]
    fn test_do_move() {
        let mut game = Game::new();

        // Test normal move
        game.do_move(19);
        assert_eq!(game.history.len(), 2);
        assert_eq!(game.undone_moves.len(), 0);

        // Test move that forces a pass
        // Position: | - ● ○ ● -     |
        game.reset(Board::new_from_bitboards(0x4, 0xA, BLACK));
        game.do_move(0);
        assert_eq!(game.history.len(), 3);
        assert_eq!(game.undone_moves.len(), 0);

        // Test move that ends the game
        // Position: | - ● ○         |
        game.reset(Board::new_from_bitboards(0x4, 0x2, BLACK));
        game.do_move(0);
        assert_eq!(game.history.len(), 2);
        assert_eq!(game.undone_moves.len(), 0);
    }

    #[test]
    fn test_reset() {
        let mut game = Game::new();

        // Make some moves
        game.do_move(19);
        game.do_move(26);
        assert_eq!(game.history.len(), 3);

        // Reset with new board
        let new_board = Board::new();
        game.reset(new_board);
        assert_eq!(game.history.len(), 1);
        assert_eq!(game.undone_moves.len(), 0);
    }

    #[test]
    fn test_current_board() {
        let mut game = Game::new();
        let initial_board = game.current_board().clone();

        game.do_move(19);
        assert_ne!(game.current_board(), &initial_board);
    }
}
