use crate::{
    collections::{forward_pool_list::ForwardPoolList, pool_list::PoolList},
    othello::position::Position,
};

use super::{
    eval::{Eval, EVAL_N_FEATURES},
    r#const::{
        NodeType, GAME_SIZE, NWS_STABILITY_THRESHOLD, PVS_STABILITY_THRESHOLD, SCORE_INF,
        SCORE_MAX, SCORE_MIN,
    },
    r#move::Move,
    search::Bound,
    square::{Square, PRESORTED_X, QUADRANT_ID},
    weights::EVAL_WEIGHT,
};

/// Mutable search state that changes frequently during search
pub struct SearchState {
    /// Search position, changes during search
    pub position: Position,

    /// Number of empty squares in `position`
    pub n_empties: i32,

    /// Empty squares in `position`
    pub empties: PoolList<Square, 64>,

    /// Legal moves in `position`
    pub movelist: ForwardPoolList<Move, 64>,

    /// Quadrant parity of `position`
    pub parity: u32,

    /// Evaluation of `position`
    pub eval: Eval,

    /// Index of the empty square in `empties`
    pub x_to_empties: [usize; 64],

    /// Height of the search tree
    pub height: i32,

    /// Type of the node at `height`
    pub node_type: [NodeType; GAME_SIZE],

    /// Depth of PV extension
    pub depth_pv_extension: i32,

    /// Stability bound
    pub stability_bound: Bound,
}

impl SearchState {
    pub fn new(position: &Position) -> Self {
        let (empties, x_to_empties, parity) = Self::setup(position);

        Self {
            position: *position,
            n_empties: position.count_empty() as i32,
            empties,
            parity,
            eval: Eval::new(position),
            x_to_empties,
            movelist: Self::get_movelist(position),
            height: 0,
            node_type: [NodeType::default(); GAME_SIZE],
            depth_pv_extension: 0,
            stability_bound: Bound::default(),
        }
    }

    /// Like search_setup() in Edax
    fn setup(position: &Position) -> (PoolList<Square, 64>, [usize; 64], u32) {
        let mut empties = PoolList::default();
        let mut x_to_empties = [0; 64];
        let mut parity = 0;

        let e = !(position.player | position.opponent);

        for (i, &x) in PRESORTED_X.iter().enumerate() {
            if e & (1 << x) != 0 {
                empties.push(Square::new(x));
                x_to_empties[x] = i;
            }
        }

        for empty in empties.iter() {
            parity ^= empty.x as u32;
        }

        (empties, x_to_empties, parity)
    }

    /// Like search_get_movelist() in Edax
    pub fn get_movelist(position: &Position) -> ForwardPoolList<Move, 64> {
        let mut movelist = ForwardPoolList::new();

        for x in position.iter_move_indices() {
            let move_ = Move {
                cost: 0,
                flipped: position.get_flipped(x),
                x: x as i32,
                score: 0,
            };

            movelist.push(move_);
        }

        movelist
    }

    /// Like get_pv_extension() in Edax
    pub fn get_pv_extension(&self, depth: i32) -> i32 {
        let n_empties = self.n_empties;

        if depth >= n_empties || depth <= 9 {
            -1
        } else if depth <= 12 {
            10
        } else if depth <= 18 {
            12
        } else if depth <= 24 {
            14
        } else {
            16
        }
    }

    /// Like search_bound() in Edax
    pub fn bound(&self, score: i32) -> i32 {
        score.clamp(self.stability_bound.lower, self.stability_bound.upper)
    }

    /// Like update_pass_midgame() in Edax
    pub fn update_pass_midgame(&mut self) {
        const NEXT_NODE_TYPE: [NodeType; 3] =
            [NodeType::CutNode, NodeType::AllNode, NodeType::CutNode];

        self.position.pass();
        self.eval.pass();
        self.height += 1;
        self.node_type[self.height as usize] =
            NEXT_NODE_TYPE[self.node_type[(self.height - 1) as usize] as usize];
    }

    /// Like restore_pass_midgame() in Edax
    pub fn restore_pass_midgame(&mut self) {
        self.position.pass();
        self.eval.pass();
        self.height -= 1;
    }

    /// Like search_eval_0() in Edax
    pub fn eval_0(&self) -> i32 {
        self.eval.heuristic()
    }

    /// Like search_eval_1() in Edax
    pub fn eval_1(&mut self, alpha: i32, mut beta: i32) -> i32 {
        let weights =
            &EVAL_WEIGHT[(self.eval.player() ^ 1) as usize][(61 - self.n_empties) as usize];
        let mut bestscore;

        let moves = self.position.get_moves();

        if moves == 0 {
            if self.position.opponent_has_moves() {
                self.update_pass_midgame();
                bestscore = -self.eval_1(beta, alpha);
                self.restore_pass_midgame();
            } else {
                // game over
                bestscore = self.solve();
            }
        } else {
            bestscore = -SCORE_INF;
            if beta >= SCORE_MAX {
                beta = SCORE_MAX - 1;
            }
            for empty in self.empties.iter() {
                if moves & empty.b != 0 {
                    let flipped = self.position.get_flipped(empty.x as usize);

                    if flipped == self.position.opponent {
                        return SCORE_MAX;
                    }
                    self.eval.do_move(empty.x as usize, flipped);
                    let f = self.eval.features();

                    let mut score = 0;
                    for i in 0..EVAL_N_FEATURES {
                        score -= weights[f[i] as usize] as i32;
                    }

                    self.eval.undo_move(empty.x as usize, flipped);

                    if score > 0 {
                        score += 64;
                    } else {
                        score -= 64;
                    }
                    score /= 128;

                    if score > bestscore {
                        bestscore = score;
                        if bestscore >= beta {
                            break;
                        }
                    }
                }
            }
            if bestscore <= SCORE_MIN {
                bestscore = SCORE_MIN + 1;
            } else if bestscore >= SCORE_MAX {
                bestscore = SCORE_MAX - 1;
            }
        }

        bestscore
    }

