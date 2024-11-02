use edax::bot::EdaxBot;
use random::RandomBot;
use squared::SquaredBot;

use crate::othello::position::Position;

pub mod edax;
pub mod random;
pub mod squared;

pub trait Bot: Send {
    // Returns the index of a valid move
    fn get_move(&self, position: &Position) -> usize;
}

pub fn get_bot(name: &str) -> Option<Box<dyn Bot>> {
    match name {
        "random" => Some(Box::new(RandomBot)),
        "squared" => Some(Box::new(SquaredBot)),
        "edax" => Some(Box::new(EdaxBot)),
        _ => None,
    }
}
