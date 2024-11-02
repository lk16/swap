// This is adapted from https://github.com/abulmo/edax-reversi/blob/master/src/eval.c

use crate::othello::position::Position;
use lazy_static::lazy_static;

const A1: usize = 0;
const B1: usize = 1;
const C1: usize = 2;
const D1: usize = 3;
const E1: usize = 4;
const F1: usize = 5;
const G1: usize = 6;
const H1: usize = 7;
const A2: usize = 8;
const B2: usize = 9;
const C2: usize = 10;
const D2: usize = 11;
const E2: usize = 12;
const F2: usize = 13;
const G2: usize = 14;
const H2: usize = 15;
const A3: usize = 16;
const B3: usize = 17;
const C3: usize = 18;
const D3: usize = 19;
const E3: usize = 20;
const F3: usize = 21;
const G3: usize = 22;
const H3: usize = 23;
const A4: usize = 24;
const B4: usize = 25;
const C4: usize = 26;
const D4: usize = 27;
const E4: usize = 28;
const F4: usize = 29;
const G4: usize = 30;
const H4: usize = 31;
const A5: usize = 32;
const B5: usize = 33;
const C5: usize = 34;
const D5: usize = 35;
const E5: usize = 36;
const F5: usize = 37;
const G5: usize = 38;
const H5: usize = 39;
const A6: usize = 40;
const B6: usize = 41;
const C6: usize = 42;
const D6: usize = 43;
const E6: usize = 44;
const F6: usize = 45;
const G6: usize = 46;
const H6: usize = 47;
const A7: usize = 48;
const B7: usize = 49;
const C7: usize = 50;
const D7: usize = 51;
const E7: usize = 52;
const F7: usize = 53;
const G7: usize = 54;
const H7: usize = 55;
const A8: usize = 56;
const B8: usize = 57;
const C8: usize = 58;
const D8: usize = 59;
const E8: usize = 60;
const F8: usize = 61;
const G8: usize = 62;
const H8: usize = 63;

/// The number of features in the evaluation
pub const EVAL_N_FEATURES: usize = 47;

lazy_static! {
    pub static ref EVAL_F2X: [Vec<usize>; EVAL_N_FEATURES] = [
        vec![A1, B1, A2, B2, C1, A3, C2, B3, C3],
        vec![H1, G1, H2, G2, F1, H3, F2, G3, F3],
        vec![A8, A7, B8, B7, A6, C8, B6, C7, C6],
        vec![H8, H7, G8, G7, H6, F8, G6, F7, F6],
        vec![A5, A4, A3, A2, A1, B2, B1, C1, D1, E1],
        vec![H5, H4, H3, H2, H1, G2, G1, F1, E1, D1],
        vec![A4, A5, A6, A7, A8, B7, B8, C8, D8, E8],
        vec![H4, H5, H6, H7, H8, G7, G8, F8, E8, D8],
        vec![B2, A1, B1, C1, D1, E1, F1, G1, H1, G2],
        vec![B7, A8, B8, C8, D8, E8, F8, G8, H8, G7],
        vec![B2, A1, A2, A3, A4, A5, A6, A7, A8, B7],
        vec![G2, H1, H2, H3, H4, H5, H6, H7, H8, G7],
        vec![A1, C1, D1, C2, D2, E2, F2, E1, F1, H1],
        vec![A8, C8, D8, C7, D7, E7, F7, E8, F8, H8],
        vec![A1, A3, A4, B3, B4, B5, B6, A5, A6, A8],
        vec![H1, H3, H4, G3, G4, G5, G6, H5, H6, H8],
        vec![A2, B2, C2, D2, E2, F2, G2, H2],
        vec![A7, B7, C7, D7, E7, F7, G7, H7],
        vec![B1, B2, B3, B4, B5, B6, B7, B8],
        vec![G1, G2, G3, G4, G5, G6, G7, G8],
        vec![A3, B3, C3, D3, E3, F3, G3, H3],
        vec![A6, B6, C6, D6, E6, F6, G6, H6],
        vec![C1, C2, C3, C4, C5, C6, C7, C8],
        vec![F1, F2, F3, F4, F5, F6, F7, F8],
        vec![A4, B4, C4, D4, E4, F4, G4, H4],
        vec![A5, B5, C5, D5, E5, F5, G5, H5],
        vec![D1, D2, D3, D4, D5, D6, D7, D8],
        vec![E1, E2, E3, E4, E5, E6, E7, E8],
        vec![A1, B2, C3, D4, E5, F6, G7, H8],
        vec![A8, B7, C6, D5, E4, F3, G2, H1],
        vec![B1, C2, D3, E4, F5, G6, H7],
        vec![H2, G3, F4, E5, D6, C7, B8],
        vec![A2, B3, C4, D5, E6, F7, G8],
        vec![G1, F2, E3, D4, C5, B6, A7],
        vec![C1, D2, E3, F4, G5, H6],
        vec![A3, B4, C5, D6, E7, F8],
        vec![F1, E2, D3, C4, B5, A6],
        vec![H3, G4, F5, E6, D7, C8],
        vec![D1, E2, F3, G4, H5],
        vec![A4, B5, C6, D7, E8],
        vec![E1, D2, C3, B4, A5],
        vec![H4, G5, F6, E7, D8],
        vec![D1, C2, B3, A4],
        vec![A5, B6, C7, D8],
        vec![E1, F2, G3, H4],
        vec![H5, G6, F7, E8],
        vec![],
    ];
}

