use std::time::{Duration, Instant};

use crate::bot::{print_move_stats, print_search_header, print_total_stats};
use crate::othello::position::Position;

use super::bot::MIDGAME_DEPTH;
use super::eval::{Eval, EVAL_N_FEATURES};
use super::weights::EVAL_WEIGHT;

const SCORE_MIN: i32 = -64;
const SCORE_MAX: i32 = 64;

pub struct MidgameSearch {
    position: Position,
    eval: Eval,
    n_empties: u32,
    nodes: u64,
}

impl MidgameSearch {
    // TODO #6 bring better midgame search from Edax

    pub fn new(position: Position) -> Self {
        Self {
            position,
            eval: Eval::new(&position),
            n_empties: position.count_empty(),
            nodes: 0,
        }
    }

    fn do_move(&mut self, move_: usize) -> u64 {
        let flipped = self.position.do_move(move_);
        self.eval.update(move_, flipped);
        self.n_empties -= 1;
        flipped
    }

    fn undo_move(&mut self, move_: usize, flipped: u64) {
        self.position.undo_move(move_, flipped);
        self.eval.restore(move_, flipped);
        self.n_empties += 1;
    }

    fn pass(&mut self) {
        self.position.pass();
        self.eval.pass();
    }

    pub fn get_move(&mut self) -> usize {
        let children = self.position.children_with_index();
        let mut best_move = children.first().unwrap().0;
        let mut alpha = SCORE_MIN;

        let mut total_nodes = 0;
        let mut total_duration = Duration::ZERO;

        print_search_header("EdaxBot", false, MIDGAME_DEPTH);
        for (i, (move_, child)) in children.iter().enumerate() {
            let start = Instant::now();

            // TODO make function
            self.eval = Eval::new(child);
            self.n_empties = child.count_empty();
            self.position = *child;

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
            return self.heuristic();
        }

        let mut remaining_moves = self.position.get_moves();

        // If no moves available
        if remaining_moves == 0 {
            // Check if the game is finished
            self.pass();

            if self.position.get_moves() == 0 {
                // Game is over, return final evaluation
                let score = self.position.final_score() as i32;
                self.pass();
                return score;
            }

            // Recursively evaluate after passing
            let score = -self.negamax(depth - 1, -beta, -alpha);
            self.pass();
            return score;
        }

        while remaining_moves != 0 {
            let move_ = remaining_moves.trailing_zeros() as usize;

            let flipped = self.do_move(move_);
            let score = -self.negamax(depth - 1, -beta, -alpha);
            self.undo_move(move_, flipped);

            alpha = alpha.max(score);

            if alpha >= beta {
                break; // Beta cutoff
            }

            remaining_moves &= remaining_moves - 1;
        }

        alpha
    }

    fn heuristic(&self) -> i32 {
        let player_index = self.eval.player as usize;
        let empty_index = (60 - self.n_empties) as usize;

        let w = &EVAL_WEIGHT[player_index][empty_index];
        let f = &self.eval.features;

        let mut score = 0;
        for i in 0..EVAL_N_FEATURES {
            score += w[f[i] as usize] as i32;
        }

        if score > 0 {
            score += 64;
        } else {
            score -= 64;
        }
        score /= 128;

        if score <= SCORE_MIN {
            score = SCORE_MIN + 1;
        } else if score >= SCORE_MAX {
            score = SCORE_MAX - 1;
        }

        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_midgame_search() {
        let position = Position::new();
        let search = MidgameSearch::new(position);
        assert_eq!(search.n_empties, 60);
    }

    #[test]
    fn test_do_and_undo_move() {
        let position = Position::new();
        let mut search = MidgameSearch::new(position);
        let initial_empties = search.n_empties;
        let initial_board = search.position.clone();

        // Do move
        let move_ = 19; // Valid move for initial position (D3)
        let flipped = search.do_move(move_);
        assert_eq!(search.n_empties, initial_empties - 1);

        // Undo move
        search.undo_move(move_, flipped);
        assert_eq!(search.n_empties, initial_empties);
        assert_eq!(search.position, initial_board);
    }

    #[test]
    fn test_get_move_returns_valid_move() {
        let position = Position::new();
        let mut search = MidgameSearch::new(position);
        let best_move = search.get_move();

        // Check if returned move is valid (one of the four possible initial moves)
        let valid_initial_moves = vec![19, 26, 37, 44];
        assert!(valid_initial_moves.contains(&best_move));
    }

    #[test]
    fn test_heuristic_bounds() {
        let position = Position::new();
        let search = MidgameSearch::new(position);
        let score = search.heuristic();

        assert!(score > SCORE_MIN);
        assert!(score < SCORE_MAX);
    }
}
