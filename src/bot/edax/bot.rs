use crate::othello::position::Position;

use crate::bot::Bot;

use super::r#const::BLACK;
use super::search::Search;

pub struct EdaxBot;

pub const MIDGAME_DEPTH: u32 = 10;
pub const ENDGAME_DEPTH: u32 = 18;

impl Bot for EdaxBot {
    fn get_move(&mut self, position: &Position) -> usize {
        let moves = position.get_moves();

        if moves == 0 {
            panic!("No moves available");
        }

        if moves.count_ones() == 1 {
            return moves.trailing_zeros() as usize;
        }

        // TODO confirm player == BLACK is correct
        let mut search = Search::new(position, BLACK);

        // TODO confirm level == 6 is correct
        search.set_level(6, search.n_empties);

        search.run();

        search.result.move_
    }
}
