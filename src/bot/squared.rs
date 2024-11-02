// This is inspired by my earlier project Squared, see http://github.com/lk16/squared

use crate::othello::position::Position;

use super::Bot;

pub struct SquaredBot;

static MIDGAME_DEPTH: u32 = 8;
static ENDGAME_DEPTH: u32 = 14;
static MAX_SCORE: isize = 64000;
static MIN_SCORE: isize = -64000;

impl Bot for SquaredBot {
    fn get_move(&self, position: &Position) -> usize {
        let moves = position.get_moves();

        if moves == 0 {
            panic!("No moves available");
        }

        if moves.count_ones() == 1 {
            return moves.trailing_zeros() as usize;
        }

        if position.count_empty() > ENDGAME_DEPTH {
            return Self::midgame_get_move(position);
        }

        Self::endgame_get_move(position)
    }
}

impl SquaredBot {
    fn midgame_get_move(position: &Position) -> usize {
        let children = position.children_with_index();

        let (mut best_move, first_child) = children.first().unwrap();
        let mut alpha =
            -Self::midgame_negamax(first_child, MIDGAME_DEPTH - 1, MIN_SCORE, MAX_SCORE);

        for (move_, child) in children.iter().skip(1) {
            let score = -Self::midgame_negamax(child, MIDGAME_DEPTH - 1, -MAX_SCORE, -alpha);

            if score > alpha {
                alpha = score;
                best_move = *move_;
            }
        }

        best_move
    }

    fn midgame_negamax(position: &Position, depth: u32, mut alpha: isize, beta: isize) -> isize {
        if depth == 0 {
            return Self::heuristic(position);
        }

        let children = position.children();

        // If no moves available
        if children.is_empty() {
            // Check if the game is finished
            let mut passed_position = *position;
            passed_position.pass();

            if passed_position.get_moves() == 0 {
                // Game is over, return final evaluation
                return position.final_score();
            }

            // Recursively evaluate after passing
            return -Self::midgame_negamax(&passed_position, depth - 1, -beta, -alpha);
        }

        for child in &children {
            let score = -Self::midgame_negamax(child, depth - 1, -beta, -alpha);
            alpha = alpha.max(score);

            if alpha >= beta {
                break; // Beta cutoff
            }
        }

        alpha
    }

    fn heuristic(position: &Position) -> isize {
        // Count corners for both players
        let corners = 0x8100000000000081u64; // Mask for corner positions
        let player_corners = (position.player & corners).count_ones() as isize;
        let opponent_corners = (position.opponent & corners).count_ones() as isize;

        let corner_diff = player_corners - opponent_corners;

        // Calculate move difference
        let player_moves = position.get_moves().count_ones() as isize;

        let mut opponent_position = *position;
        opponent_position.pass();
        let opponent_moves = opponent_position.get_moves().count_ones() as isize;

        let move_diff = player_moves - opponent_moves;

        // Final heuristic calculation
        (3 * corner_diff) + move_diff
    }

    // #5 bring from Edax, make this private again
    pub fn endgame_get_move(position: &Position) -> usize {
        let children = position.children_with_index();

        let (mut best_move, first_child) = children.first().unwrap();
        let mut alpha = -Self::endgame_negamax(first_child, MIN_SCORE, MAX_SCORE);

        for (move_, child) in children.iter().skip(1) {
            let score = -Self::endgame_negamax(child, -MAX_SCORE, -alpha);

            if score > alpha {
                alpha = score;
                best_move = *move_;
            }
        }

        best_move
    }

    fn endgame_negamax(position: &Position, mut alpha: isize, beta: isize) -> isize {
        let children = position.children();

        // If no moves available
        if children.is_empty() {
            // Check if the game is finished
            let mut passed_position = *position;
            passed_position.pass();

            if passed_position.get_moves() == 0 {
                // Game is over, return final evaluation
                return position.final_score();
            }

            // Recursively evaluate after passing
            return -Self::endgame_negamax(&passed_position, -beta, -alpha);
        }

        for child in &children {
            let score = -Self::endgame_negamax(child, -beta, -alpha);
            alpha = alpha.max(score);

            if alpha >= beta {
                break; // Beta cutoff
            }
        }

        alpha
    }
}
