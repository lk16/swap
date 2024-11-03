use std::time::{Duration, Instant};

use crate::{
    bot::{print_move_stats, print_search_header, print_total_stats},
    othello::position::Position,
};

pub static MIN_ENDGAME_SCORE: isize = -64;
pub static MAX_ENDGAME_SCORE: isize = 64;

pub struct EndgameSearch {
    nodes: u64,
    position: Position,
}

impl EndgameSearch {
    pub fn new(position: &Position) -> Self {
        Self {
            nodes: 0,
            position: *position,
        }
    }

    // TODO #5 bring from Edax, make this private again
    pub fn get_move(&mut self) -> usize {
        let children = self.position.children_with_index();

        let mut best_move = children.first().unwrap().0;
        let mut alpha = MIN_ENDGAME_SCORE;

        let mut total_nodes = 0;
        let mut total_duration = Duration::ZERO;

        print_search_header("SquaredBot", true, self.position.count_empty());
        for (i, (move_, child)) in children.iter().enumerate() {
            let start = Instant::now();
            let score = -self.negamax(child, -MAX_ENDGAME_SCORE, -alpha);
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

    fn negamax(&mut self, position: &Position, mut alpha: isize, beta: isize) -> isize {
        self.nodes += 1;

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
            return -self.negamax(&passed_position, -beta, -alpha);
        }

        for child in &children {
            let score = -self.negamax(child, -beta, -alpha);
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
    use super::*;
    use crate::othello::ffo_problems::parse_ffo_problems;

    #[test]
    fn test_ffo_problems() {
        if std::env::var("RUN_FFO_TESTS").is_err() {
            println!("Skipping FFO tests. Set RUN_FFO_TESTS environment variable to run them.");
            return;
        }

        use rayon::prelude::*;

        let problems: Vec<_> = parse_ffo_problems()
            .into_iter()
            .filter(|p| p.depth <= 14)
            .collect();

        for (problem_id, problem) in problems.iter().enumerate() {
            println!(
                "Testing problem {:2}/{:2}, line {:2}, depth {:2}",
                problem_id + 1,
                problems.len(),
                problem.line_number,
                problem.depth,
            );

            problem
                .solutions
                .iter()
                .par_bridge()
                .for_each(|(&move_, &expected_score)| {
                    let child = problem.position.do_move_cloned(move_);

                    let mut search = EndgameSearch::new(&child);

                    let start = Instant::now();
                    let score = -search.negamax(&child, MIN_ENDGAME_SCORE, MAX_ENDGAME_SCORE);
                    let duration = start.elapsed();

                    print_move_stats(search.nodes, 0, 1, score, MIN_ENDGAME_SCORE, duration);

                    assert_eq!(
                        score,
                        expected_score,
                        "Problem {}, move {} failed: expected {}, got {}",
                        problem_id + 1,
                        move_,
                        expected_score,
                        score
                    );
                });
        }
    }
}