// array to convert coordinates into feature
#[rustfmt::skip]
lazy_static! {
    pub static ref EVAL_X2F: [Vec<(usize, i32)>; 65] = [
        vec![( 0,    6561), ( 4,     243), ( 8,    6561), (10,    6561), (12,   19683), (14,   19683), (28,    2187)],  /* a1 */
        vec![( 0,    2187), ( 4,      27), ( 8,    2187), (18,    2187), (30,     729)],                                /* b1 */
        vec![( 0,      81), ( 4,       9), ( 8,     729), (12,    6561), (22,    2187), (34,     243)],                 /* c1 */
        vec![( 4,       3), ( 5,       1), ( 8,     243), (12,    2187), (26,    2187), (38,      81), (42,      27)],  /* d1 */
        vec![( 4,       1), ( 5,       3), ( 8,      81), (12,       9), (27,    2187), (40,      81), (44,      27)],  /* e1 */
        vec![( 1,      81), ( 5,       9), ( 8,      27), (12,       3), (23,    2187), (36,     243)],                 /* f1 */
        vec![( 1,    2187), ( 5,      27), ( 8,       9), (19,    2187), (33,     729)],                                /* g1 */
        vec![( 1,    6561), ( 5,     243), ( 8,       3), (11,    6561), (12,       1), (15,   19683), (29,       1)],  /* h1 */
        vec![( 0,     729), ( 4,     729), (10,    2187), (16,    2187), (32,     729)],                                /* a2 */
        vec![( 0,     243), ( 4,      81), ( 8,   19683), (10,   19683), (16,     729), (18,     729), (28,     729)],  /* b2 */
        vec![( 0,       9), (12,     729), (16,     243), (22,     729), (30,     243), (42,       9)],                 /* c2 */
        vec![(12,     243), (16,      81), (26,     729), (34,      81), (40,      27)],                                /* d2 */
        vec![(12,      81), (16,      27), (27,     729), (36,      81), (38,      27)],                                /* e2 */
        vec![( 1,       9), (12,      27), (16,       9), (23,     729), (33,     243), (44,       9)],                 /* f2 */
        vec![( 1,     243), ( 5,      81), ( 8,       1), (11,   19683), (16,       3), (19,     729), (29,       3)],  /* g2 */
        vec![( 1,     729), ( 5,     729), (11,    2187), (16,       1), (31,     729)],                                /* h2 */
        vec![( 0,      27), ( 4,    2187), (10,     729), (14,    6561), (20,    2187), (35,     243)],                 /* a3 */
        vec![( 0,       3), (14,     729), (18,     243), (20,     729), (32,     243), (42,       3)],                 /* b3 */
        vec![( 0,       1), (20,     243), (22,     243), (28,     243), (40,       9)],                                /* c3 */
        vec![(20,      81), (26,     243), (30,      81), (36,      27)],                                               /* d3 */
        vec![(20,      27), (27,     243), (33,      81), (34,      27)],                                               /* e3 */
        vec![( 1,       1), (20,       9), (23,     243), (29,       9), (38,       9)],                                /* f3 */
        vec![( 1,       3), (15,     729), (19,     243), (20,       3), (31,     243), (44,       3)],                 /* g3 */
        vec![( 1,      27), ( 5,    2187), (11,     729), (15,    6561), (20,       1), (37,     243)],                 /* h3 */
        vec![( 4,    6561), ( 6,   19683), (10,     243), (14,    2187), (24,    2187), (39,      81), (42,       1)],  /* a4 */
        vec![(14,     243), (18,      81), (24,     729), (35,      81), (40,       3)],                                /* b4 */
        vec![(22,      81), (24,     243), (32,      81), (36,       9)],                                               /* c4 */
        vec![(24,      81), (26,      81), (28,      81), (33,      27)],                                               /* d4 */
        vec![(24,      27), (27,      81), (29,      27), (30,      27)],                                               /* e4 */
        vec![(23,      81), (24,       9), (31,      81), (34,       9)],                                               /* f4 */
        vec![(15,     243), (19,      81), (24,       3), (37,      81), (38,       3)],                                /* g4 */
        vec![( 5,    6561), ( 7,   19683), (11,     243), (15,    2187), (24,       1), (41,      81), (44,       1)],  /* h4 */
        vec![( 4,   19683), ( 6,    6561), (10,      81), (14,       9), (25,    2187), (40,       1), (43,      27)],  /* a5 */
        vec![(14,      81), (18,      27), (25,     729), (36,       3), (39,      27)],                                /* b5 */
        vec![(22,      27), (25,     243), (33,       9), (35,      27)],                                               /* c5 */
        vec![(25,      81), (26,      27), (29,      81), (32,      27)],                                               /* d5 */
        vec![(25,      27), (27,      27), (28,      27), (31,      27)],                                               /* e5 */
        vec![(23,      27), (25,       9), (30,       9), (37,      27)],                                               /* f5 */
        vec![(15,      81), (19,      27), (25,       3), (34,       3), (41,      27)],                                /* g5 */
        vec![( 5,   19683), ( 7,    6561), (11,      81), (15,       9), (25,       1), (38,       1), (45,      27)],  /* h5 */
        vec![( 2,      81), ( 6,    2187), (10,      27), (14,       3), (21,    2187), (36,       1)],                 /* a6 */
        vec![( 2,       9), (14,      27), (18,       9), (21,     729), (33,       3), (43,       9)],                 /* b6 */
        vec![( 2,       1), (21,     243), (22,       9), (29,     243), (39,       9)],                                /* c6 */
        vec![(21,      81), (26,       9), (31,       9), (35,       9)],                                               /* d6 */
        vec![(21,      27), (27,       9), (32,       9), (37,       9)],                                               /* e6 */
        vec![( 3,       1), (21,       9), (23,       9), (28,       9), (41,       9)],                                /* f6 */
        vec![( 3,       9), (15,      27), (19,       9), (21,       3), (30,       3), (45,       9)],                 /* g6 */
        vec![( 3,      81), ( 7,    2187), (11,      27), (15,       3), (21,       1), (34,       1)],                 /* h6 */
        vec![( 2,    2187), ( 6,     729), (10,       9), (17,    2187), (33,       1)],                                /* a7 */
        vec![( 2,     243), ( 6,      81), ( 9,   19683), (10,       1), (17,     729), (18,       3), (29,     729)],  /* b7 */
        vec![( 2,       3), (13,     729), (17,     243), (22,       3), (31,       3), (43,       3)],                 /* c7 */
        vec![(13,     243), (17,      81), (26,       3), (37,       3), (39,       3)],                                /* d7 */
        vec![(13,      81), (17,      27), (27,       3), (35,       3), (41,       3)],                                /* e7 */
        vec![( 3,       3), (13,      27), (17,       9), (23,       3), (32,       3), (45,       3)],                 /* f7 */
        vec![( 3,     243), ( 7,      81), ( 9,       1), (11,       1), (17,       3), (19,       3), (28,       3)],  /* g7 */
        vec![( 3,    2187), ( 7,     729), (11,       9), (17,       1), (30,       1)],                                /* h7 */
        vec![( 2,    6561), ( 6,     243), ( 9,    6561), (10,       3), (13,   19683), (14,       1), (29,    2187)],  /* a8 */
        vec![( 2,     729), ( 6,      27), ( 9,    2187), (18,       1), (31,       1)],                                /* b8 */
        vec![( 2,      27), ( 6,       9), ( 9,     729), (13,    6561), (22,       1), (37,       1)],                 /* c8 */
        vec![( 6,       3), ( 7,       1), ( 9,     243), (13,    2187), (26,       1), (41,       1), (43,       1)],  /* d8 */
        vec![( 6,       1), ( 7,       3), ( 9,      81), (13,       9), (27,       1), (39,       1), (45,       1)],  /* e8 */
        vec![( 3,      27), ( 7,       9), ( 9,      27), (13,       3), (23,       1), (35,       1)],                 /* f8 */
        vec![( 3,     729), ( 7,      27), ( 9,       9), (19,       1), (32,       1)],                                /* g8 */
        vec![( 3,    6561), ( 7,     243), ( 9,       3), (11,       3), (13,       1), (15,       1), (28,       1)],  /* h8 */
        vec![( 0,       0)] // <- PASS
    ];
}

