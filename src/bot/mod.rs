use random::RandomBot;

use crate::othello::board::Board;

pub mod random;

pub trait Bot: Send {
    // Returns the index of a valid move
    fn get_move(&self, board: &Board) -> usize;
}

pub fn get_bot(name: &str) -> Option<Box<dyn Bot>> {
    match name {
        "random" => Some(Box::new(RandomBot)),
        _ => None,
    }
}
