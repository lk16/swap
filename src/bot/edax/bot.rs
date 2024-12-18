use crate::othello::position::Position;

use crate::bot::Bot;

use super::r#const::BLACK;
use super::search::Search;

/// EdaxBot is a bot that uses the Edax engine to get moves.
pub struct EdaxBot;

pub const EDAX_LEVEL: i32 = 6;

impl Bot for EdaxBot {
    /// Get the best move from the current position.
    fn get_move(&mut self, position: &Position) -> usize {
        let moves = position.get_moves();

        if moves == 0 {
            panic!("No moves available");
        }

        if moves.count_ones() == 1 {
            return moves.trailing_zeros() as usize;
        }

        let mut search = Search::new(position, BLACK, EDAX_LEVEL);
        let result = search.run();
        result.move_
    }
}
