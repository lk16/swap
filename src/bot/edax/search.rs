#![allow(dead_code)] // TODO

use rand::Rng;
use std::sync::atomic::{AtomicI64, AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::bot::edax::r#const::{
    ITERATIVE_MIN_EMPTIES, NO_SELECTIVITY, SCORE_INF, SCORE_MAX, SCORE_MIN,
};
use crate::bot::edax::square::SQUARE_VALUE;
use crate::collections::hashtable::{HashData, StoreArgs};
use crate::othello::position::GameState;
use crate::{
    bot::edax::square::QUADRANT_ID,
    collections::{forward_pool_list::ForwardPoolList, hashtable::HashTable, pool_list::PoolList},
    othello::{position::Position, squares::*},
};

use super::eval::EVAL_N_FEATURES;
use super::r#const::{
    NodeType, GAME_SIZE, NWS_STABILITY_THRESHOLD, PVS_STABILITY_THRESHOLD, SORT_ALPHA_DELTA,
};
use super::weights::EVAL_WEIGHT;
use super::{
    eval::Eval,
    r#const::{Stop, LEVEL},
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
    fn new(position: &Position, x: i32) -> Self {
        Self {
            flipped: position.get_flipped(x as usize),
            x,
            score: 0,
            cost: 0,
        }
    }

    /// Like board_check_move() in Edax
    fn is_legal(&self, position: &Position) -> bool {
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
    fn is_wipeout(&self, position: &Position) -> bool {
        self.flipped == position.opponent
    }

    /// Like board_update() in Edax
    fn apply(&self, position: &mut Position) {
        if self.x == PASS as i32 {
            position.pass();
        } else {
            position.player |= self.flipped | (1u64 << self.x as usize);
            position.opponent ^= self.flipped;
            std::mem::swap(&mut position.player, &mut position.opponent);
        }
    }
}

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

// Like unnamed struct field `options` of Search in Edax, does not change during search
pub struct SerachOptions {
    /// Requested depth of search
    depth: i32,

    /// Selectivity of search
    selectivity: i32,

    /// If true, preserves hashtable date when `Search::run()` is called
    keep_date: bool,

    /// Depth to use for multipv
    multipv_depth: i32,
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

    fn push(&mut self, x: u8) {
        self.moves.push(x);
    }
}

impl Default for Line {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Mutable search state that changes frequently during search
pub struct SearchState {
    /// Search position, changes during search
    position: Position,

    /// Number of empty squares in `position`
    n_empties: i32,

    /// Empty squares in `position`
    empties: PoolList<Square, 64>,

    /// Legal moves in `position`
    movelist: ForwardPoolList<Move, 64>,

    /// Quadrant parity
    parity: u32,

    /// Evaluation of the position
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
    fn new(position: &Position) -> Self {
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
                let square = Square {
                    b: 1 << x,
                    x: x as i32,
                    quadrant: QUADRANT_ID[x],
                };

                empties.push(square);

                x_to_empties[x] = i;
            }
        }

        for empty in empties.iter() {
            parity ^= empty.x as u32;
        }

        (empties, x_to_empties, parity)
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

    /// Like get_pv_extension() in Edax
    fn get_pv_extension(&self, depth: i32) -> i32 {
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
    fn bound(&self, score: i32) -> i32 {
        score.clamp(self.stability_bound.lower, self.stability_bound.upper)
    }

    /// Like update_pass_midgame() in Edax
    fn update_pass_midgame(&mut self) {
        const NEXT_NODE_TYPE: [NodeType; 3] =
            [NodeType::CutNode, NodeType::AllNode, NodeType::CutNode];

        self.position.pass();
        self.eval.pass();
        self.height += 1;
        self.node_type[self.height as usize] =
            NEXT_NODE_TYPE[self.node_type[(self.height - 1) as usize] as usize];
    }

    /// Like restore_pass_midgame() in Edax
    fn restore_pass_midgame(&mut self) {
        self.position.pass();
        self.eval.pass();
        self.height -= 1;
    }

    /// Like search_eval_0() in Edax
    fn eval_0(&self) -> i32 {
        self.eval.heuristic()
    }

    /// Like search_eval_1() in Edax
    fn eval_1(&mut self, alpha: i32, mut beta: i32) -> i32 {
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
    fn eval_2(&mut self, mut alpha: i32, beta: i32) -> i32 {
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
    fn solve(&self) -> i32 {
        self.position.final_score_with_empty(self.n_empties)
    }

    /// Like search_update_midgame() in Edax
    fn update_midgame(&mut self, move_: &Move) {
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
    fn restore_midgame(&mut self, move_: &Move) {
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
    fn stability_cutoff_nws(&self, alpha: i32) -> Option<i32> {
        if alpha >= NWS_STABILITY_THRESHOLD[self.n_empties as usize] {
            let score = SCORE_MAX - 2 * self.position.count_opponent_stable_discs();
            if score <= alpha {
                return Some(score);
            }
        }

        None
    }

    /// Like search_SC_PVS() in Edax
    fn stability_cutoff_pvs(&mut self, alpha: i32, beta: &mut i32) -> Option<i32> {
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

/// Search configuration that changes less frequently
pub struct SearchConfig {
    /// Search options, does not change during search
    options: SerachOptions,

    /// Selectivity level of the search
    selectivity: i32,

    /// Depth of the search
    depth: i32,

    /// Probcut recursionlevel
    probcut_level: i32,
}

impl SearchConfig {
    /// Like search_set_level() in Edax, sets other fields to default
    fn new(level: i32, n_empties: i32) -> Self {
        Self {
            options: SerachOptions {
                depth: LEVEL[level as usize][n_empties as usize].depth,
                selectivity: LEVEL[level as usize][n_empties as usize].selectivity,
                keep_date: false,
                multipv_depth: 0,
            },
            selectivity: 0,
            depth: 0,
            probcut_level: 0,
        }
    }
}

/// Like Search in Edax
pub struct Search {
    /// Color of player to move
    pub player: i32,

    /// Frequently changing search state
    pub state: Mutex<SearchState>,

    /// Search configuration
    pub config: RwLock<SearchConfig>,

    /// Result of search, changes during search
    pub result: Arc<Mutex<SearchResult>>,

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
}

/// Like Search in Edax
impl Search {
    /// Like search_init() in Edax, but also does the following:
    /// - sets `player` and `position` like search_set_board() in Edax
    /// - sets `movelist` like search_get_movelist() in Edax
    /// - calls `setup()` to initialize other fields
    pub fn new(position: &Position, player: i32, level: i32) -> Arc<Self> {
        let state = SearchState::new(position);
        let n_empties = state.n_empties;

        Arc::new(Self {
            player,
            state: Mutex::new(state),
            config: RwLock::new(SearchConfig::new(level, n_empties)),
            result: Arc::new(Mutex::new(SearchResult::default())),
            stop: AtomicU8::new(Stop::StopEnd as u8),
            n_nodes: AtomicU64::new(0),
            child_nodes: AtomicU64::new(0),
            time: SearchTime::default(),
            hash_table: HashTable::new(1 << 21),
            pv_table: HashTable::new(1 << 17),
            shallow_table: HashTable::new(1 << 21),
        })
    }

    /// Like search_clock() in Edax
    fn clock() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
    }

    /// Like search_count_nodes() in Edax
    fn count_nodes(&self) -> u64 {
        self.n_nodes.load(Ordering::Relaxed) + self.child_nodes.load(Ordering::Relaxed)
    }

    /// Like statistics_sum_nodes() in Edax
    fn sum_nodes(self: &Arc<Self>) {
        // TODO #8 Add stats when we do parallel searches
    }

    /// Like search_run() in Edax
    pub fn run(self: Arc<Self>) -> SearchResult {
        self.stop.store(Stop::Running as u8, Ordering::Relaxed);
        self.n_nodes.store(0, Ordering::Relaxed);
        self.child_nodes.store(0, Ordering::Relaxed);

        self.time.spent.store(0, Ordering::Relaxed);

        if !self.config.read().unwrap().options.keep_date {
            self.hash_table.clear();
            self.pv_table.clear();
            self.shallow_table.clear();
        }

        {
            let mut state = self.state.lock().unwrap();
            state.height = 0;
            state.node_type[0] = NodeType::PvNode;
            state.depth_pv_extension = state.get_pv_extension(0);
            state.stability_bound.upper =
                SCORE_MAX - 2 * state.position.count_opponent_stable_discs();
            state.stability_bound.lower =
                2 * state.position.count_player_stable_discs() - SCORE_MAX;

            let mut result = self.result.lock().unwrap();
            result.score = state.bound(state.eval_0());
            result.n_moves_left = state.movelist.len();
            result.n_moves = state.movelist.len() as i32;
            result.book_move = false;

            if state.movelist.is_empty() {
                result.bound[PASS] = Bound {
                    lower: SCORE_MIN,
                    upper: SCORE_MAX,
                };
            } else {
                for move_ in state.movelist.iter() {
                    result.bound[move_.x as usize] = Bound {
                        lower: SCORE_MIN,
                        upper: SCORE_MAX,
                    };
                }
            }
        }

        // TODO consider sending guards as arguments to avoid locking twice here and in other functions.
        self.iterative_deepening(SCORE_MIN, SCORE_MAX);

        {
            let mut result = self.result.lock().unwrap();

            result.n_nodes = self.count_nodes();

            if self.stop.load(Ordering::Relaxed) == Stop::Running as u8 {
                self.stop.store(Stop::StopEnd as u8, Ordering::Relaxed);
            }

            self.time.spent.fetch_add(Self::clock(), Ordering::Relaxed);
            result.time = self.time.spent.load(Ordering::Relaxed);

            self.sum_nodes();

            result.clone()
        }
    }

    /// Returns Some((depth, selectivity)) if found in hash tables, None otherwise
    ///
    /// Like get_last_level() in Edax
    fn get_last_level(&self) -> Option<(i32, i32)> {
        let mut position = self.state.lock().unwrap().position;

        let mut depth: i32 = -1;
        let mut selectivity: i32 = -1;

        let mut i = 0;
        while i < 4 {
            let hash_data = if let Some(hash_data) = self.pv_table.get(&position) {
                hash_data
            } else if let Some(hash_data) = self.hash_table.get(&position) {
                hash_data
            } else {
                break;
            };

            let d = hash_data.depth as i32 + i;
            let s = hash_data.selectivity as i32;

            if d > depth {
                depth = d;
            }

            if s > selectivity {
                selectivity = s;
            }

            // Edax constructs Move object here, no need for that.
            let x = hash_data.move_[0] as usize;
            position.do_move(x);

            if x != PASS {
                i += 1;
            }
        }

        if depth > -1 && selectivity > -1 {
            Some((depth, selectivity))
        } else {
            None
        }
    }

    /// Like search_adjust_time() in Edax
    fn adjust_time(&self, _new_search: bool) {
        // TODO #14 Implement time management
    }

    /// Like guess_move() in Edax
    fn guess_move(&self, _position: &Position) -> usize {
        unreachable!() // We don't support this.
    }

    /// Like search_time() in Edax
    fn get_time_spent(&self) -> i64 {
        if self.stop.load(Ordering::Relaxed) != Stop::StopEnd as u8 {
            Self::clock() + self.time.spent.load(Ordering::Relaxed)
        } else {
            self.time.spent.load(Ordering::Relaxed)
        }
    }

    /// Like record_best_move() in Edax
    fn record_best_move(
        self: &Arc<Self>,
        position: &Position,
        bestmove: &Move,
        alpha: i32,
        beta: i32,
        depth: i32,
    ) {
        let mut result = self.result.lock().unwrap();

        {
            // Create local copy to avoid borrowing issues
            let mut bound = result.bound[bestmove.x as usize];

            result.move_ = bestmove.x as usize;
            result.score = bestmove.score;

            if result.score < beta && result.score < bound.upper {
                bound.upper = result.score;
            }
            if result.score > alpha && result.score > bound.lower {
                bound.lower = result.score;
            }
            if bound.lower > bound.upper {
                if result.score < beta {
                    bound.upper = result.score;
                } else {
                    let state = self.state.lock().unwrap();
                    bound.upper = state.stability_bound.upper;
                }
                if result.score > alpha {
                    bound.lower = result.score;
                } else {
                    let state = self.state.lock().unwrap();
                    bound.lower = state.stability_bound.lower;
                }
            }

            result.bound[bestmove.x as usize] = bound;
        }

        let mut expected_depth = depth;
        result.depth = depth;

        let config = self.config.read().unwrap();
        let expected_selectivity = config.selectivity;
        result.selectivity = config.selectivity;
        drop(config);

        let mut expected_bound = result.bound[bestmove.x as usize];

        result.pv = Line::new(self.player);
        let mut x = bestmove.x as usize;

        // NOTE: we don't guess the PV, like in Edax.
        let guess_pv = false;

        let mut fail_low = bestmove.score <= alpha;
        let mut position = *position;

        // TODO #15 Further optimization: x should never be NO_MOVE here
        while x != NO_MOVE {
            // TODO #15 Further optimization: constructing a Move here is unnecessary
            let move_ = Move::new(&position, x as i32);

            // TODO #15 Further optimization: a move should not be illegal here, since we just created it
            if !move_.is_legal(&position) {
                break;
            }

            position.do_move(x);
            expected_depth -= 1;

            // Swap and negate bounds
            expected_bound = Bound {
                upper: -expected_bound.lower,
                lower: -expected_bound.upper,
            };

            fail_low = !fail_low;
            result.pv.push(x as u8);

            // Try to get hash data from either table
            let hash_data = self
                .pv_table
                .get(&position)
                .or_else(|| self.hash_table.get(&position));

            // Determine next move
            x = if let Some(hash_data) = hash_data {
                // Check if hash data meets our criteria
                if hash_data.depth as i32 >= expected_depth
                    && hash_data.selectivity as i32 >= expected_selectivity
                    && hash_data.upper as i32 <= expected_bound.upper
                    && hash_data.lower as i32 >= expected_bound.lower
                {
                    hash_data.move_[0] as usize
                } else {
                    break;
                }
            } else if guess_pv && fail_low {
                self.guess_move(&position)
            } else {
                break;
            };
        }

        result.time = self.get_time_spent();
        result.n_nodes = self.count_nodes();
    }

    /// Like movelist_evaluate() in Edax
    fn evaluate_movelist(
        self: &Arc<Self>,
        movelist: &mut ForwardPoolList<Move, 64>,
        hash_data: &HashData,
        alpha: i32,
        _beta: i32, // TODO this is unused ?!?
    ) {
        let state = self.state.lock().unwrap();
        let config = self.config.read().unwrap();

        let mut min_depth = 9;
        if state.n_empties <= 27 {
            min_depth += (30 - state.n_empties) / 3;
        }

        let sort_depth = if config.depth >= min_depth {
            let mut sort_depth = (config.depth - 15) / 3;
            if let Some(hash_data) = self.pv_table.get(&state.position) {
                if (hash_data.upper as i32) < alpha {
                    sort_depth -= 2;
                }
            }
            if state.n_empties >= 27 {
                sort_depth += 1;
            }

            sort_depth.clamp(0, 6)
        } else {
            -1
        };

        let sort_alpha = SCORE_MIN.max(alpha - SORT_ALPHA_DELTA);
        for move_ in movelist.iter_mut() {
            self.evaluate_move(move_, hash_data, sort_alpha, sort_depth);
        }
    }

    /// Evaluate a move to sort it.
    /// This sets `score` for the move.
    ///
    /// Like move_evaluate() in Edax
    fn evaluate_move(
        self: &Arc<Self>,
        move_: &mut Move,
        hash_data: &HashData,
        sort_alpha: i32,
        sort_depth: i32,
    ) {
        const WEIGHT_HASH: i32 = 1 << 15;
        const WEIGHT_EVAL: i32 = 1 << 15;
        const WEIGHT_MOBILITY: i32 = 1 << 15;
        const WEIGHT_CORNER_STABILITY: i32 = 1 << 11;
        const WEIGHT_EDGE_STABILITY: i32 = 1 << 11;
        const WEIGHT_POTENTIAL_MOBILITY: i32 = 1 << 5;
        const WEIGHT_LOW_PARITY: i32 = 1 << 3;
        const WEIGHT_MID_PARITY: i32 = 1 << 2;
        const WEIGHT_HIGH_PARITY: i32 = 1 << 1;

        let mut state = self.state.lock().unwrap();

        if move_.is_wipeout(&state.position) {
            move_.score = 1 << 30;
        } else if move_.x == hash_data.move_[0] as i32 {
            move_.score = 1 << 29;
        } else if move_.x == hash_data.move_[1] as i32 {
            move_.score = 1 << 28;
        } else {
            move_.score = SQUARE_VALUE[move_.x as usize];
            if state.n_empties < 12 && (state.parity & QUADRANT_ID[move_.x as usize]) != 0 {
                move_.score += WEIGHT_LOW_PARITY;
            } else if state.n_empties < 21 && (state.parity & QUADRANT_ID[move_.x as usize]) != 0 {
                move_.score += WEIGHT_MID_PARITY;
            } else if state.n_empties < 30 && (state.parity & QUADRANT_ID[move_.x as usize]) != 0 {
                move_.score += WEIGHT_HIGH_PARITY;
            }

            if sort_depth < 0 {
                // TODO #15 Optimize: use flipped discs from `move_` for doing and undoing move
                let flipped = state.position.do_move(move_.x as usize);

                move_.score +=
                    (36 - state.position.potential_mobility()) * WEIGHT_POTENTIAL_MOBILITY;
                move_.score += state.position.opponent_corner_stability() * WEIGHT_CORNER_STABILITY;
                move_.score += (36 - state.position.weighted_mobility()) * WEIGHT_MOBILITY;

                state.position.undo_move(move_.x as usize, flipped);
            } else {
                let mut config = self.config.write().unwrap();
                let selectivity = config.selectivity;
                config.selectivity = NO_SELECTIVITY;
                drop(config);

                state.update_midgame(move_);
                move_.score +=
                    (36 - state.position.potential_mobility()) * WEIGHT_POTENTIAL_MOBILITY; // potential mobility
                move_.score += state.position.opponent_edge_stability() * WEIGHT_EDGE_STABILITY; // edge stability
                move_.score += (36 - state.position.weighted_mobility()) * WEIGHT_MOBILITY; // real mobility

                move_.score += match sort_depth {
                    0 => ((SCORE_MAX - state.eval_0()) >> 2) * WEIGHT_EVAL,
                    1 => ((SCORE_MAX - state.eval_1(SCORE_MIN, -sort_alpha)) >> 1) * WEIGHT_EVAL,
                    2 => ((SCORE_MAX - state.eval_2(SCORE_MIN, -sort_alpha)) >> 1) * WEIGHT_EVAL,
                    _ => {
                        let mut score = (SCORE_MAX
                            - self.pvs_shallow(SCORE_MIN, -sort_alpha, sort_depth))
                            * WEIGHT_EVAL;

                        if self.hash_table.get(&state.position).is_some() {
                            score += WEIGHT_HASH;
                        }

                        score
                    }
                };
                state.restore_midgame(move_);

                self.config.write().unwrap().selectivity = selectivity;
            }
        }
    }

    /// Like pvs_shallow() in Edax
    fn pvs_shallow(self: &Arc<Self>, alpha: i32, mut beta: i32, depth: i32) -> i32 {
        let mut cost = -(self.n_nodes.load(Ordering::Relaxed) as i64);

        let mut state = self.state.lock().unwrap();

        if depth == 2 {
            return state.eval_2(alpha, beta);
        }

        if let Some(score) = state.stability_cutoff_pvs(alpha, &mut beta) {
            return score;
        }

        let mut movelist = SearchState::get_movelist(&state.position);

        let mut bestmove;
        let mut bestscore;

        if movelist.is_empty() {
            if state.position.opponent_has_moves() {
                state.update_pass_midgame();
                bestscore = -self.pvs_shallow(-beta, -alpha, depth);
                bestmove = PASS;
            } else {
                bestscore = state.solve();
                bestmove = NO_MOVE;
            }
        } else {
            let hash_data = self.shallow_table.get_or_default(&state.position);

            self.evaluate_movelist(&mut movelist, &hash_data, alpha, beta);
            movelist.sort();

            bestscore = -SCORE_INF;
            bestmove = NO_MOVE;
            let mut lower = alpha;

            for move_ in movelist.iter() {
                state.update_midgame(move_);

                let score = if bestscore == -SCORE_INF {
                    -self.pvs_shallow(-beta, -lower, depth - 1)
                } else {
                    let mut score = -self.nws_shallow_with_shallow_table(-lower - 1, depth - 1);
                    if alpha < score && score < beta {
                        score = -self.pvs_shallow(-beta, -lower, depth - 1);
                    }

                    score
                };

                state.restore_midgame(move_);

                if score > bestscore {
                    bestscore = score;
                    bestmove = move_.x as usize;

                    if bestscore >= beta {
                        break;
                    } else if bestscore > lower {
                        lower = bestscore;
                    }
                }
            }
        }

        cost += self.n_nodes.load(Ordering::Relaxed) as i64;

        self.shallow_table.store(&StoreArgs {
            position: &state.position,
            depth,
            selectivity: self.config.read().unwrap().selectivity,
            cost: cost.ilog2() as i32,
            alpha,
            beta,
            score: bestscore,
            move_: bestmove as i32,
        });

        bestscore
    }

    /// Like nws_shallow() in Edax, but using self.shallow_table
    fn nws_shallow_with_shallow_table(self: &Arc<Self>, alpha: i32, depth: i32) -> i32 {
        // TODO inline this
        self.nws_shallow(alpha, depth, &self.shallow_table)
    }

    /// Like nws_shallow() in Edax, but using self.hash_table
    fn nws_shallow_with_hash_table(self: &Arc<Self>, alpha: i32, depth: i32) -> i32 {
        // TODO inline this
        self.nws_shallow(alpha, depth, &self.hash_table)
    }

    fn nws_shallow(self: &Arc<Self>, alpha: i32, depth: i32, table: &HashTable) -> i32 {
        let selectivity = self.config.read().unwrap().selectivity;

        let beta = alpha + 1;
        let mut cost = -(self.n_nodes.load(Ordering::Relaxed) as i64);

        let mut state = self.state.lock().unwrap();

        if depth == 2 {
            return state.eval_2(alpha, beta);
        }

        if let Some(score) = state.stability_cutoff_nws(alpha) {
            return score;
        }

        let hash_data = table.get(&state.position);

        if let Some(ref hash_data) = hash_data {
            if let Some(score) =
                Self::transposition_cutoff_nws(hash_data, depth, selectivity, alpha)
            {
                return score;
            }
        }

        let hash_data = hash_data.unwrap_or_default();

        let mut movelist = SearchState::get_movelist(&state.position);

        let mut bestscore;
        let mut bestmove;

        if movelist.is_empty() {
            if state.position.opponent_has_moves() {
                state.update_pass_midgame();
                bestscore = -self.nws_shallow(beta, depth - 1, table);
                bestmove = PASS;
                state.restore_pass_midgame();
            } else {
                bestscore = state.solve();
                bestmove = NO_MOVE;
            }
        } else {
            self.evaluate_movelist(&mut movelist, &hash_data, alpha, beta);
            movelist.sort();

            bestscore = -SCORE_INF;
            bestmove = NO_MOVE;

            for move_ in movelist.iter() {
                state.update_midgame(move_);
                let score = -self.nws_shallow(-beta, depth - 1, table);
                state.restore_midgame(move_);

                if score > bestscore {
                    bestscore = score;
                    bestmove = move_.x as usize;

                    if bestscore >= beta {
                        break;
                    }
                }
            }
        }

        cost += self.n_nodes.load(Ordering::Relaxed) as i64;

        let store_args = StoreArgs {
            position: &state.position,
            depth,
            selectivity,
            cost: cost.ilog2() as i32,
            alpha,
            beta,
            score: bestscore,
            move_: bestmove as i32,
        };

        table.store(&store_args);

        bestscore
    }

    /// Like search_TC_NWS() in Edax
    fn transposition_cutoff_nws(
        hash_data: &HashData,
        depth: i32,
        selectivity: i32,
        alpha: i32,
    ) -> Option<i32> {
        if hash_data.selectivity as i32 >= selectivity && hash_data.depth as i32 >= depth {
            if alpha < hash_data.lower as i32 {
                return Some(hash_data.lower as i32);
            }
            if alpha >= hash_data.upper as i32 {
                return Some(hash_data.upper as i32);
            }
        }

        None
    }

    /// Like search_continue() in Edax
    fn continue_search(&self) -> bool {
        // TODO #14 when we support time management, we need to check if we have time left
        self.stop.load(Ordering::Relaxed) == Stop::Running as u8
    }

    /// Like iterative_deepening() in Edax
    fn iterative_deepening(self: &Arc<Self>, alpha: i32, beta: i32) {
        let mut result = self.result.lock().unwrap();

        result.move_ = NO_MOVE;
        result.score = -SCORE_INF;
        result.depth = -1;
        result.selectivity = 0;
        result.time = 0;
        result.n_nodes = 0;
        result.pv = Line::new(self.player);

        let mut state = self.state.lock().unwrap();

        // Game is over
        if state.movelist.is_empty() && !state.position.opponent_has_moves() {
            result.move_ = NO_MOVE;
            result.score = state.solve();
            result.depth = state.n_empties;
            result.selectivity = NO_SELECTIVITY;
            result.time = self.time.spent.load(Ordering::Relaxed);
            result.n_nodes = self.count_nodes();
            result.bound[NO_MOVE] = Bound {
                lower: result.score,
                upper: result.score,
            };
            result.pv = Line::new(self.player);
            return;
        }

        let options_depth = self.config.read().unwrap().options.depth;

        let mut score = state.bound(state.eval_0());
        let mut end = options_depth;
        if end >= state.n_empties {
            end = state.n_empties - ITERATIVE_MIN_EMPTIES + 2;
            if end <= 0 {
                end = 2 - (state.n_empties & 1);
            }
        }
        let mut start = 6 - (end & 1);
        if start > end - 2 {
            start = end - 2;
        }
        if start <= 0 {
            start = 2 - (end & 1);
        }

        result.selectivity = if options_depth > 10 {
            0
        } else {
            NO_SELECTIVITY
        };

        let mut old_depth = 0;
        let mut old_selectivity = result.selectivity;

        // Release mutex, we don't need it anymore
        drop(result);

        if let Some(hash_data) = self.pv_table.get(&state.position) {
            old_depth = hash_data.depth as i32;
            old_selectivity = hash_data.selectivity as i32;

            if hash_data.lower == hash_data.upper {
                if let Some((depth, selectivity)) = self.get_last_level() {
                    start = depth;
                    self.config.write().unwrap().selectivity = selectivity;
                }
                score = hash_data.lower as i32;
            } else {
                self.adjust_time(true);
            }
        } else {
            self.adjust_time(false);
        }

        {
            let mut config = self.config.write().unwrap();
            config.selectivity = config.selectivity.min(config.options.selectivity);
            config.options.depth = config.options.depth.min(options_depth);
        }

        start = start.min(state.n_empties);

        if start < state.n_empties {
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

        if state.movelist.is_empty() {
            let mut bestmove = MOVE_PASS;
            bestmove.score = score;

            // Create local copy to avoid borrowing issues
            let position = state.position;
            self.record_best_move(&position, &bestmove, alpha, beta, old_depth);
        } else {
            if end == 0 {
                // shuffle the movelist
                for move_ in state.movelist.iter_mut() {
                    move_.score = rand::thread_rng().gen::<i32>() & 0x7fffffff;
                }
            } else {
                // Clone movelist to avoid borrowing issues
                let mut movelist = state.movelist.clone();

                // Get hash data from pv_table
                let hash_data = self.pv_table.get_or_default(&state.position);

                // Set `score` for all moves in movelist
                self.evaluate_movelist(&mut movelist, &hash_data, alpha, start);

                // Replace updated movelist
                state.movelist = movelist;
            }
            state.movelist.sort();

            // Create local copy to avoid borrowing issues
            let mut bestmove = *state.movelist.first().unwrap();
            bestmove.score = score;

            let position = state.position;
            self.record_best_move(&position, &bestmove, alpha, beta, old_depth);
        }

        self.config.write().unwrap().selectivity = old_selectivity;

        // Special case: level 0
        if end == 0 {
            return;
        }

        // midgame: iterative depth
        let mut depth = start;
        while depth < end {
            state.depth_pv_extension = state.get_pv_extension(depth);
            score = self.aspiration_search(alpha, beta, depth, score);

            if !self.continue_search() {
                return;
            }

            if score.abs() >= SCORE_MAX - 1
                && depth > end - ITERATIVE_MIN_EMPTIES
                && self.config.read().unwrap().options.depth >= state.n_empties
            {
                break;
            }

            depth += 2;
        }
        self.config.write().unwrap().depth = end;

        // Switch to endgame
        if self.config.read().unwrap().options.depth >= state.n_empties {
            self.config.write().unwrap().depth = state.n_empties;
        }

        // iterative selectivity

        // TODO #14 pretend we have time, since we don't do time management yet
        let has_time = true;

        loop {
            // Take write lock to check/update selectivity
            let mut config = self.config.write().unwrap();

            if config.selectivity > config.options.selectivity {
                break;
            }

            // Check if we should jump to exact endgame for faster solving
            if config.depth == state.n_empties
                && ((config.depth < 21 && config.selectivity >= 1)
                    || (config.depth < 24 && config.selectivity >= 2)
                    || (config.depth < 27 && config.selectivity >= 3)
                    || (config.depth < 30 && config.selectivity >= 4)
                    || (has_time && config.depth < 30 && config.selectivity >= 2)
                    || score.abs() >= SCORE_MAX)
            {
                config.selectivity = config.options.selectivity;
            }

            let current_depth = config.depth;
            let current_selectivity = config.selectivity;

            if current_selectivity == config.options.selectivity {
                self.adjust_time(true);
            }

            // Drop the lock before calling aspiration_search
            drop(config);

            score = self.aspiration_search(alpha, beta, current_depth, score);

            if !self.continue_search() {
                return;
            }

            // Take the lock again to increment selectivity
            let mut config = self.config.write().unwrap();
            config.selectivity = current_selectivity + 1;
        }

        // Ensure selectivity doesn't exceed options.selectivity
        let mut config = self.config.write().unwrap();
        if config.selectivity > config.options.selectivity {
            config.selectivity = config.options.selectivity;
        }
    }

    /// Like aspiration_search() in Edax
    fn aspiration_search(
        self: &Arc<Self>,
        mut alpha: i32,
        mut beta: i32,
        depth: i32,
        mut score: i32,
    ) -> i32 {
        let state = self.state.lock().unwrap();

        if Self::is_depth_solving(depth, state.n_empties) {
            if alpha & 1 != 0 {
                alpha -= 1;
            }
            if beta & 1 != 0 {
                beta += 1;
            }
        }

        let config = self.config.read().unwrap();

        if depth <= config.options.multipv_depth {
            alpha = SCORE_MIN;
            beta = SCORE_MAX;
        }

        let mut high = SCORE_MAX.min(state.stability_bound.upper + 2);
        let mut low = SCORE_MIN.max(state.stability_bound.lower - 2);

        alpha = alpha.max(low);
        beta = beta.min(high);
        score = score.clamp(low, high);
        score = score.clamp(alpha, beta);

        let mut result = self.result.lock().unwrap();

        for move_ in state.movelist.iter() {
            result.bound[move_.x as usize] = Bound {
                lower: low,
                upper: high,
            };
        }

        drop(result);

        let width = {
            let mut width = 10 - depth;
            width = width.min(1);

            if width & 1 != 0 && depth == state.n_empties {
                width += 1;
            }

            width
        };

        let mut left: i32;
        let mut right: i32;

        for i in 0..10 {
            let old_score = score;

            if depth < config.options.multipv_depth || beta - alpha <= 2 * width {
                score = self.pvs_root(alpha, beta, depth);
            } else {
                left = if i <= 0 { 1 } else { i } * width;
                right = left;

                loop {
                    low = (score - left).max(alpha);
                    high = (score + right).min(beta);

                    if low >= high {
                        break;
                    }

                    if low >= SCORE_MAX {
                        low = SCORE_MAX - 1;
                    }

                    if high <= SCORE_MIN {
                        high = SCORE_MIN + 1;
                    }

                    score = self.pvs_root(low, high, depth);

                    if self.stop.load(Ordering::Relaxed) != Stop::Running as u8 {
                        break;
                    }

                    if score <= low && score > alpha && left > 0 {
                        left *= 2;
                        right = 0;
                    } else if score >= high && score < beta && right > 0 {
                        left = 0;
                        right *= 2;
                    } else {
                        break;
                    }
                }

                if self.stop.load(Ordering::Relaxed) != Stop::Running as u8 {
                    break;
                }

                if Self::is_depth_solving(depth, state.n_empties)
                    && ((alpha < score && score < beta)
                        || (score == alpha && score == SCORE_MIN)
                        || (score == beta && score == SCORE_MAX))
                    && !self.is_pv_ok(self.result.lock().unwrap().move_ as i32, depth)
                {
                    break;
                }

                if Self::is_depth_solving(depth, state.n_empties) && (score & 1) != 0 {
                    break;
                }

                if score == old_score {
                    break;
                }
            }
        }

        if self.stop.load(Ordering::Relaxed) != Stop::Running as u8 {
            // TODO #15: Refactor to avoid cloning
            // Make local copies to avoid borrowing issues
            let position = state.position;
            let bestmove = *state.movelist.first().unwrap();

            self.record_best_move(&position, &bestmove, alpha, beta, depth);
        }

        // TODO #14: update search time

        self.result.lock().unwrap().n_nodes = self.count_nodes();

        score
    }

    /// Like is_depth_solving() in Edax
    fn is_depth_solving(depth: i32, n_empties: i32) -> bool {
        (depth >= n_empties)
            || (depth > 9 && depth <= 12 && depth + 8 >= n_empties)
            || (depth > 12 && depth <= 18 && depth + 10 >= n_empties)
            || (depth > 18 && depth <= 24 && depth + 12 >= n_empties)
            || (depth > 24 && depth + 14 >= n_empties)
    }

    /// Like is_pv_ok() in Edax
    fn is_pv_ok(self: &Arc<Self>, bestmove: i32, mut depth: i32) -> bool {
        let mut position = self.state.lock().unwrap().position;
        let selectivity = self.config.read().unwrap().selectivity;

        let mut x = bestmove;

        while depth > 0 && x != NO_MOVE as i32 {
            if x != PASS as i32 {
                depth -= 1;
            }

            let move_ = Move::new(&position, x);
            move_.apply(&mut position);

            let hash_data = if let Some(hash_data) = self.pv_table.get(&position) {
                x = hash_data.move_[0] as i32;
                hash_data
            } else if let Some(hash_data) = self.hash_table.get(&position) {
                x = hash_data.move_[0] as i32;
                hash_data
            } else {
                break;
            };

            if (hash_data.depth as i32) < depth
                || (hash_data.selectivity as i32) < selectivity
                || hash_data.lower != hash_data.upper
            {
                return false;
            }

            if x == NO_MOVE as i32 && position.game_state() != GameState::Finished {
                return false;
            }
        }

        true
    }

    /// Like PVS_root() in Edax
    fn pvs_root(self: &Arc<Self>, _alpha: i32, _beta: i32, _depth: i32) -> i32 {
        todo!() // TODO
    }
}

#[cfg(test)]
mod tests {
    use crate::bot::edax::r#const::BLACK;

    use super::*;

    #[test]
    fn test_search_initialization() {
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
        let state = search.state.lock().unwrap();

        // Check position and player
        assert_eq!(state.position, *expected_position);
        assert_eq!(search.player, expected_player);

        // Check n_empties matches actual empty squares count
        assert_eq!(state.n_empties, expected_position.count_empty() as i32);

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