    /// Like search_eval_2() in Edax
    pub fn eval_2(&mut self, mut alpha: i32, beta: i32) -> i32 {
        let moves = self.position.get_moves();

        let mut bestscore;
        if moves == 0 {
            if self.position.opponent_has_moves() {
                self.update_pass_midgame();
                bestscore = -self.eval_2(-beta, -alpha);
                self.restore_pass_midgame();
            } else {
                bestscore = self.solve();
            }
        } else {
            bestscore = -SCORE_INF;

            // Clone empties to avoid problems with borrow checker
            // TODO #15 Further optimization: do not clone empties
            let empties = self.empties.clone();

            for empty in empties.iter() {
                if moves & empty.b != 0 {
                    let move_ = Move::new(&self.position, empty.x);
                    self.update_midgame(&move_);
                    let score = -self.eval_1(-beta, -alpha);
                    self.restore_midgame(&move_);

                    if score > bestscore {
                        bestscore = score;
                        if bestscore >= beta {
                            break;
                        } else if bestscore > alpha {
                            alpha = bestscore;
                        }
                    }
                }
            }
        }

        bestscore
    }

    /// Computes final score knowing the number of empty squares.
    ///
    /// Like search_solve() in Edax
    pub fn solve(&self) -> i32 {
        self.position.final_score_with_empty(self.n_empties)
    }

    /// Like search_update_midgame() in Edax
    pub fn update_midgame(&mut self, move_: &Move) {
        const NEXT_NODE_TYPE: [NodeType; 3] =
            [NodeType::CutNode, NodeType::AllNode, NodeType::CutNode];

        // Update parity by XORing with the quadrant ID of the played move
        self.parity ^= QUADRANT_ID[move_.x as usize];

        // Remove the played square from empties list using x_to_empties mapping
        self.empties.remove(self.x_to_empties[move_.x as usize]);

        // Update position and evaluation
        self.position.do_move(move_.x as usize);
        self.eval.do_move(move_.x as usize, move_.flipped);

        // Update search state
        self.n_empties -= 1;
        self.height += 1;
        self.node_type[self.height as usize] =
            NEXT_NODE_TYPE[self.node_type[(self.height - 1) as usize] as usize];
    }

    /// Like search_restore_midgame() in Edax
    pub fn restore_midgame(&mut self, move_: &Move) {
        // Restore parity by XORing again with the same quadrant ID (XOR is its own inverse)
        self.parity ^= QUADRANT_ID[move_.x as usize];

        // Add back the square to empties list using x_to_empties mapping
        self.empties.restore(self.x_to_empties[move_.x as usize]);

        // Restore position and evaluation
        self.position.undo_move(move_.x as usize, move_.flipped);
        self.eval.undo_move(move_.x as usize, move_.flipped);

        // Restore search state
        self.n_empties += 1;
        self.height -= 1;
    }

    /// Like search_SC_NWS() in Edax
    pub fn stability_cutoff_nws(&self, alpha: i32) -> Option<i32> {
        if alpha >= NWS_STABILITY_THRESHOLD[self.n_empties as usize] {
            let score = SCORE_MAX - 2 * self.position.count_opponent_stable_discs();
            if score <= alpha {
                return Some(score);
            }
        }

        None
    }

    /// Like search_SC_PVS() in Edax
    pub fn stability_cutoff_pvs(&mut self, alpha: i32, beta: &mut i32) -> Option<i32> {
        if *beta >= PVS_STABILITY_THRESHOLD[self.n_empties as usize] {
            let score = SCORE_MAX - 2 * self.position.count_opponent_stable_discs();
            if score <= alpha {
                return Some(score);
            } else if score < *beta {
                *beta = score;
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_state_initialization() {
        // Test new() with a custom position
        let custom_pos = Position::new();
        let state = SearchState::new(&custom_pos);
        verify_invariants(&state);
    }

    fn verify_invariants(state: &SearchState) {
        // Check n_empties matches actual empty squares count
        assert_eq!(state.n_empties, state.position.count_empty() as i32);

        // Check parity calculation
        let mut expected_parity = 0;
        for empty in state.empties.iter() {
            expected_parity ^= empty.x as u32;
        }
        assert_eq!(state.parity, expected_parity);

        // Check empties contains only empty squares
        let empty_squares = !(state.position.player | state.position.opponent);
        for empty in state.empties.iter() {
            assert_ne!(empty.b & empty_squares, 0);
        }

        // Check x_to_empties is correctly set up
        for (i, empty) in state.empties.iter().enumerate() {
            assert_eq!(state.x_to_empties[empty.x as usize], i);
        }
    }
}
