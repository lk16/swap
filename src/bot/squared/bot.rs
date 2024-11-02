// This is inspired by my earlier project Squared, see http://github.com/lk16/squared

use super::endgame::EndgameSearch;
use super::midgame::MidgameSearch;
use crate::othello::position::Position;

use crate::bot::Bot;

pub struct SquaredBot;

pub static MIDGAME_DEPTH: u32 = 12;
pub static ENDGAME_DEPTH: u32 = 14;

impl Bot for SquaredBot {
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
