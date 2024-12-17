use rand::Rng;

use crate::{
    collections::{
        empties_list::{EmptiesList, Square},
        hashtable::HashData,
        move_list::{Move, MoveList},
    },
    othello::position::Position,
};

use super::{
    eval::{Eval, EVAL_N_FEATURES},
    r#const::{
        NodeType, GAME_SIZE, NWS_STABILITY_THRESHOLD, PRESORTED_X, PVS_STABILITY_THRESHOLD,
        QUADRANT_ID, SCORE_INF, SCORE_MAX, SCORE_MIN,
    },
    search::Bound,
    weights::EVAL_WEIGHT,
};

/// Mutable search state that changes frequently during search.
///
/// All fields are private to maintain the following invariants (verified in tests):
/// - `n_empties` matches the actual count of empty squares in `position`
/// - `parity` contains quadrant parity of `position`
/// - `empties` contains only squares that are actually empty in `position`
/// - `x_to_empties` correctly maps square indices to their position in `empties`
///
/// Additionally, for midgame search we maintain this invariant:
/// - `eval` correctly represents the evaluation state of `position`
///
/// Invariants are maintained by all public methods and the constructor.
/// See the test module for details and verification.
pub struct SearchState {
    /// Search position, changes during search
    position: Position,

    /// Number of empty squares in `position`
    n_empties: i32,

    /// Empty squares in `position`
    empties: EmptiesList,

    /// Legal moves in `position`
    move_list: MoveList,

    /// Quadrant parity of `position`
    parity: u32,

    /// Evaluation of `position`
    eval: Eval,

    /// Height of the search tree
    height: i32,

    /// Type of the node at `height`
    node_type: [NodeType; GAME_SIZE],

    /// Depth of PV extension
    depth_pv_extension: i32,

    /// Stability bound
    stability_bound: Bound,

    /// Probcut recursion level
    probcut_level: i32,
}

impl SearchState {
    /// Create a new search state for a position.
    ///
    /// Includes logic of Edax's search_setup()
    pub fn new(position: &Position) -> Self {
        let n_empties = position.count_empty() as i32;

        // TODO create helper function for this
        let empties_bitset = !(position.player() | position.opponent());

        let mut parity = 0;
        for x in PRESORTED_X {
            if empties_bitset & (1 << x) != 0 {
                parity ^= QUADRANT_ID[x];
            }
        }

        let empties = EmptiesList::from_iter_with_size(
            PRESORTED_X
                .iter()
                .filter(|&x| empties_bitset & (1 << x) != 0)
                .map(|x| Square::new(*x)),
            n_empties as usize,
        );

        Self {
            position: *position,
            n_empties,
            empties,
            parity,
            eval: Eval::new(position),
            move_list: MoveList::new(position),
            height: 0,
            node_type: [NodeType::default(); GAME_SIZE],
            depth_pv_extension: Self::get_pv_extension(n_empties, 0),
            stability_bound: Bound {
                upper: SCORE_MAX - 2 * position.count_opponent_stable_discs(),
                lower: 2 * position.count_player_stable_discs() - SCORE_MAX,
            },
            probcut_level: 0,
        }
    }

    /// Compute depth of PV extension.
    ///
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

    /// Clamp a score to the stability bound.
    ///
    /// Like search_bound() in Edax
    pub fn bound(&self, score: i32) -> i32 {
        score.clamp(self.stability_bound.lower, self.stability_bound.upper)
    }

    /// Pass move for midgame search.
    ///
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

    /// Restore passing a move for midgame search.
    ///
    /// Like restore_pass_midgame() in Edax
    pub fn restore_pass_midgame(&mut self) {
        self.position.pass();
        self.eval.pass();
        self.height -= 1;
    }

    /// Evaluate the position using heuristic.
    ///
    /// Like search_eval_0() in Edax
    pub fn eval_0(&self) -> i32 {
        self.eval.heuristic()
    }

