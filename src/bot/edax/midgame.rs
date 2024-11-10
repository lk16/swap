use std::time::{Duration, Instant};

use crate::bot::{print_move_stats, print_search_header, print_total_stats};
use crate::othello::position::Position;

use super::bot::MIDGAME_DEPTH;
use super::eval::Eval;

pub const SCORE_MIN: i32 = -64;
pub const SCORE_MAX: i32 = 64;

pub struct MidgameSearch {
    // Contains search root position, does not change during search
    root: Position,

    // Contains search state, changes during search
    eval: Eval,

    // Number of nodes visited in the current search
    nodes: u64,
}

impl MidgameSearch {
    pub fn new(position: Position) -> Self {
        // TODO This should match search_setup() in Edax

        Self {
            root: position,
            eval: Eval::default(),
            nodes: 0,
        }
    }

    pub fn get_move(&mut self) -> usize {
        let children = self.root.children_with_index();
        let mut best_move = children.first().unwrap().0;
        let mut alpha = SCORE_MIN;

        let mut total_nodes = 0;
        let mut total_duration = Duration::ZERO;

        print_search_header("EdaxBot", false, MIDGAME_DEPTH);
        for (i, (move_, child)) in children.iter().enumerate() {
            let start = Instant::now();

            self.eval = Eval::new(child);

            // TODO replace by PVS_midgame() from Edax
            let score = -self.negamax(MIDGAME_DEPTH - 1, -SCORE_MAX, -alpha);
            let duration = start.elapsed();

            print_move_stats(
                self.nodes,
                i,
                children.len(),
                score as isize,
                alpha as isize,
                duration,
            );
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

    fn negamax(&mut self, depth: u32, mut alpha: i32, beta: i32) -> i32 {
        self.nodes += 1;
        if depth == 0 {
            return self.eval.heuristic();
        }

        let moves = self.eval.position().iter_move_indices();

        // If no moves available
        if moves.is_empty() {
            self.eval.pass();

            // Check if the game is finished
            if !self.eval.position().has_moves() {
                // Game is over, return final evaluation
                let score = self.eval.position().final_score() as i32;
                self.eval.pass();
                return score;
            }

            // Recursively evaluate after passing
            let score = -self.negamax(depth - 1, -beta, -alpha);
            self.eval.pass();
            return score;
        }

        for move_ in moves {
            let flipped = self.eval.do_move(move_);
            let score = -self.negamax(depth - 1, -beta, -alpha);
            self.eval.undo_move(move_, flipped);

            alpha = alpha.max(score);

            if alpha >= beta {
                break; // Beta cutoff
            }
        }

        alpha
    }
}

#[cfg(test)]
mod tests {
    // TODO bring back NaiveMidgameSearch
}
