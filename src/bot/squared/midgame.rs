use std::time::{Duration, Instant};

use crate::{
    bot::{print_move_stats, print_search_header, print_total_stats, squared::bot::MIDGAME_DEPTH},
    othello::position::Position,
};

static MIN_MIDGAME_SCORE: isize = -64000;
static MAX_MIDGAME_SCORE: isize = 64000;

pub struct MidgameSearch {
    nodes: u64,
    position: Position,
}

impl MidgameSearch {
    pub fn new(position: Position) -> Self {
        Self { nodes: 0, position }
    }

    pub fn get_move(&mut self) -> usize {
        let children = self.position.children_with_index();
        let mut best_move = children.first().unwrap().0;
        let mut alpha = MIN_MIDGAME_SCORE;

        let mut total_nodes = 0;
        let mut total_duration = Duration::ZERO;

        print_search_header("SquaredBot", false, MIDGAME_DEPTH);
        for (i, (move_, child)) in children.iter().enumerate() {
            let start = Instant::now();
            let score = -self.negamax(child, MIDGAME_DEPTH - 1, -MAX_MIDGAME_SCORE, -alpha);
            let duration = start.elapsed();

            print_move_stats(self.nodes, i, children.len(), score, alpha, duration);
            total_nodes += self.nodes;
            total_duration += duration;
            self.nodes = 0;

            if score > alpha {
                alpha = score;
                best_move = *move_;
            }
        }

        print_total_stats(total_nodes, total_duration);

        best_move
    }

    fn negamax(&mut self, position: &Position, depth: u32, mut alpha: isize, beta: isize) -> isize {
        self.nodes += 1;

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
                return 1000 * position.final_score();
            }

            // Recursively evaluate after passing
            return -self.negamax(&passed_position, depth - 1, -beta, -alpha);
        }

        for child in &children {
            let score = -self.negamax(child, depth - 1, -beta, -alpha);
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
}