    /// Evaluate the position using heuristic at depth 1.
    ///
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

                    if flipped == self.position.opponent() {
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

    /// Evaluate the position using heuristic at depth 2.
    ///
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

    /// Update the search by playing a move in midgame search.
    ///
    /// Like search_update_midgame() in Edax
    pub fn update_midgame(&mut self, move_: &Move) {
        const NEXT_NODE_TYPE: [NodeType; 3] =
            [NodeType::CutNode, NodeType::AllNode, NodeType::CutNode];

        // Update parity by XORing with the quadrant ID of the played move
        self.parity ^= QUADRANT_ID[move_.x as usize];

        // Remove the played square from empties list using x_to_empties mapping
        self.empties.remove_by_x(move_.x);

        // Update position and evaluation
        self.position.do_move(move_.x as usize);
        self.eval.do_move(move_.x as usize, move_.flipped);

        // Update search state
        self.n_empties -= 1;
        self.height += 1;
        self.node_type[self.height as usize] =
            NEXT_NODE_TYPE[self.node_type[(self.height - 1) as usize] as usize];
    }

    /// Restore a move in midgame search.
    ///
    /// Like search_restore_midgame() in Edax
    pub fn restore_midgame(&mut self, move_: &Move) {
        // Restore parity by XORing again with the same quadrant ID (XOR is its own inverse)
        self.parity ^= QUADRANT_ID[move_.x as usize];

        // Add back the square to empties list using x_to_empties mapping
        self.empties.restore_by_x(move_.x);

        // Restore position and evaluation
        self.position.undo_move(move_.x as usize, move_.flipped);
        self.eval.undo_move(move_.x as usize, move_.flipped);

        // Restore search state
        self.n_empties += 1;
        self.height -= 1;
    }

    /// Stability cutoff for Null Window Search.
    ///
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

    /// Stability cutoff for Principal Variation Search.
    ///
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

    /// Get move_list.
    pub fn move_list(&self) -> &MoveList {
        &self.move_list
    }

    /// Get position.
    pub fn position(&self) -> &Position {
        &self.position
    }

    /// Get stability bound.
    pub fn stability_bound(&self) -> &Bound {
        &self.stability_bound
    }

    /// Get number of empty squares.
    pub fn n_empties(&self) -> i32 {
        self.n_empties
    }

    /// Get parity.
    pub fn parity(&self) -> u32 {
        self.parity
    }

    /// Sort move list by score.
    pub fn sort_move_list_by_score(&mut self) {
        self.move_list.sort_by_score();
    }

    /// Sort move list by cost using hash data.
    pub fn sort_move_list_by_cost(&mut self, hash_data: &HashData) {
        self.move_list.sort_by_cost(hash_data);
    }

    /// Set first move.
    pub fn set_first_move(&mut self, move_: i32) {
        self.move_list.set_first_move(move_);
    }

    /// Set best move score.
    pub fn set_best_move_score(&self, score: i32) {
        self.move_list.first().unwrap().score.set(score);
    }

    /// Get best move.
    pub fn get_best_move(&self) -> &Move {
        self.move_list.first().unwrap()
    }

    /// Set depth of PV extension.
    pub fn set_pv_extension(&mut self, depth: i32) {
        self.depth_pv_extension = Self::get_pv_extension(self.n_empties, depth);
    }

    /// Randomize score for all moves in move list.
    pub fn randomize_move_list_score(&self) {
        for move_ in self.move_list.iter() {
            let score = rand::thread_rng().gen::<i32>() & 0x7fffffff;
            move_.score.set(score);
        }
    }

    /// Set move list.
    pub fn set_move_list(&mut self, move_list: MoveList) {
        self.move_list = move_list;
    }

    /// Get height.
    pub fn height(&self) -> i32 {
        self.height
    }

    /// Get depth of PV extension.
    pub fn depth_pv_extension(&self) -> i32 {
        self.depth_pv_extension
    }

    /// Update the search by playing a move in endgame search.
    ///
    /// Like search_update_endgame() in Edax
    pub fn update_endgame(&mut self, move_: &Move) {
        self.parity ^= QUADRANT_ID[move_.x as usize];
        self.empties.remove_by_x(move_.x);
        move_.update(&mut self.position);
        self.n_empties -= 1;
    }

    /// Restore a move in endgame search.
    ///
    /// Like search_restore_endgame() in Edax
    pub fn restore_endgame(&mut self, move_: &Move) {
        self.parity ^= QUADRANT_ID[move_.x as usize];
        self.empties.restore_by_x(move_.x);
        move_.restore(&mut self.position);
        self.n_empties += 1;
    }

    /// Get empties list.
    pub fn empties(&self) -> &EmptiesList {
        &self.empties
    }

    /// Pass move in endgame search.
    /// Also functions as restore for passing a move in endgame search.
    ///
    /// Like search_pass_endgame() in Edax
    pub fn pass_endgame(&mut self) {
        self.position.pass();
    }

    /// Get probcut level.
    pub fn probcut_level(&self) -> i32 {
        self.probcut_level
    }

    /// Set probcut level.
    pub fn set_probcut_level(&mut self, level: i32) {
        self.probcut_level = level;
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

            #[allow(clippy::needless_range_loop)]
            for i in 0..64 {
                if self.position.is_empty(i) {
                    expected_parity ^= QUADRANT_ID[i];
                }
            }
            assert_eq!(self.parity, expected_parity);
        }

        fn check_invariant_n_empties(&self) {
            assert_eq!(self.n_empties, self.position.count_empty() as i32);
        }

        fn check_invariant_empties(&self) {
            let expected_empties = (0..64)
                .filter(|&i| self.position.is_empty(i))
                .collect::<HashSet<_>>();

            let empties = self
                .empties
                .iter()
                .map(|s| s.x as usize)
                .collect::<HashSet<_>>();
            assert_eq!(empties, expected_empties);
        }

        fn check_invariant_eval(&self) {
            let expected_eval = if self.eval.player() == 0 {
                Eval::new(&self.position)
            } else {
                Eval::new_for_opponent(&self.position)
            };

            assert_eq!(self.eval, expected_eval);
        }

        // Not all fields are updated when using update_endgame() and restore_endgame(),
        // so we have two validate functions.

        fn validate_midgame(&self, expected_position: &Position) {
            self.validate_endgame(expected_position);
            self.check_invariant_eval();
        }

        fn validate_endgame(&self, expected_position: &Position) {
            // Print position, since we use randomized positions for state.
            println!("self.position = {:?}", self.position);

            assert_eq!(self.position, *expected_position);

            self.check_invariant_parity();
            self.check_invariant_n_empties();
            self.check_invariant_empties();
        }
    }