/// feature offset
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

#[derive(Clone, PartialEq, Debug)]
pub struct Eval {
    /// The features of the position
    pub features: [i32; EVAL_N_FEATURES],

    /// The player to evaluate from
    pub player: i32,
}

impl Eval {
    pub fn new(position: &Position) -> Self {
        let mut eval = Self {
            features: [0; EVAL_N_FEATURES],
            player: 0,
        };

        for i in 0..EVAL_N_FEATURES {
            eval.features[i] = 0;
            for j in 0..EVAL_F2X[i].len() {
                let c = position.get_square_color(EVAL_F2X[i][j]) as i32;
                eval.features[i] = eval.features[i] * 3 + c;
            }
            eval.features[i] += EVAL_OFFSET[i];
        }

        eval
    }

    fn swap(&mut self) {
        self.player = 1 - self.player;
    }

    fn update0(&mut self, move_pos: usize, flipped: u64) {
        let s = &EVAL_X2F[move_pos];

        for feature in s {
            self.features[feature.0] -= 2 * feature.1;
        }

        // Update for all flipped pieces
        let mut remaining_flips = flipped;
        while remaining_flips != 0 {
            let pos = remaining_flips.trailing_zeros() as usize;
            let s = &EVAL_X2F[pos];

            for feature in s {
                self.features[feature.0] -= feature.1;
            }

            remaining_flips &= remaining_flips - 1;
        }
    }

