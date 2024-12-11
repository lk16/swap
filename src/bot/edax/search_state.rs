use rand::Rng;

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

/// Mutable search state that changes frequently during search.
///
/// All fields are private to maintain the following invariants (verified in tests):
/// - `n_empties` matches the actual count of empty squares in `position`
/// - `parity` contains quadrant parity of `position`
/// - `empties` contains only squares that are actually empty in `position`
/// - `x_to_empties` correctly maps square indices to their position in `empties`
/// - `eval` correctly represents the evaluation state of `position`
///
/// These invariants are maintained by all public methods and the constructor.
/// See the test module for verification of these invariants.
pub struct SearchState {
    /// Search position, changes during search
    position: Position,

    /// Number of empty squares in `position`
    n_empties: i32,

    /// Empty squares in `position`
    empties: PoolList<Square, 64>,

    /// Legal moves in `position`
    move_list: ForwardPoolList<Move, 64>,

    /// Quadrant parity of `position`
    parity: u32,

    /// Evaluation of `position`
    eval: Eval,

    /// Index of the empty square in `empties`
    x_to_empties: [usize; 64],

    /// Height of the search tree
    height: i32,

    /// Type of the node at `height`
    node_type: [NodeType; GAME_SIZE],

    /// Depth of PV extension
    depth_pv_extension: i32,

    /// Stability bound
    stability_bound: Bound,
}

impl SearchState {
    /// Includes logic of search_setup()
    pub fn new(position: &Position) -> Self {
        let n_empties = position.count_empty() as i32;

        let mut empties = PoolList::default();
        let mut x_to_empties = [0; 64];
        let mut parity = 0;

        let empties_bitset = !(position.player | position.opponent);

        for x in PRESORTED_X {
            if empties_bitset & (1 << x) != 0 {
                x_to_empties[x] = empties.push(Square::new(x));
                parity ^= QUADRANT_ID[x];
            }
        }

        Self {
            position: *position,
            n_empties,
            empties,
            parity,
            eval: Eval::new(position),
            x_to_empties,
            move_list: Self::get_movelist(position),
            height: 0,
            node_type: [NodeType::default(); GAME_SIZE],
            depth_pv_extension: Self::get_pv_extension(n_empties, 0),
            stability_bound: Bound {
                upper: SCORE_MAX - 2 * position.count_opponent_stable_discs(),
                lower: 2 * position.count_player_stable_discs() - SCORE_MAX,
            },
        }
    }

    /// Like search_get_movelist() in Edax
    pub fn get_movelist(position: &Position) -> ForwardPoolList<Move, 64> {
        let moves = position.iter_move_indices();

        // We know that the lower_bound on size_hint() is giving exact size for MoveIndices
        let size = moves.size_hint().0;

        ForwardPoolList::from_iter_with_size(moves.map(|x| Move::new(position, x as i32)), size)
    }