    #[test]
    fn test_search_state_invariants_new() {
        // Test new() with initial position
        for position in test_positions() {
            let state = SearchState::new(&position);
            state.validate_midgame(&position);
        }
    }

    #[test]
    fn test_search_state_invariants_pass() {
        for mut position in test_positions() {
            let mut state = SearchState::new(&position);
            state.validate_midgame(&position);

            state.update_pass_midgame();
            position.pass();
            state.validate_midgame(&position);

            state.restore_pass_midgame();
            position.pass();
            state.validate_midgame(&position);
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
            state.validate_midgame(&position);

            state.eval_1(3, 6);
            state.validate_midgame(&position);
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
            state.validate_midgame(&position);

            state.eval_2(3, 6);
            state.validate_midgame(&position);
        }
    }

    #[test]
    fn test_search_state_invariants_move_midgame() {
        for mut position in test_positions() {
            let mut state = SearchState::new(&position);
            state.validate_midgame(&position);

            for move_ in MoveList::new(&position).iter() {
                let index = move_.x as usize;

                state.update_midgame(move_);
                let flipped = position.do_move(index);
                state.validate_midgame(&position);

                state.restore_midgame(move_);
                position.undo_move(index, flipped);
                state.validate_midgame(&position);
            }
        }
    }

    #[test]
    fn test_search_state_invariants_sort_move_list() {
        for position in test_positions() {
            let mut state = SearchState::new(&position);
            state.validate_midgame(&position);

            state.sort_move_list_by_score();
            state.validate_midgame(&position);
        }
    }

