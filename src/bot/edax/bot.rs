use crate::othello::position::Position;

use crate::bot::Bot;

use super::r#const::BLACK;
use super::search::Search;

pub struct EdaxBot;

pub const EDAX_LEVEL: i32 = 6;

/// EdaxBot is a bot that the Edax engine to get moves.
impl Bot for EdaxBot {
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