    fn update1(&mut self, move_pos: usize, flipped: u64) {
        let s = &EVAL_X2F[move_pos];

        for feature in s {
            self.features[feature.0] -= feature.1;
        }

        // Update for all flipped pieces
        let mut remaining_flips = flipped;
        while remaining_flips != 0 {
            let pos = remaining_flips.trailing_zeros() as usize;
            let s = &EVAL_X2F[pos];

            for feature in s {
                self.features[feature.0] += feature.1;
            }

            remaining_flips &= remaining_flips - 1;
        }
    }

    pub fn update(&mut self, move_pos: usize, flipped: u64) {
        const UPDATE_FUNCTIONS: [fn(&mut Eval, usize, u64); 2] = [Eval::update0, Eval::update1];
        UPDATE_FUNCTIONS[self.player as usize](self, move_pos, flipped);
        self.swap();
    }

    fn restore0(&mut self, move_pos: usize, flipped: u64) {
        let s = &EVAL_X2F[move_pos];

        for feature in s {
            self.features[feature.0] += 2 * feature.1;
        }

        // Update for all flipped pieces
        let mut remaining_flips = flipped;
        while remaining_flips != 0 {
            let pos = remaining_flips.trailing_zeros() as usize;
            let s = &EVAL_X2F[pos];

            for feature in s {
                self.features[feature.0] += feature.1;
            }

            remaining_flips &= remaining_flips - 1;
        }
    }

    fn restore1(&mut self, move_pos: usize, flipped: u64) {
        let s = &EVAL_X2F[move_pos];

        for feature in s {
            self.features[feature.0] += feature.1;
        }

        // Update for all flipped pieces
        let mut remaining_flips = flipped;
        while remaining_flips != 0 {
            let pos = remaining_flips.trailing_zeros() as usize;
            let s = &EVAL_X2F[pos];

            for feature in s {
                self.features[feature.0] -= feature.1;
            }

            remaining_flips &= remaining_flips - 1;
        }
    }

    pub fn restore(&mut self, move_pos: usize, flipped: u64) {
        const RESTORE_FUNCTIONS: [fn(&mut Eval, usize, u64); 2] = [Eval::restore0, Eval::restore1];
        self.swap();
        RESTORE_FUNCTIONS[self.player as usize](self, move_pos, flipped);
    }

    pub fn pass(&mut self) {
        self.swap();
    }

    pub fn eval_sigma(n_empty: i32, depth: i32, probcut_depth: i32) -> f64 {
        let sigma = -0.10026799 * n_empty as f64
            + 0.31027733 * depth as f64
            + -0.57772603 * probcut_depth as f64;

        0.07585621 * sigma * sigma + 1.16492647 * sigma + 5.4171698
    }
}

#[cfg(test)]
mod tests {
    use crate::othello::position::Position;

    use super::*;

    #[test]
    fn test_update0() {
        let mut position = Position::new();
        let initial_eval = Eval::new(&position);
        let mut eval = initial_eval.clone();

        let flipped = position.do_move(19);
        eval.update0(19, flipped);
        eval.restore0(19, flipped);

        assert_eq!(eval, initial_eval);
    }

    #[test]
    fn test_update1() {
        let mut position = Position::new();
        let initial_eval = Eval::new(&position);
        let mut eval = initial_eval.clone();

        let flipped = position.do_move(19);
        eval.update1(19, flipped);
        eval.restore1(19, flipped);

        assert_eq!(eval, initial_eval);
    }
}