    /// Like get_pv_extension() in Edax
    fn get_pv_extension(n_empties: i32, depth: i32) -> i32 {
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
    pub fn stability_cutoff_pvs(&self, alpha: i32, beta: &mut i32) -> Option<i32> {
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

    pub fn move_list(&self) -> &ForwardPoolList<Move, 64> {
        &self.move_list
    }

    /// Returns a mutable reference to the move list.
    ///
    /// # Contract
    /// The caller MUST ensure that:
    /// - No moves are added to or removed from the list
    /// - Only the scores or ordering of existing moves are modified
    ///
    /// Violating these requirements will break internal invariants and may cause
    /// incorrect behavior, though it won't cause memory safety issues.
    pub fn move_list_mut(&mut self) -> &mut ForwardPoolList<Move, 64> {
        &mut self.move_list
    }

    pub fn position(&self) -> &Position {
        &self.position
    }

    pub fn stability_bound(&self) -> &Bound {
        &self.stability_bound
    }

    pub fn n_empties(&self) -> i32 {
        self.n_empties
    }

    pub fn parity(&self) -> u32 {
        self.parity
    }

    pub fn sort_move_list(&mut self) {
        self.move_list.sort();
    }

    pub fn set_best_move_score(&mut self, score: i32) {
        self.move_list.first_mut().unwrap().score = score;
    }

    pub fn get_best_move(&self) -> &Move {
        self.move_list.first().unwrap()
    }

    pub fn set_pv_extension(&mut self, depth: i32) {
        self.depth_pv_extension = Self::get_pv_extension(self.n_empties, depth);
    }

    pub fn randomize_move_list_score(&mut self) {
        for move_ in self.move_list.iter_mut() {
            move_.score = rand::thread_rng().gen::<i32>() & 0x7fffffff;
        }
    }
}

#[cfg(test)]
mod tests {

    use std::collections::HashSet;

    use crate::bot::edax::eval::tests::test_positions;

    use super::*;

    impl SearchState {
        fn check_invariant_parity(&self) {
            let mut expected_parity = 0;
            for i in 0..64 {
                if (self.position.player | self.position.opponent) & (1 << i) == 0 {
                    expected_parity ^= QUADRANT_ID[i as usize] as u32;
                }
            }
            assert_eq!(self.parity, expected_parity);
        }

        fn check_invariant_n_empties(&self) {
            assert_eq!(self.n_empties, self.position.count_empty() as i32);
        }

        fn check_invariant_empties(&self) {
            let expected_empties = (0..64)
                .filter(|&i| (self.position.player | self.position.opponent) & (1 << i) == 0)
                .collect::<HashSet<_>>();

            let empties = self.empties.iter().map(|s| s.x).collect::<HashSet<_>>();
            assert_eq!(empties, expected_empties);
        }

        fn check_invariant_x_to_empties(&self) {
            let empties = self.empties.iter().map(|s| s.x).collect::<HashSet<_>>();

            for (x, &empty_index) in self.x_to_empties.iter().enumerate() {
                if empties.contains(&(x as i32)) {
                    assert_eq!(self.empties.get(empty_index).x, x as i32);
                }
                // NOTE: if x is not in empties, it should not be used and the value can be anything.
            }
        }

        fn check_invariant_eval(&self) {
            let expected_eval = if self.eval.player() == 0 {
                Eval::new(&self.position)
            } else {
                Eval::new_for_opponent(&self.position)
            };

            assert_eq!(self.eval, expected_eval);
        }

        fn validate(&self, expected_position: &Position) {
            // Print position, since we use randomized positions for state.
            println!("self.position = {:?}", self.position);
            println!("{}", self.position);

            assert_eq!(self.position, *expected_position);

            self.check_invariant_parity();
            self.check_invariant_n_empties();
            self.check_invariant_empties();
            self.check_invariant_x_to_empties();
            self.check_invariant_eval();
        }
    }

    #[test]
    fn test_search_state_invariants_new() {
        // Test new() with initial position
        for position in test_positions() {
            let state = SearchState::new(&position);
            state.validate(&position);
        }
    }

    #[test]
    fn test_search_state_invariants_pass() {
        for mut position in test_positions() {
            let mut state = SearchState::new(&position);
            state.validate(&position);

            state.update_pass_midgame();
            position.pass();
            state.validate(&position);

            state.restore_pass_midgame();
            position.pass();
            state.validate(&position);
        }
    }

    #[test]
    fn test_search_state_invariants_eval_1() {
        for position in test_positions() {
            if position.count_empty() <= 1 {
                // Prevent index out of bounds in Eval.
                continue;
            }

            let mut state = SearchState::new(&position);
            state.validate(&position);

            state.eval_1(3, 6);
            state.validate(&position);
        }
    }

    #[test]
    fn test_search_state_invariants_eval_2() {
        for position in test_positions() {
            if position.count_empty() <= 2 {
                // Prevent index out of bounds in Eval.
                continue;
            }

            let mut state = SearchState::new(&position);
            state.validate(&position);

            state.eval_2(3, 6);
            state.validate(&position);
        }
    }

    #[test]
    fn test_search_state_invariants_move_midgame() {
        for mut position in test_positions() {
            let mut state = SearchState::new(&position);
            state.validate(&position);

            for move_ in SearchState::get_movelist(&position).iter() {
                let index = move_.x as usize;

                state.update_midgame(&move_);
                let flipped = position.do_move(index);
                state.validate(&position);

                state.restore_midgame(&move_);
                position.undo_move(index, flipped);
                state.validate(&position);
            }
        }
    }

    #[test]
    fn test_search_state_invariants_sort_move_list() {
        for position in test_positions() {
            let mut state = SearchState::new(&position);
            state.validate(&position);

            state.sort_move_list();
            state.validate(&position);
        }
    }

    #[test]
    fn test_search_state_invariants_set_best_move_score() {
        for position in test_positions() {
            if !position.has_moves() {
                continue; // Can't set score if there are no moves.
            }

            let mut state = SearchState::new(&position);
            state.validate(&position);

            state.set_best_move_score(10);
            state.validate(&position);
        }
    }

    #[test]
    fn test_search_state_invariants_set_pv_extension() {
        for position in test_positions() {
            let mut state = SearchState::new(&position);
            state.validate(&position);

            state.set_pv_extension(10);
            state.validate(&position);
        }
    }

    #[test]
    fn test_search_state_invariants_randomize_move_list_score() {
        for position in test_positions() {
            let mut state = SearchState::new(&position);
            state.validate(&position);

            state.randomize_move_list_score();
            state.validate(&position);
        }
    }

    #[test]
    fn test_search_state_invariants_move_list_mut_changing_scores() {
        // This test does not PROVE that the invariants are maintained,
        // however this is one of the ways move_list_mut() is intended to be used.

        for position in test_positions() {
            let mut state = SearchState::new(&position);
            state.validate(&position);

            for move_ in state.move_list_mut().iter_mut() {
                move_.score = rand::thread_rng().gen::<i32>() & 0x7fffffff;
            }

            state.validate(&position);
        }
    }

    #[test]
    fn test_search_state_get_movelist() {
        for position in test_positions() {
            let movelist = SearchState::get_movelist(&position);

            let move_indeces = position.iter_move_indices().collect::<HashSet<_>>();
            let movelist_indeces = movelist
                .iter()
                .map(|m| m.x as usize)
                .collect::<HashSet<_>>();

            // Check that the move list contains all and only the legal moves.
            assert_eq!(move_indeces, movelist_indeces);

            for move_ in movelist.iter() {
                assert_eq!(move_.flipped, position.get_flipped(move_.x as usize));
                assert_eq!(move_.x, move_.x as usize as i32);
                assert_eq!(move_.score, 0);
                assert_eq!(move_.cost, 0);
            }
        }
    }

    #[test]
    fn test_search_bound() {
        let mut state = SearchState::new(&Position::new());
        state.stability_bound = Bound { lower: 2, upper: 5 };

        assert_eq!(state.bound(0), 2);
        assert_eq!(state.bound(2), 2);
        assert_eq!(state.bound(3), 3);
        assert_eq!(state.bound(5), 5);
        assert_eq!(state.bound(7), 5);
    }

    #[test]
    fn test_eval_0() {
        for position in test_positions() {
            let state = SearchState::new(&position);
            assert_eq!(state.eval_0(), state.eval.heuristic());
        }
    }

    impl SearchState {
        fn eval_1_naive(&mut self, alpha: i32, mut beta: i32) -> i32 {
            let moves = self.position.get_moves();

            if moves == 0 {
                if self.position.opponent_has_moves() {
                    self.update_pass_midgame();
                    let score = -self.eval_1_naive(beta, alpha);
                    self.restore_pass_midgame();
                    return score;
                } else {
                    return self.solve();
                }
            } else {
                let mut bestscore = -SCORE_INF;
                if beta >= SCORE_MAX {
                    beta = SCORE_MAX - 1;
                }

                for empty in self.empties.clone().iter() {
                    if moves & empty.b != 0 {
                        let flipped = self.position.get_flipped(empty.x as usize);

                        if flipped == self.position.opponent {
                            return SCORE_MAX;
                        }

                        let move_ = Move::new(&self.position, empty.x);
                        self.update_midgame(&move_);
                        let score = -self.eval_0();
                        self.restore_midgame(&move_);

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

                return bestscore;
            }
        }

        fn eval_2_naive(&mut self, mut alpha: i32, beta: i32) -> i32 {
            let moves = self.position.get_moves();

            if moves == 0 {
                if self.position.opponent_has_moves() {
                    self.update_pass_midgame();
                    let score = -self.eval_2_naive(-beta, -alpha);
                    self.restore_pass_midgame();
                    return score;
                } else {
                    return self.solve();
                }
            } else {
                let mut bestscore = -SCORE_INF;

                for empty in self.empties.clone().iter() {
                    if moves & empty.b != 0 {
                        let flipped = self.position.get_flipped(empty.x as usize);

                        if flipped == self.position.opponent {
                            return SCORE_MAX;
                        }

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

                if bestscore <= SCORE_MIN {
                    bestscore = SCORE_MIN + 1;
                } else if bestscore >= SCORE_MAX {
                    bestscore = SCORE_MAX - 1;
                }

                return bestscore;
            }
        }
    }

    #[test]
    fn test_eval_1_naive() {
        let bounds = [
            (SCORE_MIN, SCORE_MAX),
            (SCORE_MIN, 0),
            (0, SCORE_MAX),
            (-10, 10),
        ];

        for position in test_positions() {
            if position.count_empty() <= 1 {
                continue; // Prevent index out of bounds in Eval.
            }

            for (alpha, beta) in bounds {
                let mut state = SearchState::new(&position);

                assert_eq!(state.eval_1(alpha, beta), state.eval_1_naive(alpha, beta));
            }
        }
    }

    #[test]
    fn test_eval_2_naive() {
        let bounds = [
            (SCORE_MIN, SCORE_MAX),
            (SCORE_MIN, 0),
            (0, SCORE_MAX),
            (-10, 10),
        ];

        for position in test_positions() {
            if position.count_empty() <= 2 {
                continue; // Prevent index out of bounds in Eval.
            }

            for (alpha, beta) in bounds {
                let mut state = SearchState::new(&position);
                assert_eq!(state.eval_2(alpha, beta), state.eval_2_naive(alpha, beta));
            }
        }
    }

    #[test]
    fn test_solve() {
        for position in test_positions() {
            let state = SearchState::new(&position);
            assert_eq!(
                state.solve(),
                position.final_score_with_empty(state.n_empties)
            );
            assert_eq!(state.solve(), position.final_score() as i32);
        }
    }

    #[test]
    fn test_set_best_move_score() {
        for position in test_positions() {
            if !position.has_moves() {
                continue; // Can't set score if there are no moves.
            }

            let mut state = SearchState::new(&position);
            state.set_best_move_score(10);
            assert_eq!(state.get_best_move().score, 10);
        }
    }

    #[test]
    fn test_get_best_move() {
        for position in test_positions() {
            if !position.has_moves() {
                continue; // Can't get best move if there are no moves.
            }

            let mut state = SearchState::new(&position);

            // This proves it works because the score is 0 for all moves initially.
            state.set_best_move_score(10);
            assert_eq!(state.get_best_move().score, 10);
        }
    }

    #[test]
    fn test_randomize_move_list_score() {
        for position in test_positions() {
            let mut state = SearchState::new(&position);

            // Test that no moves are added or removed.
            let before = state.move_list.iter().map(|m| m.x).collect::<Vec<_>>();
            state.randomize_move_list_score();
            let after = state.move_list.iter().map(|m| m.x).collect::<Vec<_>>();

            assert_eq!(before, after);
        }
    }
}
