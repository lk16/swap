// This is adapted from https://github.com/abulmo/edax-reversi/blob/master/src/eval.c

use crate::othello::{position::Position, squares::*};
use lazy_static::lazy_static;

use super::{
    r#const::{SCORE_MAX, SCORE_MIN},
    weights::EVAL_WEIGHT,
};

/// The number of features in the evaluation
pub const EVAL_N_FEATURES: usize = 47;

lazy_static! {
    /// Table that maps features to its squares.
    pub static ref EVAL_F2X: [Vec<usize>; EVAL_N_FEATURES] = [
        /* 0 */ vec![A1, B1, A2, B2, C1, A3, C2, B3, C3],
        /* 1 */ vec![H1, G1, H2, G2, F1, H3, F2, G3, F3],
        /* 2 */ vec![A8, A7, B8, B7, A6, C8, B6, C7, C6],
        /* 3 */ vec![H8, H7, G8, G7, H6, F8, G6, F7, F6],
        /* 4 */ vec![A5, A4, A3, A2, A1, B2, B1, C1, D1, E1],
        /* 5 */ vec![H5, H4, H3, H2, H1, G2, G1, F1, E1, D1],
        /* 6 */ vec![A4, A5, A6, A7, A8, B7, B8, C8, D8, E8],
        /* 7 */ vec![H4, H5, H6, H7, H8, G7, G8, F8, E8, D8],
        /* 8 */ vec![B2, A1, B1, C1, D1, E1, F1, G1, H1, G2],
        /* 9 */ vec![B7, A8, B8, C8, D8, E8, F8, G8, H8, G7],
        /* 10 */ vec![B2, A1, A2, A3, A4, A5, A6, A7, A8, B7],
        /* 11 */ vec![G2, H1, H2, H3, H4, H5, H6, H7, H8, G7],
        /* 12 */ vec![A1, C1, D1, C2, D2, E2, F2, E1, F1, H1],
        /* 13 */ vec![A8, C8, D8, C7, D7, E7, F7, E8, F8, H8],
        /* 14 */ vec![A1, A3, A4, B3, B4, B5, B6, A5, A6, A8],
        /* 15 */ vec![H1, H3, H4, G3, G4, G5, G6, H5, H6, H8],
        /* 16 */ vec![A2, B2, C2, D2, E2, F2, G2, H2],
        /* 17 */ vec![A7, B7, C7, D7, E7, F7, G7, H7],
        /* 18 */ vec![B1, B2, B3, B4, B5, B6, B7, B8],
        /* 19 */ vec![G1, G2, G3, G4, G5, G6, G7, G8],
        /* 20 */ vec![A3, B3, C3, D3, E3, F3, G3, H3],
        /* 21 */ vec![A6, B6, C6, D6, E6, F6, G6, H6],
        /* 22 */ vec![C1, C2, C3, C4, C5, C6, C7, C8],
        /* 23 */ vec![F1, F2, F3, F4, F5, F6, F7, F8],
        /* 24 */ vec![A4, B4, C4, D4, E4, F4, G4, H4],
        /* 25 */ vec![A5, B5, C5, D5, E5, F5, G5, H5],
        /* 26 */ vec![D1, D2, D3, D4, D5, D6, D7, D8],
        /* 27 */ vec![E1, E2, E3, E4, E5, E6, E7, E8],
        /* 28 */ vec![A1, B2, C3, D4, E5, F6, G7, H8],
        /* 29 */ vec![A8, B7, C6, D5, E4, F3, G2, H1],
        /* 30 */ vec![B1, C2, D3, E4, F5, G6, H7],
        /* 31 */ vec![H2, G3, F4, E5, D6, C7, B8],
        /* 32 */ vec![A2, B3, C4, D5, E6, F7, G8],
        /* 33 */ vec![G1, F2, E3, D4, C5, B6, A7],
        /* 34 */ vec![C1, D2, E3, F4, G5, H6],
        /* 35 */ vec![A3, B4, C5, D6, E7, F8],
        /* 36 */ vec![F1, E2, D3, C4, B5, A6],
        /* 37 */ vec![H3, G4, F5, E6, D7, C8],
        /* 38 */ vec![D1, E2, F3, G4, H5],
        /* 39 */ vec![A4, B5, C6, D7, E8],
        /* 40 */ vec![E1, D2, C3, B4, A5],
        /* 41 */ vec![H4, G5, F6, E7, D8],
        /* 42 */ vec![D1, C2, B3, A4],
        /* 43 */ vec![A5, B6, C7, D8],
        /* 44 */ vec![E1, F2, G3, H4],
        /* 45 */ vec![H5, G6, F7, E8],
        /* 46 */ vec![],
    ];
}

