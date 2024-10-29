use crate::othello::position::Position;

use crate::bot::Bot;

pub struct EdaxBot;

impl Bot for EdaxBot {
    // Returns the index of a random valid move
    fn get_move(&self, _position: &Position) -> usize {
        todo!() // TODO implement EdaxBot
    }
}
