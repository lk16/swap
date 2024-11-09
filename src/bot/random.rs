use rand::rngs::ThreadRng;
use rand::RngCore;

use crate::othello::position::Position;

use super::Bot;

pub struct RandomBot;

impl Bot for RandomBot {
    // Returns the index of a random valid move
    fn get_move(&mut self, position: &Position) -> usize {
        let moves = position.get_moves();

        if moves == 0 {
            panic!("No moves available");
        }

        let move_count = moves.count_ones() as usize;
        let n = ThreadRng::default().next_u64() as usize % move_count;

        // Find the nth set bit by skipping n bits and getting the index of the next one
        let mut remaining = n;
        let mut current_moves = moves;

        while remaining > 0 {
            current_moves &= current_moves - 1; // Clear the lowest set bit
            remaining -= 1;
        }

        current_moves.trailing_zeros() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::othello::position::Position;

    #[test]
    fn test_random_bot_valid_moves() {
        let position = Position::new(); // Initial position position has 4 valid moves
        let mut bot = RandomBot;

        // Call get_move 10 times and verify each move is valid
        for _ in 0..10 {
            let selected_move = bot.get_move(&position);
            assert!(
                position.is_valid_move(selected_move),
                "Move {} was invalid! Valid moves: {:b}",
                selected_move,
                position.get_moves()
            );
        }
    }

    #[test]
    #[should_panic]
    fn test_random_bot_no_moves() {
        let position = Position::new_from_bitboards(0, 0); // Empty position has no moves
        let mut bot = RandomBot;

        bot.get_move(&position); // Should panic when there are no valid moves
    }
}
