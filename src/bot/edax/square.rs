use crate::othello::squares::*;

/// A square on the board.
/// This is used for keeping track of remaining empty squares.
///
/// Like SquareList in Edax, except we don't store links in the same struct.
/// In this implementation, Square is used as an item in EmptiesList.
#[derive(Default, Clone, PartialEq, Debug)]
pub struct Square {
    /// Bitset representation of the square
    pub b: u64,

    /// Index of the square
    pub x: i32,

    /// Parity quadrant of the square
    pub quadrant: u32,
}

impl Square {
    /// Create a new square from an index.
    pub fn new(x: usize) -> Self {
        Self {
            b: 1 << x,
            x: x as i32,
            quadrant: QUADRANT_ID[x],
        }
    }
}

/// Maps the coordinates of a square to its quadrant.
#[rustfmt::skip]
pub const QUADRANT_ID: [u32; 66] = [
    1, 1, 1, 1, 2, 2, 2, 2,
    1, 1, 1, 1, 2, 2, 2, 2,
    1, 1, 1, 1, 2, 2, 2, 2,
    1, 1, 1, 1, 2, 2, 2, 2,
    4, 4, 4, 4, 8, 8, 8, 8,
    4, 4, 4, 4, 8, 8, 8, 8,
    4, 4, 4, 4, 8, 8, 8, 8,
    4, 4, 4, 4, 8, 8, 8, 8,
    0, 0
];

/// Presorted square coordinates.
///
/// Like PRESORTED_X in Edax
#[rustfmt::skip]
pub const PRESORTED_X: [usize; 64] = [
    A1, A8, H1, H8,                 /* Corner */
    C4, C5, D3, D6, E3, E6, F4, F5, /* E */
    C3, C6, F3, F6,                 /* D */
    A3, A6, C1, C8, F1, F8, H3, H6, /* A */
    A4, A5, D1, D8, E1, E8, H4, H5, /* B */
    B4, B5, D2, D7, E2, E7, G4, G5, /* G */
    B3, B6, C2, C7, F2, F7, G3, G6, /* F */
    A2, A7, B1, B8, G1, G8, H2, H7, /* C */
    B2, B7, G2, G7,                 /* X */
    D4, E4, D5, E5,                 /* center */
];

/// Square values. Used for move sorting.
///
/// Like SQUARE_VALUE in Edax
#[rustfmt::skip]
pub const SQUARE_VALUE: [i32; 64] = [
	18,  4,  16, 12, 12, 16,  4, 18,
	 4,  2,   6,  8,  8,  6,  2,  4,
	16,  6,  14, 10, 10, 14,  6, 16,
	12,  8,  10,  0,  0, 10,  8, 12,
	12,  8,  10,  0,  0, 10,  8, 12,
	16,  6,  14, 10, 10, 14,  6, 16,
	 4,  2,   6,  8,  8,  6,  2,  4,
    18,  4,  16, 12, 12, 16,  4, 18
];
