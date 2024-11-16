#![allow(dead_code)] // TODO

use rand::Rng;
use std::sync::atomic::{AtomicI64, AtomicU64, AtomicU8, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::bot::edax::r#const::{
    ITERATIVE_MIN_EMPTIES, NO_SELECTIVITY, SCORE_INF, SCORE_MAX, SCORE_MIN,
};
use crate::{
    bot::edax::square::QUADRANT_ID,
    collections::{forward_pool_list::ForwardPoolList, hashtable::HashTable, pool_list::PoolList},
    othello::{position::Position, squares::*},
};

use super::r#const::{NodeType, GAME_SIZE};
use super::{
    eval::Eval,
    r#const::{Stop, BLACK, LEVEL},
    square::{Square, PRESORTED_X},
};

/// Like Move in Edax
#[derive(Default, Copy, Clone)]
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

/// Like Result in Edax
#[derive(Clone)]
pub struct SearchResult {
    /// Index of the best move
    pub move_: usize,

    /// Score of the best move
    pub score: i32,

    /// Number of moves left to search
    pub n_moves_left: usize,

    /// If true, the move is from the opening book
    pub book_move: bool,

    /// Total moves to search
    pub n_moves: i32,

    /// Score bounds for each move
    pub bound: [Bound; 66],

    /// Number of nodes searched
    pub n_nodes: u64,

    /// Time spent searching in milliseconds
    pub time: i64,

    /// Depth of the search
    pub depth: i32,

    /// Selectivity of the search
    pub selectivity: i32,

    /// Principal variation of the search
    pub pv: Line,
}

impl Default for SearchResult {
    fn default() -> Self {
        Self {
            move_: NO_MOVE,
            score: 0,
            n_moves_left: 0,
            book_move: false,
            n_moves: 0,
            bound: [Bound::default(); 66],
            n_nodes: 0,
            time: 0,
            depth: 0,
            selectivity: 0,
            pv: Line::default(),
        }
    }
}

// Like unnamed struct field `options` of Search in Edax
#[derive(Default)]
pub struct SerachOptions {
    /// Depth of search
    depth: i32,

    /// Selectivity of search
    selectivity: i32,

    /// If true, preserves hashtable date when `Search::run()` is called
    keep_date: bool,
}

/// Like unnamed struct field `time` of Search in Edax
pub struct SearchTime {
    /// Time spent thinking in milliseconds
    spent: AtomicI64,
}

impl Default for SearchTime {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchTime {
    fn new() -> Self {
        let now = -Search::clock();

        Self {
            // Use negative so we can add current time to it later to get the elapsed time
            spent: AtomicI64::new(-now),
        }
    }
}

#[derive(Default, Copy, Clone)]
pub struct Bound {
    /// Lower bound
    lower: i32,

    /// Upper bound
    upper: i32,
}

/// Like Line in Edax
#[derive(Clone)]
pub struct Line {
    /// Moves in the line
    moves: Vec<u8>,

