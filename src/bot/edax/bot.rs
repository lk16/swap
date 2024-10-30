use crate::bot::squared::SquaredBot;
use crate::othello::position::Position;

use crate::bot::Bot;

use super::eval::Eval;
use super::weights::EVAL_WEIGHT;

pub struct EdaxBot;

const SCORE_MIN: i16 = -64;
const SCORE_MAX: i16 = 64;

const MIDGAME_DEPTH: u32 = 8;
const ENDGAME_DEPTH: u32 = 14;

// TODO something is not working, check if weights loaded correctly and add many tests

impl Bot for EdaxBot {
    fn get_move(&self, position: &Position) -> usize {
        let moves = position.get_moves();

        if moves == 0 {
            panic!("No moves available");
        }

        if moves.count_ones() == 1 {
            return moves.trailing_zeros() as usize;
        }

        if position.count_empty() > ENDGAME_DEPTH {
            let mut search = MidgameSearch::new(*position);
            return search.get_move();
        }

        // TODO #5 bring Edax endgame search
        SquaredBot::endgame_get_move(position)
    }
}

struct MidgameSearch {
    position: Position,
    eval: Eval,
    n_empties: u32,
}

impl MidgameSearch {
    // TODO #6 bring better midgame search from Edax

    fn new(position: Position) -> Self {
        Self {
            position,
            eval: Eval::new(&position),
            n_empties: position.count_empty(),
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

    fn get_move(&mut self) -> usize {
        let mut remaining_moves = self.position.get_moves();

        let mut best_move = 99; // Invalid move
        let mut alpha = SCORE_MIN;

        while remaining_moves != 0 {
            let move_ = remaining_moves.trailing_zeros() as usize;

            let flipped = self.do_move(move_);
            let score = -self.negamax(MIDGAME_DEPTH - 1, -SCORE_MAX, -alpha);
            self.undo_move(move_, flipped);

            if score > alpha {
                alpha = score;
                best_move = move_;
            }

            remaining_moves &= remaining_moves - 1;
        }

        best_move
    }

    fn negamax(&mut self, depth: u32, mut alpha: i16, beta: i16) -> i16 {
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
                let score = self.position.final_score() as i16;
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

    fn heuristic(&self) -> i16 {
        let w = &EVAL_WEIGHT[self.eval.player as usize][(60 - self.n_empties) as usize];
        let f = &self.eval.features;

        let mut score = w[f[0] as usize]
            + w[f[1] as usize]
            + w[f[2] as usize]
            + w[f[3] as usize]
            + w[f[4] as usize]
            + w[f[5] as usize]
            + w[f[6] as usize]
            + w[f[7] as usize]
            + w[f[8] as usize]
            + w[f[9] as usize]
            + w[f[10] as usize]
            + w[f[11] as usize]
            + w[f[12] as usize]
            + w[f[13] as usize]
            + w[f[14] as usize]
            + w[f[15] as usize]
            + w[f[16] as usize]
            + w[f[17] as usize]
            + w[f[18] as usize]
            + w[f[19] as usize]
            + w[f[20] as usize]
            + w[f[21] as usize]
            + w[f[22] as usize]
            + w[f[23] as usize]
            + w[f[24] as usize]
            + w[f[25] as usize]
            + w[f[26] as usize]
            + w[f[27] as usize]
            + w[f[28] as usize]
            + w[f[29] as usize]
            + w[f[30] as usize]
            + w[f[31] as usize]
            + w[f[32] as usize]
            + w[f[33] as usize]
            + w[f[34] as usize]
            + w[f[35] as usize]
            + w[f[36] as usize]
            + w[f[37] as usize]
            + w[f[38] as usize]
            + w[f[39] as usize]
            + w[f[40] as usize]
            + w[f[41] as usize]
            + w[f[42] as usize]
            + w[f[43] as usize]
            + w[f[44] as usize]
            + w[f[45] as usize]
            + w[f[46] as usize];

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
