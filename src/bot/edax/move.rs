use crate::othello::{position::Position, squares::PASS};

use super::r#const::SCORE_INF;

/// Like Move in Edax
#[derive(Default, Copy, Clone)]
pub struct Move {
    /// Bitset representation of the flipped squares
    pub flipped: u64,

    /// Index of the move
    pub x: i32,

    /// Score of the move
    pub score: i32,

    /// Cost of the move
    pub cost: u32,
    // TODO #15 Further optimization: add a u64 with bitset of played move
}

impl PartialOrd for Move {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.score.partial_cmp(&other.score)
    }
}

impl PartialEq for Move {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

/// Like MOVE_PASS in Edax
pub const MOVE_PASS: Move = Move {
    flipped: 0,
    x: PASS as i32,
    score: -SCORE_INF,
    cost: 0,
};

impl Move {
    pub fn new(position: &Position, x: i32) -> Self {
        Self {
            flipped: position.get_flipped(x as usize),
            x,
            score: 0,
            cost: 0,
        }
    }

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

    /// Like move_wipeout() in Edax
    pub fn is_wipeout(&self, position: &Position) -> bool {
        self.flipped == position.opponent
    }

    /// Like board_update() in Edax
    pub fn apply(&self, position: &mut Position) {
        if self.x == PASS as i32 {
            position.pass();
        } else {
            position.player |= self.flipped | (1u64 << self.x as usize);
            position.opponent ^= self.flipped;
            std::mem::swap(&mut position.player, &mut position.opponent);
        }
    }
}