    /// Color of first player to move in the line
    color: i32,
}

impl Line {
    fn new(color: i32) -> Self {
        Self {
            moves: Vec::new(),
            color,
        }
    }
}

impl Default for Line {
    fn default() -> Self {
        Self::new(0)
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

    /// Search options
    pub options: SerachOptions,

    /// Stop condition
    pub stop: AtomicU8,

    /// Number of nodes searched by this search instance
    pub n_nodes: AtomicU64,

    /// Number of nodes searched by parallel searches spawned by this search instance
    pub child_nodes: AtomicU64,

    /// Time elapsed since search started
    pub time: SearchTime,

    /// Main hash table
    pub hash_table: HashTable,

    /// Principal variation table
    pub pv_table: HashTable,

    /// Hash table for shallow search
    pub shallow_table: HashTable,

    /// Height of the search tree
    pub height: i32,

    /// Type of the node at `height`
    pub node_type: [NodeType; GAME_SIZE],

    /// Depth of PV extension
    pub depth_pv_extension: i32,

    /// Stability bound
    pub stability_bound: Bound,

    /// Selectivity level of the search
    pub selectivity: i32,

    /// Depth of the search
    pub depth: i32,
}

impl Default for Search {
    fn default() -> Self {
        Self::new(&Position::new(), BLACK, 0)
    }
}

/// Like Search in Edax
impl Search {
    /// Like search_init() in Edax, but also does the following:
    /// - sets `player` and `position` like search_set_board() in Edax
    /// - sets `movelist` like search_get_movelist() in Edax
    /// - calls `setup()` to initialize other fields
    pub fn new(position: &Position, player: i32, level: i32) -> Self {
        let mut search = Self {
            player,
            position: *position,
            n_empties: position.count_empty() as i32,
            result: SearchResult::default(),
            empties: PoolList::default(),
            movelist: Self::get_movelist(position),
            parity: 0,
            eval: Eval::new(position),
            x_to_empties: [0; 64],
            options: SerachOptions::default(),
            stop: AtomicU8::new(Stop::StopEnd as u8),
            n_nodes: AtomicU64::new(0),
            child_nodes: AtomicU64::new(0),
            time: SearchTime::default(),
            hash_table: HashTable::new(1 << 21),
            pv_table: HashTable::new(1 << 17),
            shallow_table: HashTable::new(1 << 21),
            height: 0,
            depth_pv_extension: 0,
            stability_bound: Bound::default(),
            node_type: [NodeType::default(); GAME_SIZE],
            selectivity: 0,
            depth: 0,
        };

        search.setup();
        search.set_level(level, search.n_empties);

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
    fn set_level(&mut self, level: i32, n_empties: i32) {
        self.options.depth = LEVEL[level as usize][n_empties as usize].depth;
        self.options.selectivity = LEVEL[level as usize][n_empties as usize].selectivity;
    }

    /// Like search_clock() in Edax
    fn clock() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
    }

