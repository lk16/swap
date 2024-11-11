#![allow(dead_code)] // TODO

use crate::{
    bot::edax::square::QUADRANT_ID,
    collections::{forward_pool_list::ForwardPoolList, pool_list::PoolList},
    othello::{position::Position, squares::*},
};

use super::{
    eval::Eval,
    r#const::BLACK,
    square::{Square, PRESORTED_X},
};

/// Like Move in Edax
#[derive(Default)]
pub struct Move {
    /// Bitset representation of the flipped squares
    flipped: u64,

    /// Index of the move
    x: i32,

    /// Score of the move
    score: i32,

    /// Cost of the move
    cost: u32,
}

/// Like Result in Edax
pub struct SearchResult {
    pub move_: usize,
}

impl Default for SearchResult {
    fn default() -> Self {
        Self { move_: NO_MOVE }
    }
}

/// Like Search in Edax
pub struct Search {
    /// Color of player to move
    pub player: i32,

    /// Search position, changes during search
    pub position: Position,

    /// Number of empty squares in `position`
    pub n_empties: i32,

    /// Result of search, changes during search
    pub result: SearchResult,

    /// Empty squares in `position`
    pub empties: PoolList<Square, 64>,

    /// Legal moves in `position`
    pub movelist: ForwardPoolList<Move, 64>,

    /// Quadrant parity
    pub parity: u32,

    /// Evaluation of the position
    pub eval: Eval,

    /// Index of the empty square in `empties`
    pub x_to_empties: [usize; 64],
}

impl Default for Search {
    fn default() -> Self {
        Self::new(&Position::default(), BLACK)
    }
}

/// Like Search in Edax
impl Search {
    /// Like search_init() in Edax, but also does the following:
    /// - sets `player` and `position` like search_set_board() in Edax
    /// - sets `movelist` like search_get_movelist() in Edax
    /// - calls `setup()` to initialize other fields
    pub fn new(position: &Position, player: i32) -> Self {
        let mut search = Self {
            player,
            position: *position,
            n_empties: position.count_empty() as i32,
            result: SearchResult::default(),
            empties: PoolList::default(),
            movelist: Self::get_movelist(position),
            parity: 0,
            eval: Eval::new(position), // TODO don't store position both in Eval and Search
            x_to_empties: [0; 64],
        };

        search.setup();

        search
    }

    /// Like search_get_movelist() in Edax
    fn get_movelist(position: &Position) -> ForwardPoolList<Move, 64> {
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

    /// Like search_setup() in Edax
    fn setup(&mut self) {
        let e = !(self.position.player | self.position.opponent);

        for (i, &x) in PRESORTED_X.iter().enumerate() {
            if e & (1 << x) != 0 {
                let square = Square {
                    b: 1 << x,
                    x: x as i32,
                    quadrant: QUADRANT_ID[x],
                };

                self.empties.push(square);

                self.x_to_empties[x] = i;
            }
        }

        self.parity = 0;
        for empty in self.empties.iter() {
            self.parity ^= empty.x as u32;
        }
    }

    /// Like search_set_level() in Edax
    pub fn set_level(&mut self, _level: u32, _n_empties: i32) {
        todo!() // TODO
    }

    /// Like search_run() in Edax
    pub fn run(&mut self) {
        todo!() // TODO
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_initialization() {
        // Test default()
        let search_default = Search::default();
        verify_search_invariants(&search_default, &Position::default(), BLACK);

        // Test new() with a custom position
        let custom_pos = Position::new_from_bitboards(0, 0xFFFFFFFFFFFFFFFF);
        let search_new = Search::new(&custom_pos, BLACK);
        verify_search_invariants(&search_new, &custom_pos, BLACK);
    }

    fn verify_search_invariants(
        search: &Search,
        expected_position: &Position,
        expected_player: i32,
    ) {
        // Check position and player
        assert_eq!(search.position, *expected_position);
        assert_eq!(search.player, expected_player);

        // Check n_empties matches actual empty squares count
        assert_eq!(search.n_empties, expected_position.count_empty() as i32);

        // Check parity calculation
        let mut expected_parity = 0;
        for empty in search.empties.iter() {
            expected_parity ^= empty.x as u32;
        }
        assert_eq!(search.parity, expected_parity);

        // Check eval has same position
        assert_eq!(*search.eval.position(), search.position);

        // Check empties contains only empty squares
        let empty_squares = !(search.position.player | search.position.opponent);
        for empty in search.empties.iter() {
            assert_ne!(empty.b & empty_squares, 0);
        }

        // Check x_to_empties is correctly set up
        for (i, empty) in search.empties.iter().enumerate() {
            assert_eq!(search.x_to_empties[empty.x as usize], i);
        }
    }
}