lazy_static! {
    /// Table that maps coordinates to features.
    /// The values in the tuples are (feature index, feature value).
    pub static ref EVAL_X2F: [Vec<(usize, i32)>; 65] = [
        /* a1 */ vec![(0, 6561), (4, 243), (8, 6561), (10, 6561), (12, 19683), (14, 19683), (28, 2187)],
        /* b1 */ vec![(0, 2187), (4, 27), (8, 2187), (18, 2187), (30, 729)],
        /* c1 */ vec![(0, 81), (4, 9), (8, 729), (12, 6561), (22, 2187), (34, 243)],
        /* d1 */ vec![(4, 3), (5, 1), (8, 243), (12, 2187), (26, 2187), (38, 81), (42, 27)],
        /* e1 */ vec![(4, 1), (5, 3), (8, 81), (12, 9), (27, 2187), (40, 81), (44, 27)],
        /* f1 */ vec![(1, 81), (5, 9), (8, 27), (12, 3), (23, 2187), (36, 243)],
        /* g1 */ vec![(1, 2187), (5, 27), (8, 9), (19, 2187), (33, 729)],
        /* h1 */ vec![(1, 6561), (5, 243), (8, 3), (11, 6561), (12, 1), (15, 19683), (29, 1)],
        /* a2 */ vec![(0, 729), (4, 729), (10, 2187), (16, 2187), (32, 729)],
        /* b2 */ vec![(0, 243), (4, 81), (8, 19683), (10, 19683), (16, 729), (18, 729), (28, 729)],
        /* c2 */ vec![(0, 9), (12, 729), (16, 243), (22, 729), (30, 243), (42, 9)],
        /* d2 */ vec![(12, 243), (16, 81), (26, 729), (34, 81), (40, 27)],
        /* e2 */ vec![(12, 81), (16, 27), (27, 729), (36, 81), (38, 27)],
        /* f2 */ vec![(1, 9), (12, 27), (16, 9), (23, 729), (33, 243), (44, 9)],
        /* g2 */ vec![(1, 243), (5, 81), (8, 1), (11, 19683), (16, 3), (19, 729), (29, 3)],
        /* h2 */ vec![(1, 729), (5, 729), (11, 2187), (16, 1), (31, 729)],
        /* a3 */ vec![(0, 27), (4, 2187), (10, 729), (14, 6561), (20, 2187), (35, 243)],
        /* b3 */ vec![(0, 3), (14, 729), (18, 243), (20, 729), (32, 243), (42, 3)],
        /* c3 */ vec![(0, 1), (20, 243), (22, 243), (28, 243), (40, 9)],
        /* d3 */ vec![(20, 81), (26, 243), (30, 81), (36, 27)],
        /* e3 */ vec![(20, 27), (27, 243), (33, 81), (34, 27)],
        /* f3 */ vec![(1, 1), (20, 9), (23, 243), (29, 9), (38, 9)],
        /* g3 */ vec![(1, 3), (15, 729), (19, 243), (20, 3), (31, 243), (44, 3)],
        /* h3 */ vec![(1, 27), (5, 2187), (11, 729), (15, 6561), (20, 1), (37, 243)],
        /* a4 */ vec![(4, 6561), (6, 19683), (10, 243), (14, 2187), (24, 2187), (39, 81), (42, 1)],
        /* b4 */ vec![(14, 243), (18, 81), (24, 729), (35, 81), (40, 3)],
        /* c4 */ vec![(22, 81), (24, 243), (32, 81), (36, 9)],
        /* d4 */ vec![(24, 81), (26, 81), (28, 81), (33, 27)],
        /* e4 */ vec![(24, 27), (27, 81), (29, 27), (30, 27)],
        /* f4 */ vec![(23, 81), (24, 9), (31, 81), (34, 9)],
        /* g4 */ vec![(15, 243), (19, 81), (24, 3), (37, 81), (38, 3)],
        /* h4 */ vec![(5, 6561), (7, 19683), (11, 243), (15, 2187), (24, 1), (41, 81), (44, 1)],
        /* a5 */ vec![(4, 19683), (6, 6561), (10, 81), (14, 9), (25, 2187), (40, 1), (43, 27)],
        /* b5 */ vec![(14, 81), (18, 27), (25, 729), (36, 3), (39, 27)],
        /* c5 */ vec![(22, 27), (25, 243), (33, 9), (35, 27)],
        /* d5 */ vec![(25, 81), (26, 27), (29, 81), (32, 27)],
        /* e5 */ vec![(25, 27), (27, 27), (28, 27), (31, 27)],
        /* f5 */ vec![(23, 27), (25, 9), (30, 9), (37, 27)],
        /* g5 */ vec![(15, 81), (19, 27), (25, 3), (34, 3), (41, 27)],
        /* h5 */ vec![(5, 19683), (7, 6561), (11, 81), (15, 9), (25, 1), (38, 1), (45, 27)],
        /* a6 */ vec![(2, 81), (6, 2187), (10, 27), (14, 3), (21, 2187), (36, 1)],
        /* b6 */ vec![(2, 9), (14, 27), (18, 9), (21, 729), (33, 3), (43, 9)],
        /* c6 */ vec![(2, 1), (21, 243), (22, 9), (29, 243), (39, 9)],
        /* d6 */ vec![(21, 81), (26, 9), (31, 9), (35, 9)],
        /* e6 */ vec![(21, 27), (27, 9), (32, 9), (37, 9)],
        /* f6 */ vec![(3, 1), (21, 9), (23, 9), (28, 9), (41, 9)],
        /* g6 */ vec![(3, 9), (15, 27), (19, 9), (21, 3), (30, 3), (45, 9)],
        /* h6 */ vec![(3, 81), (7, 2187), (11, 27), (15, 3), (21, 1), (34, 1)],
        /* a7 */ vec![(2, 2187), (6, 729), (10, 9), (17, 2187), (33, 1)],
        /* b7 */ vec![(2, 243), (6, 81), (9, 19683), (10, 1), (17, 729), (18, 3), (29, 729)],
        /* c7 */ vec![(2, 3), (13, 729), (17, 243), (22, 3), (31, 3), (43, 3)],
        /* d7 */ vec![(13, 243), (17, 81), (26, 3), (37, 3), (39, 3)],
        /* e7 */ vec![(13, 81), (17, 27), (27, 3), (35, 3), (41, 3)],
        /* f7 */ vec![(3, 3), (13, 27), (17, 9), (23, 3), (32, 3), (45, 3)],
        /* g7 */ vec![(3, 243), (7, 81), (9, 1), (11, 1), (17, 3), (19, 3), (28, 3)],
        /* h7 */ vec![(3, 2187), (7, 729), (11, 9), (17, 1), (30, 1)],
        /* a8 */ vec![(2, 6561), (6, 243), (9, 6561), (10, 3), (13, 19683), (14, 1), (29, 2187)],
        /* b8 */ vec![(2, 729), (6, 27), (9, 2187), (18, 1), (31, 1)],
        /* c8 */ vec![(2, 27), (6, 9), (9, 729), (13, 6561), (22, 1), (37, 1)],
        /* d8 */ vec![(6, 3), (7, 1), (9, 243), (13, 2187), (26, 1), (41, 1), (43, 1)],
        /* e8 */ vec![(6, 1), (7, 3), (9, 81), (13, 9), (27, 1), (39, 1), (45, 1)],
        /* f8 */ vec![(3, 27), (7, 9), (9, 27), (13, 3), (23, 1), (35, 1)],
        /* g8 */ vec![(3, 729), (7, 27), (9, 9), (19, 1), (32, 1)],
        /* h8 */ vec![(3, 6561), (7, 243), (9, 3), (11, 3), (13, 1), (15, 1), (28, 1)],
        /* pass */ vec![(0, 0)]
    ];
}

