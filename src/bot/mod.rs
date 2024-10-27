use random::RandomBot;
use squared::SquaredBot;

use crate::othello::board::Board;

pub mod random;
pub mod squared;

pub trait Bot: Send {
    // Returns the index of a valid move
    fn get_move(&self, board: &Board) -> usize;
}

pub fn get_bot(name: &str) -> Option<Box<dyn Bot>> {
    match name {
        "random" => Some(Box::new(RandomBot)),
        "squared" => Some(Box::new(SquaredBot)),
        _ => None,
    }
}
