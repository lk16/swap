use crate::bot::squared::endgame::EndgameSearch;
use crate::othello::position::Position;

use crate::bot::Bot;

use super::midgame::MidgameSearch;

pub struct EdaxBot;

pub const MIDGAME_DEPTH: u32 = 8;
pub const ENDGAME_DEPTH: u32 = 14;

impl Bot for EdaxBot {
    fn get_move(&mut self, position: &Position) -> usize {
        let moves = position.get_moves();

        if moves == 0 {
            panic!("No moves available");
        }

        if moves.count_ones() == 1 {
            return moves.trailing_zeros() as usize;
        }

        if position.count_empty() > ENDGAME_DEPTH {
            let mut search = MidgameSearch::new(*position);
            return search.get_move();
        }

        let mut search = EndgameSearch::new(position);
        search.get_move()
    }
}