/// Offset per feature.
pub const EVAL_OFFSET: [i32; EVAL_N_FEATURES] = [
    0, 0, 0, 0, 19683, 19683, 19683, 19683, 78732, 78732, 78732, 78732, 137781, 137781, 137781,
    137781, 196830, 196830, 196830, 196830, 203391, 203391, 203391, 203391, 209952, 209952, 209952,
    209952, 216513, 216513, 223074, 223074, 223074, 223074, 225261, 225261, 225261, 225261, 225990,
    225990, 225990, 225990, 226233, 226233, 226233, 226233, 226314,
];

pub const EVAL_MAX_VALUE: [i32; EVAL_N_FEATURES] = [
    19682, 19682, 19682, 19682, 78731, 78731, 78731, 78731, 137780, 137780, 137780, 137780, 196829,
    196829, 196829, 196829, 203390, 203390, 203390, 203390, 209951, 209951, 209951, 209951, 216512,
    216512, 223073, 223073, 223073, 223073, 225260, 225260, 225260, 225260, 225989, 225989, 225989,
    225989, 226232, 226232, 226232, 226232, 226313, 226313, 226313, 226313, 226314,
];

const DO_MOVE_FUNCTIONS: [fn(&mut Eval, usize, u64); 2] =
    [Eval::do_move_player, Eval::do_move_opponent];

