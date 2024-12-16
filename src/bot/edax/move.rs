use std::cell::Cell;

use crate::othello::{
    position::Position,
    squares::{NO_MOVE, PASS},
};

use super::r#const::SCORE_INF;

/// Details of a move on a particular position.
/// This is mostly used as the item inside MoveList.
///
/// Like Move in Edax
#[derive(Default, Clone, Debug)]
pub struct Move {
    /// Bitset representation of the flipped squares
    pub flipped: u64,

    /// Index of the move
    pub x: i32,

    /// Score of the move
    pub score: Cell<i32>,

    /// Cost of the move
    pub cost: Cell<u32>,
    // TODO #15 Further optimization: add a u64 with bitset of played move
}

impl PartialEq for Move {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

impl Move {
    /// Create a new move from a position and a square.
    pub fn new(position: &Position, x: i32) -> Self {
        Self {
            flipped: position.get_flipped(x as usize),
            x,
            score: Cell::new(0),
            cost: Cell::new(0),
        }
    }

    /// Create a new pass move.
    ///
    /// Like MOVE_PASS in Edax
    pub fn new_pass() -> Self {
        Self::new_pass_with_score(-SCORE_INF)
    }

    /// Create a new move with a minimum score.
    /// Useful as initial value when iterating over moves.
    pub fn new_min_score() -> Self {
        Self::new_no_move_with_score(-SCORE_INF)
    }

    /// Create a new pass move with a custom score.
    pub fn new_pass_with_score(score: i32) -> Self {
        Self {
            flipped: 0,
            x: PASS as i32,
            score: Cell::new(score),
            cost: Cell::new(0),
        }
    }

    /// Create a new no-move move with a custom score.
    pub fn new_no_move_with_score(score: i32) -> Self {
        Self {
            flipped: 0,
            x: NO_MOVE as i32,
            score: Cell::new(score),
            cost: Cell::new(0),
        }
    }

    /// Check if the move is legal for a given position.
    ///
    /// Like board_check_move() in Edax
    pub fn is_legal(&self, position: &Position) -> bool {
        // TODO #15 Further optimization: this function checks too many things for certain call-sites

        let x = self.x as usize;

        if x == PASS {
            return !position.has_moves();
        }

        if (position.player | position.opponent) & (1u64 << x) != 0 {
            return false;
        }

        position.get_flipped(x) == self.flipped
    }

    /// Check if the move captures all opponent's pieces.
    ///
    /// Like move_wipeout() in Edax
    pub fn is_wipeout(&self, position: &Position) -> bool {
        self.flipped == position.opponent
    }

    /// Update the position with the move.
    ///
    /// Like board_update() in Edax
    pub fn update(&self, position: &mut Position) {
        if self.x == PASS as i32 {
            position.pass();
        } else {
            position.player |= self.flipped | (1u64 << self.x as usize);
            position.opponent ^= self.flipped;
            std::mem::swap(&mut position.player, &mut position.opponent);
        }
    }

    /// Restore the position to before the move.
    ///
    /// Like board_restore() in Edax
    pub fn restore(&self, position: &mut Position) {
        if self.x == PASS as i32 {
            position.pass();
        } else {
            std::mem::swap(&mut position.player, &mut position.opponent);
            position.player &= !(self.flipped | (1u64 << self.x as usize));
            position.opponent |= self.flipped;
        }
    }
}