    /// Like get_pv_extension() in Edax
    fn get_pv_extension(&self, depth: i32) -> i32 {
        if depth >= self.n_empties || depth <= 9 {
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
    fn bound(&self, score: i32) -> i32 {
        score.clamp(self.stability_bound.lower, self.stability_bound.upper)
    }

    /// Like search_eval_0() in Edax
    fn eval_0(&self) -> i32 {
        self.eval.heuristic()
    }

    /// Like search_count_nodes() in Edax
    fn count_nodes(&self) -> u64 {
        self.n_nodes.load(Ordering::Relaxed) + self.child_nodes.load(Ordering::Relaxed)
    }

    /// Like statistics_sum_nodes() in Edax
    fn sum_nodes(&mut self) {
        // TODO #8 Add stats when we do parallel searches
    }

    /// Like search_run() in Edax
    pub fn run(&mut self) -> SearchResult {
        self.stop.store(Stop::Running as u8, Ordering::Relaxed);
        self.n_nodes = AtomicU64::new(0);
        self.child_nodes = AtomicU64::new(0);
        self.time = SearchTime::new();

        if !self.options.keep_date {
            self.hash_table.clear();
            self.pv_table.clear();
            self.shallow_table.clear();
        }

        self.height = 0;
        self.node_type[self.height as usize] = NodeType::PvNode;
        self.depth_pv_extension = self.get_pv_extension(0);
        self.stability_bound.upper = SCORE_MAX - 2 * self.position.count_opponent_stable_discs();
        self.stability_bound.lower = 2 * self.position.count_player_stable_discs() - SCORE_MAX;
        self.result.score = self.bound(self.eval_0());
        self.result.n_moves_left = self.movelist.len();
        self.result.n_moves = self.movelist.len() as i32;
        self.result.book_move = false;

        if self.movelist.is_empty() {
            self.result.bound[PASS] = Bound {
                lower: SCORE_MIN,
                upper: SCORE_MAX,
            };
        } else {
            for move_ in self.movelist.iter() {
                self.result.bound[move_.x as usize] = Bound {
                    lower: SCORE_MIN,
                    upper: SCORE_MAX,
                };
            }
        }

        self.iterative_deepening(SCORE_MIN, SCORE_MAX);

        self.result.n_nodes = self.count_nodes();

        if self.stop.load(Ordering::Relaxed) == Stop::Running as u8 {
            self.stop.store(Stop::StopEnd as u8, Ordering::Relaxed);
        }

        self.time.spent.fetch_add(Self::clock(), Ordering::Relaxed);
        self.result.time = self.time.spent.load(Ordering::Relaxed);

        self.sum_nodes();

        self.result.clone()
    }

    /// Computes final score knowing the number of empty squares.
    ///
    /// Like search_solve() in Edax
    pub fn solve(&self) -> i32 {
        self.position.final_score_with_empty(self.n_empties)
    }

    /// Like get_last_level() in Edax
    fn get_last_level(&self, _depth: &i32, _selectivity: &i32) -> bool {
        todo!() // TODO
    }

    /// Like search_adjust_time() in Edax
    fn adjust_time(&self, _new_search: bool) {
        todo!() // TODO
    }

    /// Like record_best_move() in Edax
    fn record_best_move(
        &mut self,
        _position: &Position,
        _move_: &Move,
        _alpha: i32,
        _beta: i32,
        _depth: i32,
    ) {
        todo!() // TODO
    }

    /// Evaluates the movelist in `self.movelist`
    ///
    /// Like movelist_evaluate() in Edax, except we don't send movelist as a parameter due to borrowing issues
    fn evaluate_own_movelist(&mut self, _alpha: i32, _beta: i32) {
        // TODO use self.movelist

        // TODO retrieve hash data from self.pv_table again
        todo!() // TODO
    }

    /// Like search_continue() in Edax
    fn continue_search(&self) -> bool {
        todo!() // TODO
    }

    /// Like iterative_deepening() in Edax
    #[allow(unused_assignments)] // TODO
    fn iterative_deepening(&mut self, alpha: i32, beta: i32) {
        self.result.move_ = NO_MOVE;
        self.result.score = -SCORE_INF;
        self.result.depth = -1;
        self.result.selectivity = 0;
        self.result.time = 0;
        self.result.n_nodes = 0;
        self.result.pv = Line::new(self.player);

        // Game is over
        if self.movelist.is_empty() && !self.position.opponent_has_moves() {
            self.result.move_ = NO_MOVE;
            self.result.score = self.solve();
            self.result.depth = self.n_empties;
            self.result.selectivity = NO_SELECTIVITY;
            self.result.time = self.time.spent.load(Ordering::Relaxed);
            self.result.n_nodes = self.count_nodes();
            self.result.bound[NO_MOVE] = Bound {
                lower: self.result.score,
                upper: self.result.score,
            };
            self.result.pv = Line::new(self.player);
            return;
        }

        let mut score = self.bound(self.eval_0());
        let mut end = self.options.depth;
        if end >= self.n_empties {
            end = self.n_empties - ITERATIVE_MIN_EMPTIES + 2;
            if end <= 0 {
                end = 2 - (self.n_empties & 1);
            }
        }
        let mut start = 6 - (end & 1);
        if start > end - 2 {
            start = end - 2;
        }
        if start <= 0 {
            start = 2 - (end & 1);
        }

        self.result.selectivity = if self.options.depth > 10 {
            0
        } else {
            NO_SELECTIVITY
        };

        let mut old_depth = 0;
        let mut old_selectivity = self.result.selectivity;

        if let Some(hash_data) = self.pv_table.get(&self.position) {
            old_depth = hash_data.depth as i32;
            old_selectivity = hash_data.selectivity as i32;

            if hash_data.lower == hash_data.upper {
                if self.get_last_level(&old_depth, &old_selectivity) {
                    start = old_depth;
                    self.selectivity = old_selectivity;
                }
                score = hash_data.lower as i32;
            } else {
                self.adjust_time(true);
            }
        } else {
            self.adjust_time(false);
        }

        if self.selectivity > self.options.selectivity {
            self.selectivity = self.options.selectivity;
        }

        if start > self.options.depth {
            start = self.options.depth;
        }
        if start > self.n_empties {
            start = self.n_empties;
        }
        if start < self.n_empties {
            if (start & 1) != (end & 1) {
                start += 1;
            }
            if start <= 0 {
                start = 2 - (end & 1);
            }
            if start > end {
                start = end;
            }
        }

        if self.movelist.is_empty() {
            let mut bestmove = MOVE_PASS;
            bestmove.score = score;

            // Create local copy to avoid borrowing issues
            let position = self.position;
            self.record_best_move(&position, &bestmove, alpha, beta, old_depth);
        } else {
            if end == 0 {
                // shuffle the movelist
                for move_ in self.movelist.iter_mut() {
                    move_.score = rand::thread_rng().gen::<i32>() & 0x7fffffff;
                }
            } else {
                // sort the moves
                self.evaluate_own_movelist(alpha, start);
            }
            self.movelist.sort();

            // Create local copy to avoid borrowing issues
            let mut bestmove = *self.movelist.first().unwrap();
            bestmove.score = score;

            let position = self.position;
            self.record_best_move(&position, &bestmove, alpha, beta, old_depth);
        }

        self.selectivity = old_selectivity;

        // Special case: level 0
        if end == 0 {
            return;
        }

        // midgame: iterative depth
        let mut depth = start;
        while depth < end {
            self.depth_pv_extension = self.get_pv_extension(depth);
            score = self.aspiration_search(alpha, beta, depth, score);

            if !self.continue_search() {
                return;
            }

            if score.abs() >= SCORE_MAX - 1
                && depth > end - ITERATIVE_MIN_EMPTIES
                && self.options.depth >= self.n_empties
            {
                break;
            }

            depth += 2;
        }
        self.depth = end;

        // Switch to endgame
        if self.options.depth >= self.n_empties {
            self.depth = self.n_empties;
        }

        // iterative selectivity

        // TODO pretend we have time, since we don't do time management
        let has_time = true;

        while self.selectivity <= self.options.selectivity {
            // Check if we should jump to exact endgame for faster solving
            if self.depth == self.n_empties
                && ((self.depth < 21 && self.selectivity >= 1)
                    || (self.depth < 24 && self.selectivity >= 2)
                    || (self.depth < 27 && self.selectivity >= 3)
                    || (self.depth < 30 && self.selectivity >= 4)
                    || (has_time && self.depth < 30 && self.selectivity >= 2)
                    || score.abs() >= SCORE_MAX)
            {
                self.selectivity = self.options.selectivity;
            }

            if self.selectivity == self.options.selectivity {
                self.adjust_time(true);
            }

            score = self.aspiration_search(alpha, beta, self.depth, score);

            if !self.continue_search() {
                return;
            }

            self.selectivity += 1;
        }

        // Ensure selectivity doesn't exceed options.selectivity
        if self.selectivity > self.options.selectivity {
            self.selectivity = self.options.selectivity;
        }
    }

    /// Like aspiration_search() in Edax
    fn aspiration_search(&mut self, _alpha: i32, _beta: i32, _depth: i32, _score: i32) -> i32 {
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
        verify_search_invariants(&search_default, &Position::new(), BLACK);

        // Test new() with a custom position
        let custom_pos = Position::new_from_bitboards(0, 0xFFFFFFFFFFFFFFFF);
        let search_new = Search::new(&custom_pos, BLACK, 0);
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