const UNDO_MOVE_FUNCTIONS: [fn(&mut Eval, usize, u64); 2] =
    [Eval::undo_move_player, Eval::undo_move_opponent];

/// Represents the evaluation of a position using pattern-based features and pre-computed weights.
/// This evaluation is used only for midgame search.
///
/// This struct maintains the state needed to evaluate an Othello position but does not store the
/// Position itself. Instead, Position and Eval are stored together in GameState and
/// kept in sync during midgame search.
///
/// The evaluation uses a set of pattern-based features (stored in `features`) combined with
/// pre-computed weights to create a strong heuristic evaluation. When moves are made or unmade,
/// the features are incrementally updated rather than recomputed from scratch for efficiency.
///
/// # Performance
/// Incrementally updating features when doing/undoing moves is much more efficient than
/// recomputing the entire evaluation from the Position. This is critical for fast search
/// performance since evaluation happens frequently during game tree traversal.
///
/// All fields are private to prevent breaking invariants.
#[derive(Clone, PartialEq, Debug)]
pub struct Eval {
    /// The features of the position
    features: [i32; EVAL_N_FEATURES],

    /// The player to evaluate from, 0 is player, 1 is opponent
    player: i32,

    /// Contains (60 - number of empty squares), used as index for EVAL_WEIGHT
    empty_index: usize,
}

impl Default for Eval {
    fn default() -> Self {
        Self::new(&Position::new())
    }
}

impl Eval {
    /// Create a new `Eval` struct for a `Position`.
    pub fn new(position: &Position) -> Self {
        let mut eval = Self {
            features: [0; EVAL_N_FEATURES],
            player: 0,
            empty_index: (60 - position.count_empty()) as usize,
        };

        for i in 0..EVAL_N_FEATURES {
            eval.features[i] = 0;

            // construct base-3 value for feature
            for j in 0..EVAL_F2X[i].len() {
                let c = position.get_square_color(EVAL_F2X[i][j]) as i32;
                eval.features[i] = eval.features[i] * 3 + c;
            }

            // add offset for feature, as a performance optimization
            eval.features[i] += EVAL_OFFSET[i];
        }

        eval
    }

    /// Swap the player to evaluate from.
    fn swap(&mut self) {
        self.player = 1 - self.player;
    }

