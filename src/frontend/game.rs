use crate::bot::{get_bot, Bot};
use crate::othello::board::Board;

/// A game of Othello as used by the websocket server.
/// We maintain a history of moves for undoing and redoing.
pub struct Game {
    /// The boards in the game history
    boards: Vec<Board>,

    /// The index of the current board in the `boards` vector
    offset: usize,

    /// Bot player for black and white. If a player is human, the bot is `None`.
    bots: [Option<Box<dyn Bot>>; 2],
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl Game {
    /// Create a new game with the starting board.
    pub fn new() -> Self {
        Self {
            boards: vec![Board::new()],
            offset: 0,
            bots: [None, None],
        }
    }

    /// Set the bot for a player.
    pub fn set_player(&mut self, color: usize, bot_name: &str) {
        let bot = if bot_name == "human" {
            None
        } else {
            get_bot(bot_name)
        };

        self.bots[color] = bot;
    }

    /// Get the bot for the current player.
    #[allow(clippy::borrowed_box)]
    pub fn get_current_bot(&mut self) -> Option<&mut Box<dyn Bot>> {
        let turn = self.current_board().turn;
        self.bots[turn].as_mut()
    }

    /// Check if a human is to move and has moves.
    fn has_human_turn(&self, board: &Board) -> bool {
        self.bots[board.turn].is_none() && board.has_moves()
    }

    /// Undo a move. Returns true if a move was undone.
    pub fn undo(&mut self) -> bool {
        let mut moves_to_undo: Option<usize> = None;

        for i in (0..self.offset).rev() {
            if self.has_human_turn(&self.boards[i]) {
                moves_to_undo = Some(self.offset - i);
                break;
            }
        }

        // If no human turn is found, don't undo
        let Some(moves_to_undo) = moves_to_undo else {
            return false;
        };

        // Effectively undo the moves
        self.offset -= moves_to_undo;

        true
    }

    /// Redo a move. Returns true if a move was redone.
    pub fn redo(&mut self) -> bool {
        for i in self.offset + 1..self.boards.len() {
            if self.has_human_turn(&self.boards[i]) {
                self.offset = i;
                return true;
            }
        }

        false
    }

    /// Make a move.
    pub fn do_move(&mut self, move_index: usize) {
        let mut board = self.current_board().do_move_cloned(move_index);

        // Prevent redo now that a move has been made
        self.boards.truncate(self.offset + 1);

        self.boards.push(board);
        self.offset += 1;

        if board.has_to_pass() {
            board.pass();
            self.boards.push(board);
            self.offset += 1;
        }
    }

    /// Reset the game to a new board.
    pub fn reset(&mut self, board: Board) {
        self.boards = vec![board];
        self.offset = 0;
    }

    /// Get the current board.
    pub fn current_board(&self) -> &Board {
        &self.boards[self.offset]
    }
}

#[cfg(test)]
mod tests {
    use crate::othello::board::BLACK;

    use super::*;

    #[test]
    fn test_new() {
        let game = Game::new();
        assert_eq!(game.boards.len(), 1);
        assert_eq!(game.offset, 0);
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
        assert_eq!(game.boards.len(), 3);
        assert_eq!(game.offset, 2);

        // Test undo
        assert!(game.undo());
        assert_eq!(game.boards.len(), 3);
        assert_eq!(game.offset, 0);

        // Test redo
        assert!(game.redo());
        assert_eq!(game.boards.len(), 3);
        assert_eq!(game.offset, 2);

        // Test redo with no moves to redo
        assert!(!game.redo());
    }

    #[test]
    fn test_do_move() {
        let mut game = Game::new();

        // Test normal move
        game.do_move(19);
        assert_eq!(game.boards.len(), 2);
        assert_eq!(game.offset, 1);

        // Test move that forces a pass
        // Position: | - ● ○ ● -     |
        game.reset(Board::new_from_bitboards(0x4, 0xA, BLACK));
        game.do_move(0);
        assert_eq!(game.boards.len(), 3);
        assert_eq!(game.offset, 2);

        // Test move that ends the game
        // Position: | - ● ○         |
        game.reset(Board::new_from_bitboards(0x4, 0x2, BLACK));
        game.do_move(0);
        assert_eq!(game.boards.len(), 2);
        assert_eq!(game.offset, 1);
    }

    #[test]
    fn test_reset() {
        let mut game = Game::new();

        // Make some moves
        game.do_move(19);
        game.do_move(26);
        assert_eq!(game.boards.len(), 3);
        assert_eq!(game.offset, 2);

        // Reset with new board
        let new_board = Board::new();
        game.reset(new_board);
        assert_eq!(game.boards.len(), 1);
        assert_eq!(game.offset, 0);
    }

    #[test]
    fn test_current_board() {
        let mut game = Game::new();
        let initial_board = *game.current_board();

        game.do_move(19);
        assert_ne!(game.current_board(), &initial_board);
    }
}
