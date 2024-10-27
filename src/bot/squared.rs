// This is inspired by my earlier project Squared, see http://github.com/lk16/squared

use crate::othello::board::Board;

use super::Bot;

pub struct SquaredBot;

static DEPTH: i32 = 6;
static MAX_SCORE: isize = 64000;
static MIN_SCORE: isize = -64000;

impl Bot for SquaredBot {
    fn get_move(&self, board: &Board) -> usize {
        let moves = board.get_moves();

        if moves.count_ones() == 1 {
            // If there is only one move, take it
            return moves.trailing_zeros() as usize;
        }

        // TODO don't use option
        let mut best_move = None;
        let mut best_score = MIN_SCORE;

        // Try each possible move
        for i in 0..64 {
            if board.is_valid_move(i) {
                let mut child = *board;
                child.do_move(i);

                let score = -Self::negamax(&child, DEPTH - 1, MIN_SCORE, MAX_SCORE);

                if score > best_score {
                    best_score = score;
                    best_move = Some(i);
                }
            }
        }

        best_move.expect("There should be at least one valid move")
    }
}

impl SquaredBot {
    fn heuristic(board: &Board) -> isize {
        // Count corners for both players
        let corners = 0x8100000000000081u64; // Mask for corner positions
        let player_corners = (board.position.player & corners).count_ones() as isize;
        let opponent_corners = (board.position.opponent & corners).count_ones() as isize;

        let corner_diff = player_corners - opponent_corners;

        // Calculate move difference
        let my_moves = board.get_moves().count_ones() as isize;

        let mut opponent_board = *board;
        opponent_board.pass();
        let opp_moves = opponent_board.get_moves().count_ones() as isize;

        let move_diff = my_moves - opp_moves;

        // Final heuristic calculation
        (3 * corner_diff) + move_diff
    }

    fn negamax(board: &Board, depth: i32, mut alpha: isize, beta: isize) -> isize {
        if depth == 0 {
            return Self::heuristic(board);
        }

        let moves = board.get_moves();

        // If no moves available
        if moves == 0 {
            // Check if the game is finished
            let mut passed_board = *board;
            passed_board.pass();

            if passed_board.get_moves() == 0 {
                // Game is over, return final evaluation
                return Self::heuristic(board);
            }

            // Recursively evaluate after passing
            return -Self::negamax(&passed_board, depth - 1, -beta, -alpha);
        }

        let mut best_score = isize::MIN;

        for i in 0..64 {
            if board.is_valid_move(i) {
                let mut child = *board;
                child.do_move(i);

                let score = -Self::negamax(&child, depth - 1, -beta, -alpha);
                best_score = best_score.max(score);
                alpha = alpha.max(score);

                if alpha >= beta {
                    break; // Beta cutoff
                }
            }
        }

        best_score
    }
}