    /// Adjusts feature values when placing/removing or flipping discs
    ///
    /// # Arguments
    /// * `move_pos` - The position where a disc is being placed or removed
    /// * `flipped` - Bitboard of discs that are being flipped
    /// * `M` - Multiplication factor for the disc being placed/removed:
    ///   * When placing: -2 for player's disc (2→0), -1 for opponent's disc (2→1)
    ///   * When removing: +2 for player's disc (0→2), +1 for opponent's disc (1→2)
    /// * `F` - Multiplication factor for flipped discs:
    ///   * When flipping to player: -1 (opponent 1→0)
    ///   * When flipping to opponent: +1 (player 0→1)
    ///   * When restoring player: +1 (player 0→1)
    ///   * When restoring opponent: -1 (opponent 1→0)
    fn update_features<const M: i32, const F: i32>(&mut self, move_pos: usize, flipped: u64) {
        let s = &EVAL_X2F[move_pos];

        for feature in s {
            self.features[feature.0] += M * feature.1;
        }

        let mut remaining_flips = flipped;
        while remaining_flips != 0 {
            let pos = remaining_flips.trailing_zeros() as usize;
            let s = &EVAL_X2F[pos];

            for feature in s {
                self.features[feature.0] += F * feature.1;
            }

            remaining_flips &= remaining_flips - 1;
        }
    }

    /// Update features when doing a move for the player.
    fn do_move_player(&mut self, move_pos: usize, flipped: u64) {
        // Place player's disc on an empty square
        // In base-3: empty (2) -> player (0) requires -2

        // Flip opponent's discs to player's color
        // In base-3: opponent (1) -> player (0) requires -1

        self.update_features::<-2, -1>(move_pos, flipped);
    }

    /// Update features when doing a move for the opponent.
    fn do_move_opponent(&mut self, move_pos: usize, flipped: u64) {
        // Place opponent's disc on an empty square
        // In base-3: empty (2) -> opponent (1) requires -1

        // Flip player's discs to opponent's color
        // In base-3: player (0) -> opponent (1) requires +1

        self.update_features::<-1, 1>(move_pos, flipped);
    }

    /// Update Eval to reflect doing a move.
    /// This is much more efficient than rebuilding an Eval from scratch.
    pub fn do_move(&mut self, index: usize, flipped: u64) {
        DO_MOVE_FUNCTIONS[self.player as usize](self, index, flipped);
        self.empty_index += 1;
        self.swap();
    }

    /// Update Eval to reflect undoing a move.
    fn undo_move_player(&mut self, move_pos: usize, flipped: u64) {
        // Restore empty square from player's disc
        // In base-3: player (0) -> empty (2) requires +2

        // Restore opponent's discs from player's color
        // In base-3: player (0) -> opponent (1) requires +1

        self.update_features::<2, 1>(move_pos, flipped);
    }

    /// Update Eval to reflect undoing a move for the opponent.
    fn undo_move_opponent(&mut self, move_pos: usize, flipped: u64) {
        // Restore empty square from opponent's disc
        // In base-3: opponent (1) -> empty (2) requires +1

        // Restore player's discs from opponent's color
        // In base-3: opponent (1) -> player (0) requires -1

        self.update_features::<1, -1>(move_pos, flipped);
    }

    /// Update Eval to reflect undoing a move.
    pub fn undo_move(&mut self, index: usize, flipped: u64) {
        self.swap();
        UNDO_MOVE_FUNCTIONS[self.player as usize](self, index, flipped);
        self.empty_index -= 1;
    }

    /// Update Eval to reflect passing.
    pub fn pass(&mut self) {
        self.swap();
    }

    /// Compute error value of the evaluation function.
    pub fn sigma(n_empty: i32, depth: i32, probcut_depth: i32) -> f64 {
        let sigma = -0.10026799 * n_empty as f64
            + 0.31027733 * depth as f64
            + -0.57772603 * probcut_depth as f64;

        0.07585621 * sigma * sigma + 1.16492647 * sigma + 5.4171698
    }