    #[test]
    fn test_search_state_invariants_set_best_move_score() {
        for position in test_positions() {
            if !position.has_moves() {
                continue; // Can't set score if there are no moves.
            }

            let state = SearchState::new(&position);
            state.validate_midgame(&position);

            state.set_best_move_score(10);
            state.validate_midgame(&position);
        }
    }

    #[test]
    fn test_search_state_invariants_set_pv_extension() {
        for position in test_positions() {
            let mut state = SearchState::new(&position);
            state.validate_midgame(&position);

            state.set_pv_extension(10);
            state.validate_midgame(&position);
        }
    }

    #[test]
    fn test_search_state_invariants_randomize_move_list_score() {
        for position in test_positions() {
            let state = SearchState::new(&position);
            state.validate_midgame(&position);

            state.randomize_move_list_score();
            state.validate_midgame(&position);
        }
    }

    #[test]
    fn test_search_state_get_movelist() {
        for position in test_positions() {
            let movelist = MoveList::new(&position);

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
                assert_eq!(move_.score.get(), 0);
                assert_eq!(move_.cost.get(), 0);
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
                    score
                } else {
                    self.solve()
                }
            } else {
                let mut bestscore = -SCORE_INF;
                if beta >= SCORE_MAX {
                    beta = SCORE_MAX - 1;
                }

                for empty in self.empties.clone().iter() {
                    if moves & empty.b != 0 {
                        let flipped = self.position.get_flipped(empty.x as usize);

                        if flipped == self.position.opponent() {
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

                bestscore
            }
        }

        fn eval_2_naive(&mut self, mut alpha: i32, beta: i32) -> i32 {
            let moves = self.position.get_moves();

            if moves == 0 {
                if self.position.opponent_has_moves() {
                    self.update_pass_midgame();
                    let score = -self.eval_2_naive(-beta, -alpha);
                    self.restore_pass_midgame();
                    score
                } else {
                    self.solve()
                }
            } else {
                let mut bestscore = -SCORE_INF;

                for empty in self.empties.clone().iter() {
                    if moves & empty.b != 0 {
                        let flipped = self.position.get_flipped(empty.x as usize);

                        if flipped == self.position.opponent() {
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

                bestscore
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

            let state = SearchState::new(&position);
            state.set_best_move_score(10);
            assert_eq!(state.get_best_move().score.get(), 10);
        }
    }

    #[test]
    fn test_get_best_move() {
        for position in test_positions() {
            if !position.has_moves() {
                continue; // Can't get best move if there are no moves.
            }

            let state = SearchState::new(&position);

            // This proves it works because the score is 0 for all moves initially.
            state.set_best_move_score(10);
            assert_eq!(state.get_best_move().score.get(), 10);
        }
    }

    #[test]
    fn test_update_restore_endgame() {
        for position in test_positions() {
            let mut state = SearchState::new(&position);

            let move_list = MoveList::new(&position);

            for move_ in move_list.iter() {
                state.update_endgame(move_);
                let mut child = position;
                move_.update(&mut child);
                state.validate_endgame(&child);

                state.restore_endgame(move_);
                state.validate_endgame(&position);
            }
        }
    }

    #[test]
    fn test_pass_endgame() {
        for position in test_positions() {
            let mut state = SearchState::new(&position);
            state.pass_endgame();

            let mut child = position;
            child.pass();
            state.validate_endgame(&child);

            state.pass_endgame();
            state.validate_endgame(&position);
        }
    }
}