    /// Compute the heuristic for the board we're currently evaluating.
    pub fn heuristic(&self) -> i32 {
        let player_index = self.player as usize;
        let empty_index = self.empty_index;
        let weights = &EVAL_WEIGHT[player_index][empty_index];

        let mut score = 0;
        for i in 0..EVAL_N_FEATURES {
            score += weights[self.features[i] as usize] as i32;
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

    /// Get the features of the current evaluation.
    pub fn features(&self) -> &[i32; EVAL_N_FEATURES] {
        &self.features
    }

    /// Get whose turn it is in the position we're evaluating.
    ///
    /// 0 is the player to move when the Eval initially was created.
    /// 1 is the opponent.
    pub fn player(&self) -> i32 {
        self.player
    }
}

#[cfg(test)]
pub mod tests {
    use crate::othello::position::Position;

    use super::*;

    pub fn test_positions() -> Vec<Position> {
        let mut positions = Vec::new();

        for disc_count in 4..=64 {
            for _ in 0..10 {
                positions.push(Position::new_random_with_discs(disc_count));
            }
        }

        positions
    }

    impl Eval {
        /// Create a new eval for the opponent's perspective
        pub fn new_for_opponent(position: &Position) -> Self {
            // Pass position
            let mut passed = *position;
            passed.pass();

            // Create eval for passed position
            // This sets features from the player's perspective
            let mut eval = Eval::new(&passed);

            // Pass again
            eval.pass();
            eval
        }

        /// Validates that the eval state matches a fresh evaluation
        fn validate(&self, position: &Position) {
            assert!(self.player == 0 || self.player == 1);
            assert!(self.empty_index == (60 - position.count_empty()) as usize);

            let fresh_eval = if self.player == 0 {
                Eval::new(position)
            } else {
                Eval::new_for_opponent(position)
            };

            if self.features != fresh_eval.features {
                println!("\ndifferences:");
                for i in 0..EVAL_N_FEATURES {
                    if self.features[i] != fresh_eval.features[i] {
                        println!(
                            "feature {:2}: {:6} != {:6}, diff = {:6}",
                            i,
                            self.features[i],
                            fresh_eval.features[i],
                            self.features[i] - fresh_eval.features[i]
                        );
                    }
                }

                panic!();
            }
        }
    }

    #[test]
    fn test_do_undo_player() {
        for position in test_positions() {
            for move_ in position.iter_move_indices() {
                let mut eval = Eval::new(&position);
                assert_eq!(eval.player, 0);
                eval.validate(&position);

                let mut child = position;
                let flipped = child.do_move(move_);
                eval.do_move(move_, flipped);
                eval.validate(&child);

                eval.undo_move(move_, flipped);
                eval.validate(&position);
            }
        }
    }

    #[test]
    fn test_do_undo_opponent() {
        for position in test_positions() {
            for move_ in position.iter_move_indices() {
                let mut child = position;

                let mut eval = Eval::new(&position);
                child.pass();
                eval.pass();
                assert_eq!(eval.player, 1);
                eval.validate(&child);

                let flipped = child.do_move(move_);
                eval.do_move(move_, flipped);
                eval.validate(&child);

                eval.undo_move(move_, flipped);
                child.undo_move(move_, flipped);
                eval.validate(&child);
            }
        }
    }

    #[test]
    fn test_pass() {
        for position in test_positions() {
            let mut eval = Eval::new(&position);
            let mut child = position;

            eval.validate(&child);

            eval.pass();
            child.pass();
            eval.validate(&child);

            eval.pass();
            child.pass();
            eval.validate(&child);
        }
    }

    #[test]
    fn test_eval() {
        for position in test_positions() {
            for move_ in position.iter_move_indices() {
                let mut eval = Eval::new(&position);
                eval.validate(&position);

                let mut child = position;
                let flipped = child.do_move(move_);
                eval.do_move(move_, flipped);
                eval.validate(&child);
            }
        }
    }

    #[test]
    fn test_getters() {
        let position = Position::new_random_with_discs(32);
        let eval = Eval::new(&position);

        assert_eq!(eval.player(), eval.player);
        assert_eq!(eval.features(), &eval.features);
    }
}
