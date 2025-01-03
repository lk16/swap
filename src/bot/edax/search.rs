use std::sync::atomic::{AtomicI64, AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::bot::edax::eval::Eval;
use crate::bot::edax::node::Node;
use crate::bot::edax::r#const::{
    DEPTH_TO_SHALLOW_SEARCH, ITERATIVE_MIN_EMPTIES, NEIGHBOUR, NO_SELECTIVITY,
    NWS_STABILITY_THRESHOLD, PROBCUT_D, QUADRANT_ID, RCD, SCORE_INF, SCORE_MAX, SCORE_MIN,
    SELECTIVITY_TABLE, SQUARE_VALUE, WEIGHT_CORNER_STABILITY, WEIGHT_EDGE_STABILITY, WEIGHT_EVAL,
    WEIGHT_FIRST_HASH_MOVE, WEIGHT_HASH, WEIGHT_HIGH_PARITY, WEIGHT_LOW_PARITY, WEIGHT_MID_PARITY,
    WEIGHT_MOBILITY, WEIGHT_POTENTIAL_MOBILITY, WEIGHT_SECOND_HASH_MOVE, WEIGHT_WIPEOUT,
};
use crate::collections::hashtable::{HashData, StoreArgs};
use crate::{
    collections::{
        hashtable::HashTable,
        move_list::{Move, MoveList},
    },
    othello::{count_last_flip::count_last_flip, position::Position, squares::*},
};

use super::r#const::{
    NodeType, DEPTH_MIDGAME_TO_ENDGAME, ETC_MIN_DEPTH, INC_SORT_DEPTH, PV_HASH_HEIGHT,
    SORT_ALPHA_DELTA,
};
use super::r#const::{Stop, LEVEL};
use super::search_state::SearchState;

/// Results of a search.
///
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

/// Options for a search, does not change during search.
///
/// Like unnamed struct field `options` of Search in Edax,
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

/// Time spent searching.
///
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
    /// Create a new search time.
    fn new() -> Self {
        let now = -Search::clock();

        Self {
            // Use negative so we can add current time to it later to get the elapsed time
            spent: AtomicI64::new(-now),
        }
    }
}

/// Score bounds for a move.
///
/// Like Bound in Edax
#[derive(Default, Copy, Clone)]
pub struct Bound {
    /// Lower bound
    pub lower: i32,

    /// Upper bound
    pub upper: i32,
}

/// Principal variation line.
///
/// Like Line in Edax
#[derive(Clone)]
pub struct Line {
    /// Moves in the line
    moves: Vec<u8>,
    // Edax has a `color` field, but it's unused in this implementation.
}

impl Line {
    /// Create a new line.
    fn new() -> Self {
        Self { moves: Vec::new() }
    }

    /// Push a move to the line.
    fn push(&mut self, x: u8) {
        self.moves.push(x);
    }
}

impl Default for Line {
    fn default() -> Self {
        Self::new()
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
        }
    }
}

/// Search fields that are shared between threads.
///
/// Like unnamed struct field `shared` of Search in Edax
pub struct Shared {
    /// Stop condition
    pub stop: AtomicU8,

    /// Number of nodes searched by this search instance
    pub n_nodes: AtomicU64,

    /// Number of nodes searched by parallel searches spawned by this search instance
    pub child_nodes: AtomicU64,

    /// Time elapsed since search started
    pub time: SearchTime,
    // TODO #8 add concurrent search: Add fields like these:
    // tasks: Arc<TaskStack>,
    // parent: Option<Arc<SharedSearchState>>,
    // children: Vec<Arc<SharedSearchState>>,
    // master: Option<Arc<SharedSearchState>>,
}

/// A game tree search implementation based on Edax's search algorithm.
///
/// This implements negamax search with:
/// - Principal Variation Search (PVS)
/// - Null Window Search (NWS)
/// - Transposition tables
/// - Move ordering
/// - Selective search
/// - Stability cutoffs
/// - Endgame solving
///
/// Like Search in Edax
pub struct Search {
    /// Color of player to move
    pub player: i32,

    /// Frequently changing search state
    pub state: SearchState,

    /// Search configuration
    pub config: SearchConfig,

    /// Result of search, changes during search
    pub result: Arc<Mutex<SearchResult>>,

    /// State shared between threads
    pub shared: Arc<Shared>,

    /// Main hash table
    pub hash_table: HashTable,

    /// Principal variation table
    pub pv_table: HashTable,

    /// Hash table for shallow search
    pub shallow_table: HashTable,

    /// Node type table
    pub node_type: [NodeType; 80],
}

impl Search {
    /// Like search_init() in Edax, but also does the following:
    /// - sets `player` and `position` like search_set_board() in Edax
    /// - sets `movelist` like search_get_movelist() in Edax
    /// - calls `setup()` to initialize other fields
    pub fn new(position: &Position, player: i32, level: i32) -> Self {
        let state = SearchState::new(position);
        let n_empties = position.count_empty() as i32;

        Self {
            player,
            state,
            config: SearchConfig::new(level, n_empties),
            result: Arc::new(Mutex::new(SearchResult::default())),
            shared: Arc::new(Shared {
                n_nodes: AtomicU64::new(0),
                child_nodes: AtomicU64::new(0),
                time: SearchTime::default(),
                stop: AtomicU8::new(Stop::StopEnd as u8),
            }),
            hash_table: HashTable::new(1 << 21),
            pv_table: HashTable::new(1 << 17),
            shallow_table: HashTable::new(1 << 21),
            node_type: [NodeType::default(); 80],
        }
    }

    /// Check if the search is running.
    fn is_running(&self) -> bool {
        self.shared.stop.load(Ordering::Relaxed) == Stop::Running as u8
    }

    /// Get current time in milliseconds since the Unix epoch.
    ///
    /// Like search_clock() in Edax
    fn clock() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
    }

    /// Count nodes searched.
    ///
    /// Like search_count_nodes() in Edax
    fn count_nodes(&self) -> u64 {
        self.shared.n_nodes.load(Ordering::Relaxed)
            + self.shared.child_nodes.load(Ordering::Relaxed)
    }

    /// Like statistics_sum_nodes() in Edax
    fn sum_nodes(&self) {
        // TODO #8 Add stats when we do parallel searches
    }

    /// Start a search.
    ///
    /// Like search_run() in Edax
    pub fn run(&mut self) -> SearchResult {
        self.shared
            .stop
            .store(Stop::Running as u8, Ordering::Relaxed);
        self.shared.n_nodes.store(0, Ordering::Relaxed);
        self.shared.child_nodes.store(0, Ordering::Relaxed);

        self.shared.time.spent.store(0, Ordering::Relaxed);

        if !self.config.options.keep_date {
            self.hash_table.soft_clear();
            self.pv_table.soft_clear();
            self.shallow_table.soft_clear();
        }

        {
            let movelist = self.state.move_list();

            let mut result = self.result.lock().unwrap();
            result.score = self.state.bound(self.state.eval_0());
            result.n_moves_left = movelist.len();
            result.n_moves = movelist.len() as i32;
            result.book_move = false;

            if self.state.move_list().is_empty() {
                result.bound[PASS] = Bound {
                    lower: SCORE_MIN,
                    upper: SCORE_MAX,
                };
            } else {
                for move_ in movelist.iter() {
                    result.bound[move_.x as usize] = Bound {
                        lower: SCORE_MIN,
                        upper: SCORE_MAX,
                    };
                }
            }
        }

        self.iterative_deepening(SCORE_MIN, SCORE_MAX);

        {
            let mut result = self.result.lock().unwrap();

            result.n_nodes = self.count_nodes();

            if self.is_running() {
                self.shared
                    .stop
                    .store(Stop::StopEnd as u8, Ordering::Relaxed);
            }

            self.shared
                .time
                .spent
                .fetch_add(Self::clock(), Ordering::Relaxed);
            result.time = self.shared.time.spent.load(Ordering::Relaxed);

            self.sum_nodes();

            result.clone()
        }
    }

    /// Returns Some((depth, selectivity)) if found in hash tables, None otherwise
    ///
    /// Like get_last_level() in Edax
    fn get_last_level(&self) -> Option<(i32, i32)> {
        let mut position = *self.state.position();

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

    /// Get time spent searching.
    ///
    /// Like search_time() in Edax
    fn get_time_spent(&self) -> i64 {
        if self.shared.stop.load(Ordering::Relaxed) != Stop::StopEnd as u8 {
            Self::clock() + self.shared.time.spent.load(Ordering::Relaxed)
        } else {
            self.shared.time.spent.load(Ordering::Relaxed)
        }
    }

    /// Record the best move to Result, hash tables and update bounds.
    ///
    /// Like record_best_move() in Edax
    pub fn record_best_move(
        &self,
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
            result.score = bestmove.score.get();

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
                    bound.upper = self.state.stability_bound().upper;
                }
                if result.score > alpha {
                    bound.lower = result.score;
                } else {
                    bound.upper = self.state.stability_bound().lower;
                }
            }

            result.bound[bestmove.x as usize] = bound;
        }

        let mut expected_depth = depth;
        result.depth = depth;

        let expected_selectivity = self.config.selectivity;
        result.selectivity = self.config.selectivity;

        let mut expected_bound = result.bound[bestmove.x as usize];

        result.pv = Line::new();
        let mut x = bestmove.x as usize;

        // NOTE: we don't guess the PV, like in Edax.
        let guess_pv = false;

        let mut fail_low = bestmove.score.get() <= alpha;
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

    /// Evaluate move list using hash data.
    /// This is used in preparation for sorting.
    ///
    /// Like movelist_evaluate() in Edax
    fn evaluate_movelist(
        &mut self,
        move_list: &MoveList,
        hash_data: &HashData,
        alpha: i32,
        depth: i32,
    ) {
        let n_empties = self.state.n_empties();

        let mut min_depth = 9;
        if n_empties <= 27 {
            min_depth += (30 - n_empties) / 3;
        }

        let sort_depth = if depth >= min_depth {
            let mut sort_depth = (depth - 15) / 3;

            if (hash_data.upper as i32) < alpha {
                sort_depth -= 2;
            }

            if n_empties >= 27 {
                sort_depth += 1;
            }

            sort_depth.clamp(0, 6)
        } else {
            -1
        };

        let sort_alpha = SCORE_MIN.max(alpha - SORT_ALPHA_DELTA);
        for move_ in move_list.iter() {
            let score = self.evaluate_move(move_.clone(), hash_data, sort_alpha, sort_depth);
            move_.score.set(score);
        }
    }

    /// Evaluate a move to sort it.
    ///
    /// Like move_evaluate() in Edax, except we return the score instead of setting it.
    fn evaluate_move(
        &mut self,
        move_: Move, // TODO #15 Optimize: use a reference, this leads to a borrow error
        hash_data: &HashData,
        sort_alpha: i32,
        sort_depth: i32,
    ) -> i32 {
        let mut score;

        if move_.is_wipeout(self.state.position()) {
            score = WEIGHT_WIPEOUT;
        } else if move_.x == hash_data.move_[0] as i32 {
            score = WEIGHT_FIRST_HASH_MOVE;
        } else if move_.x == hash_data.move_[1] as i32 {
            score = WEIGHT_SECOND_HASH_MOVE;
        } else {
            score = SQUARE_VALUE[move_.x as usize];
            if self.state.n_empties() < 12
                && (self.state.parity() & QUADRANT_ID[move_.x as usize]) != 0
            {
                score += WEIGHT_LOW_PARITY;
            } else if self.state.n_empties() < 21
                && (self.state.parity() & QUADRANT_ID[move_.x as usize]) != 0
            {
                score += WEIGHT_MID_PARITY;
            } else if self.state.n_empties() < 30
                && (self.state.parity() & QUADRANT_ID[move_.x as usize]) != 0
            {
                score += WEIGHT_HIGH_PARITY;
            }

            if sort_depth < 0 {
                // TODO #15 Optimize: use flipped discs from `move_` for doing and undoing move
                // TODO #15 Edax uses `state.position` here directly, we use a local copy for maximum correctness guarantee

                let mut position = *self.state.position();
                position.do_move(move_.x as usize);

                score += (36 - position.potential_mobility()) * WEIGHT_POTENTIAL_MOBILITY;
                score += position.opponent_corner_stability() * WEIGHT_CORNER_STABILITY;
                score += (36 - position.weighted_mobility()) * WEIGHT_MOBILITY;

                // TODO #15 If we stop using the local copy, undo move here.
            } else {
                let selectivity = self.config.selectivity;
                self.config.selectivity = NO_SELECTIVITY;

                self.state.update_midgame(&move_);
                score +=
                    (36 - self.state.position().potential_mobility()) * WEIGHT_POTENTIAL_MOBILITY; // potential mobility
                score += self.state.position().opponent_edge_stability() * WEIGHT_EDGE_STABILITY; // edge stability
                score += (36 - self.state.position().weighted_mobility()) * WEIGHT_MOBILITY; // real mobility

                score += match sort_depth {
                    0 => ((SCORE_MAX - self.state.eval_0()) >> 2) * WEIGHT_EVAL,
                    1 => {
                        ((SCORE_MAX - self.state.eval_1(SCORE_MIN, -sort_alpha)) >> 1) * WEIGHT_EVAL
                    }
                    2 => {
                        ((SCORE_MAX - self.state.eval_2(SCORE_MIN, -sort_alpha)) >> 1) * WEIGHT_EVAL
                    }
                    _ => {
                        let mut deeper_search_score = (SCORE_MAX
                            - self.pvs_shallow(SCORE_MIN, -sort_alpha, sort_depth))
                            * WEIGHT_EVAL;

                        if self.hash_table.get(self.state.position()).is_some() {
                            deeper_search_score += WEIGHT_HASH;
                        }

                        deeper_search_score
                    }
                };
                self.state.restore_midgame(&move_);

                self.config.selectivity = selectivity;
            }
        }

        score
    }

    /// Principal Variation Search at shallow depth.
    ///
    /// Like PVS_shallow() in Edax
    fn pvs_shallow(&mut self, alpha: i32, mut beta: i32, depth: i32) -> i32 {
        debug_assert!(depth >= 2);

        let mut cost = -(self.shared.n_nodes.load(Ordering::Relaxed) as i64);

        if depth == 2 {
            return self.state.eval_2(alpha, beta);
        }

        if let Some(score) = self.state.stability_cutoff_pvs(alpha, &mut beta) {
            return score;
        }

        let mut movelist = MoveList::new(self.state.position());

        let mut bestmove;
        let mut bestscore;

        if movelist.is_empty() {
            if self.state.position().opponent_has_moves() {
                self.state.update_pass_midgame();
                bestscore = -self.pvs_shallow(-beta, -alpha, depth);
                bestmove = PASS;
                self.state.restore_pass_midgame();
            } else {
                bestscore = self.state.solve();
                bestmove = NO_MOVE;
            }
        } else {
            let hash_data = self.shallow_table.get_or_default(self.state.position());

            self.evaluate_movelist(&movelist, &hash_data, alpha, depth);
            movelist.sort_by_score();

            bestscore = -SCORE_INF;
            bestmove = NO_MOVE;
            let mut lower = alpha;

            for move_ in movelist.iter() {
                self.state.update_midgame(move_);

                let score = if bestscore == -SCORE_INF {
                    -self.pvs_shallow(-beta, -lower, depth - 1)
                } else {
                    let mut score = -self.nws_shallow_with_shallow_table(-lower - 1, depth - 1);
                    if alpha < score && score < beta {
                        score = -self.pvs_shallow(-beta, -lower, depth - 1);
                    }

                    score
                };

                self.state.restore_midgame(move_);

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

        cost += self.shared.n_nodes.load(Ordering::Relaxed) as i64;

        self.shallow_table.store(&StoreArgs {
            position: self.state.position(),
            depth,
            selectivity: self.config.selectivity,
            cost: Self::ilog2(cost),
            alpha,
            beta,
            score: bestscore,
            move_: bestmove as i32,
        });

        bestscore
    }

    /// Null Window Search at shallow depth.
    ///
    /// Like NWS_shallow() in Edax
    fn nws_shallow<const USE_SHALLOW_TABLE: bool>(&mut self, alpha: i32, depth: i32) -> i32 {
        debug_assert!(depth >= 2);

        let selectivity = self.config.selectivity;

        let beta = alpha + 1;
        let mut cost = -(self.shared.n_nodes.load(Ordering::Relaxed) as i64);

        if depth == 2 {
            return self.state.eval_2(alpha, beta);
        }

        if let Some(score) = self.state.stability_cutoff_nws(alpha) {
            return score;
        }

        let hash_data = {
            let hash_data = if USE_SHALLOW_TABLE {
                self.shallow_table.get(self.state.position())
            } else {
                self.hash_table.get(self.state.position())
            };

            if let Some(ref hash_data) = hash_data {
                if let Some(score) =
                    Self::transposition_cutoff_nws(hash_data, depth, selectivity, alpha)
                {
                    return score;
                }
            }

            hash_data.unwrap_or_default()
        };

        let mut movelist = MoveList::new(self.state.position());

        let mut bestscore;
        let mut bestmove;

        if movelist.is_empty() {
            if self.state.position().opponent_has_moves() {
                self.state.update_pass_midgame();
                bestscore = -self.nws_shallow::<USE_SHALLOW_TABLE>(-beta, depth);
                bestmove = PASS;
                self.state.restore_pass_midgame();
            } else {
                bestscore = self.state.solve();
                bestmove = NO_MOVE;
            }
        } else {
            self.evaluate_movelist(&movelist, &hash_data, alpha, depth);
            movelist.sort_by_score();

            bestscore = -SCORE_INF;
            bestmove = NO_MOVE;

            for move_ in movelist.iter() {
                self.state.update_midgame(move_);
                let score = -self.nws_shallow::<USE_SHALLOW_TABLE>(-beta, depth - 1);
                self.state.restore_midgame(move_);

                if score > bestscore {
                    bestscore = score;
                    bestmove = move_.x as usize;

                    if bestscore >= beta {
                        break;
                    }
                }
            }
        }

        cost += self.shared.n_nodes.load(Ordering::Relaxed) as i64;

        let store_args = StoreArgs {
            position: self.state.position(),
            depth,
            selectivity,
            cost: Self::ilog2(cost),
            alpha,
            beta,
            score: bestscore,
            move_: bestmove as i32,
        };

        if USE_SHALLOW_TABLE {
            self.shallow_table.store(&store_args);
        } else {
            self.hash_table.store(&store_args);
        }

        bestscore
    }

    /// Like NWS_shallow() in Edax but using self.shallow_table
    fn nws_shallow_with_shallow_table(&mut self, alpha: i32, depth: i32) -> i32 {
        self.nws_shallow::<true>(alpha, depth)
    }

    /// Like NWS_shallow() in Edax but using self.hash_table
    fn nws_shallow_with_hash_table(&mut self, alpha: i32, depth: i32) -> i32 {
        self.nws_shallow::<false>(alpha, depth)
    }

    /// Transposition cutoff for Null Window Search.
    ///
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

    /// Check if the search should continue.
    ///
    /// Like search_continue() in Edax
    fn continue_search(&self) -> bool {
        // TODO #14 when we support time management, we need to check if we have time left
        self.is_running()
    }

    /// Iterative deepening search.
    ///
    /// Like iterative_deepening() in Edax
    fn iterative_deepening(&mut self, alpha: i32, beta: i32) {
        let mut result = self.result.lock().unwrap();

        result.move_ = NO_MOVE;
        result.score = -SCORE_INF;
        result.depth = -1;
        result.selectivity = 0;
        result.time = 0;
        result.n_nodes = 0;
        result.pv = Line::new();

        // Game is over
        if self.state.move_list().is_empty() && !self.state.position().opponent_has_moves() {
            result.move_ = NO_MOVE;
            result.score = self.state.solve();
            result.depth = self.state.n_empties();
            result.selectivity = NO_SELECTIVITY;
            result.time = self.shared.time.spent.load(Ordering::Relaxed);
            result.n_nodes = self.count_nodes();
            result.bound[NO_MOVE] = Bound {
                lower: result.score,
                upper: result.score,
            };
            result.pv = Line::new();
            return;
        }

        let options_depth = self.config.options.depth;

        let mut score = self.state.bound(self.state.eval_0());
        let mut end = options_depth;
        if end >= self.state.n_empties() {
            end = self.state.n_empties() - ITERATIVE_MIN_EMPTIES + 2;
            if end <= 0 {
                end = 2 - (self.state.n_empties() & 1);
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

        if let Some(hash_data) = self.pv_table.get(self.state.position()) {
            old_depth = hash_data.depth as i32;
            old_selectivity = hash_data.selectivity as i32;

            if hash_data.lower == hash_data.upper {
                if let Some((depth, selectivity)) = self.get_last_level() {
                    start = depth;
                    self.config.selectivity = selectivity;
                }
                score = hash_data.lower as i32;
            } else {
                self.adjust_time(true);
            }
        } else {
            self.adjust_time(false);
        }

        {
            self.config.selectivity = self.config.selectivity.min(self.config.options.selectivity);
            self.config.options.depth = self.config.options.depth.min(options_depth);
        }

        start = start.min(self.state.n_empties());

        if start < self.state.n_empties() {
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

        if self.state.move_list().is_empty() {
            let bestmove = Move::new_pass();
            bestmove.score.set(score);

            // Create local copy to avoid borrowing issues
            let position = *self.state.position();
            self.record_best_move(&position, &bestmove, alpha, beta, old_depth);
        } else {
            if end == 0 {
                // shuffle the movelist
                self.state.randomize_move_list_score();
            } else {
                // Get hash data from pv_table
                let hash_data = self
                    .pv_table
                    .get(self.state.position())
                    .unwrap_or_else(|| self.hash_table.get_or_default(self.state.position()));

                // TODO #15 Optimize: avoid cloning the movelist
                let move_list = self.state.move_list().clone();
                // Set `score` for all moves in movelist
                self.evaluate_movelist(&move_list, &hash_data, alpha, start);
                self.state.set_move_list(move_list);
            }
            self.state.sort_move_list_by_score();

            self.state.set_best_move_score(score);
            let bestmove = self.state.get_best_move();
            self.record_best_move(self.state.position(), bestmove, alpha, beta, old_depth);
        }

        self.config.selectivity = old_selectivity;

        // Special case: level 0
        if end == 0 {
            return;
        }

        // midgame: iterative depth
        let mut depth = start;
        while depth < end {
            self.state.set_pv_extension(depth);
            score = self.aspiration_search(alpha, beta, depth, score);

            if !self.continue_search() {
                return;
            }

            if score.abs() >= SCORE_MAX - 1
                && depth > end - ITERATIVE_MIN_EMPTIES
                && self.config.options.depth >= self.state.n_empties()
            {
                break;
            }

            depth += 2;
        }
        self.config.depth = end;

        // Switch to endgame
        if self.config.options.depth >= self.state.n_empties() {
            self.config.depth = self.state.n_empties();
        }

        // iterative selectivity

        // TODO #14 pretend we have time, since we don't do time management yet
        let has_time = true;

        loop {
            if self.config.selectivity > self.config.options.selectivity {
                break;
            }

            // Check if we should jump to exact endgame for faster solving
            if self.config.depth == self.state.n_empties()
                && ((self.config.depth < 21 && self.config.selectivity >= 1)
                    || (self.config.depth < 24 && self.config.selectivity >= 2)
                    || (self.config.depth < 27 && self.config.selectivity >= 3)
                    || (self.config.depth < 30 && self.config.selectivity >= 4)
                    || (has_time && self.config.depth < 30 && self.config.selectivity >= 2)
                    || score.abs() >= SCORE_MAX)
            {
                self.config.selectivity = self.config.options.selectivity;
            }

            let current_depth = self.config.depth;
            let current_selectivity = self.config.selectivity;

            if current_selectivity == self.config.options.selectivity {
                self.adjust_time(true);
            }

            score = self.aspiration_search(alpha, beta, current_depth, score);

            if !self.continue_search() {
                return;
            }

            self.config.selectivity = current_selectivity + 1;
        }

        // Ensure selectivity doesn't exceed options.selectivity
        if self.config.selectivity > self.config.options.selectivity {
            self.config.selectivity = self.config.options.selectivity;
        }
    }

    /// Aspiration search.
    ///
    /// Like aspiration_search() in Edax
    fn aspiration_search(
        &mut self,
        mut alpha: i32,
        mut beta: i32,
        depth: i32,
        mut score: i32,
    ) -> i32 {
        if Self::is_depth_solving(depth, self.state.n_empties()) {
            if alpha & 1 != 0 {
                alpha -= 1;
            }
            if beta & 1 != 0 {
                beta += 1;
            }
        }

        if depth <= self.config.options.multipv_depth {
            alpha = SCORE_MIN;
            beta = SCORE_MAX;
        }

        let mut high = SCORE_MAX.min(self.state.stability_bound().upper + 2);
        let mut low = SCORE_MIN.max(self.state.stability_bound().lower - 2);

        alpha = alpha.max(low);
        beta = beta.min(high);
        score = score.clamp(low, high);
        score = score.clamp(alpha, beta);

        let mut result = self.result.lock().unwrap();

        for move_ in self.state.move_list().iter() {
            result.bound[move_.x as usize] = Bound {
                lower: low,
                upper: high,
            };
        }

        drop(result);

        let width = {
            let mut width = 10 - depth;
            width = width.min(1);

            if width & 1 != 0 && depth == self.state.n_empties() {
                width += 1;
            }

            width
        };

        let mut left: i32;
        let mut right: i32;

        for i in 0..10 {
            let old_score = score;

            if depth < self.config.options.multipv_depth || beta - alpha <= 2 * width {
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

                    if !self.is_running() {
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

                if !self.is_running() {
                    break;
                }

                if Self::is_depth_solving(depth, self.state.n_empties())
                    && ((alpha < score && score < beta)
                        || (score == alpha && score == SCORE_MIN)
                        || (score == beta && score == SCORE_MAX))
                    && !self.is_pv_ok(self.result.lock().unwrap().move_ as i32, depth)
                {
                    break;
                }

                if Self::is_depth_solving(depth, self.state.n_empties()) && (score & 1) != 0 {
                    break;
                }

                if score == old_score {
                    break;
                }
            }
        }

        if self.is_running() {
            // TODO #15: Refactor to avoid cloning
            // Make local copies to avoid borrowing issues
            let bestmove = self.state.move_list().first().unwrap();
            self.record_best_move(self.state.position(), bestmove, alpha, beta, depth);
        }

        // TODO #14: update search time

        self.result.lock().unwrap().n_nodes = self.count_nodes();

        score
    }

    /// Check if the search is at a depth that should be solved.
    ///
    /// Like is_depth_solving() in Edax
    fn is_depth_solving(depth: i32, n_empties: i32) -> bool {
        (depth >= n_empties)
            || (depth > 9 && depth <= 12 && depth + 8 >= n_empties)
            || (depth > 12 && depth <= 18 && depth + 10 >= n_empties)
            || (depth > 18 && depth <= 24 && depth + 12 >= n_empties)
            || (depth > 24 && depth + 14 >= n_empties)
    }

    /// Check if the principal variation is ok.
    ///
    /// Like is_pv_ok() in Edax
    fn is_pv_ok(&self, bestmove: i32, mut depth: i32) -> bool {
        let mut position = *self.state.position();
        let selectivity = self.config.selectivity;

        let mut x = bestmove;

        while depth > 0 && x != NO_MOVE as i32 {
            if x != PASS as i32 {
                depth -= 1;
            }

            let move_ = Move::new(&position, x);
            move_.update(&mut position);

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

            if x == NO_MOVE as i32 && !position.is_game_end() {
                return false;
            }
        }

        true
    }

    /// Principal Variation Search at root.
    ///
    /// Like PVS_root() in Edax
    fn pvs_root(&mut self, alpha: i32, beta: i32, depth: i32) -> i32 {
        let mut cost = -(self.count_nodes() as i64);

        self.state.set_probcut_level(0);

        {
            let mut result = self.result.lock().unwrap();
            result.n_moves_left = result.n_moves as usize;
        }

        let move_list = MoveList::new(self.state.position());

        let node = Node::new(
            self.shared.clone(),
            alpha,
            beta,
            depth,
            None,
            self.state.height(),
        );

        node.set_pv_node(true);
        self.node_type[0] = NodeType::PvNode;

        if move_list.is_empty() {
            let move_ = if self.state.position().opponent_has_moves() {
                self.state.update_pass_midgame();
                let searched =
                    -self.route_pvs(-node.beta(), -node.alpha(), depth, Some(node.clone()));
                self.state.restore_pass_midgame();
                node.set_best_score(searched);
                Move::new_pass_with_score(searched)
            } else {
                let solved = self.solve();
                node.set_best_score(solved);
                Move::new_pass_with_score(solved)
            };

            node.set_move_list(MoveList::new_one_move(move_));
        } else {
            node.set_move_list(move_list);

            // List is not empty, so we can safely unwrap
            let (index, move_) = node.next_move().unwrap();
            self.state.update_midgame(&move_);
            self.node_type[self.state.height() as usize] = NodeType::PvNode;
            let score = -self.route_pvs(-beta, -alpha, depth - 1, Some(node.clone()));
            let cost = self.get_pv_cost() as u32;
            self.state.restore_midgame(&move_);

            node.set_move_score_and_cost(index, score, cost);
            node.update(index, self);

            while let Some((index, move_)) = node.next_move() {
                let alpha = if depth > self.config.options.multipv_depth {
                    node.alpha()
                } else {
                    SCORE_MIN
                };

                if depth > self.config.options.multipv_depth && node.split(&move_) {
                    // Do nothing
                } else {
                    self.state.update_midgame(&move_);
                    let mut score =
                        -self.route_pvs(-alpha - 1, -alpha, depth - 1, Some(node.clone()));
                    if alpha < move_.score.get() && move_.score.get() < beta {
                        self.node_type[self.state.height() as usize] = NodeType::PvNode;
                        score = -self.route_pvs(-beta, -alpha, depth - 1, Some(node.clone()));
                    }
                    let cost = self.get_pv_cost() as u32;
                    self.state.restore_midgame(&move_);

                    node.set_move_score_and_cost(index, score, cost);
                    node.update(index, self);
                }
            }

            node.wait_slaves();
        }

        if self.is_running() {
            let hash_data = self.pv_table.get_or_default(self.state.position());

            if depth < self.config.options.multipv_depth {
                self.state.sort_move_list_by_score();
            } else {
                self.state.sort_move_list_by_cost(&hash_data);
            }

            self.state.set_first_move(node.best_move());

            self.record_best_move(
                self.state.position(),
                self.state.move_list().first().unwrap(),
                alpha,
                beta,
                depth,
            );

            if self.state.move_list().len() == self.state.position().count_moves() {
                cost += self.count_nodes() as i64;
                self.hash_table.store(&StoreArgs {
                    position: self.state.position(),
                    depth,
                    selectivity: self.config.selectivity,
                    cost: Self::ilog2(cost),
                    alpha,
                    beta,
                    score: node.best_score(),
                    move_: node.best_move(),
                });
            }

            if false {
                // NOTE: Edax does force-stores a value here when guess_pv is enabled, but we don't support this.
                unreachable!()
            } else {
                self.pv_table.store(&StoreArgs {
                    position: self.state.position(),
                    depth,
                    selectivity: self.config.selectivity,
                    cost: Self::ilog2(cost),
                    alpha,
                    beta,
                    score: node.best_score(),
                    move_: node.best_move(),
                });
            }
        }

        node.best_score()
    }

    /// Get final score for the position.
    ///
    /// Like search_solve() in Edax
    fn solve(&self) -> i32 {
        self.state
            .position()
            .final_score_with_empty(self.state.n_empties())
    }

    /// Route search requests to the appropriate search function based on depth.
    /// This is a principal variation search that returns a score from the player's perspective.
    ///
    /// The function routes search requests as follows:
    /// - For depth == 0: returns eval_0()
    /// - For depth == empty squares: uses pvs_midgame()
    /// - For depth == 1: uses eval_1()
    /// - For depth == 2: uses eval_2()
    /// - For depth > 2: uses pvs_shallow()
    ///
    /// Like search_route_PVS() in Edax
    fn route_pvs(&mut self, alpha: i32, beta: i32, depth: i32, node: Option<Arc<Node>>) -> i32 {
        let score = if depth == self.state.n_empties() {
            if depth == 0 {
                self.state.position().final_score_with_empty(0)
            } else {
                self.pvs_midgame(alpha, beta, depth, node)
            }
        } else {
            match depth {
                0 => self.state.eval_0(),
                1 => self.state.eval_1(alpha, beta),
                2 => self.state.eval_2(alpha, beta),
                _ => self.pvs_midgame(alpha, beta, depth, node),
            }
        };

        -self.state.bound(-score)
    }

    /// Get cost of the principal variation.
    ///
    /// Like search_get_pv_cost() in Edax
    fn get_pv_cost(&self) -> i32 {
        let position = self.state.position();

        let hash_data = self
            .pv_table
            .get(position)
            .or_else(|| self.hash_table.get(position))
            .or_else(|| self.shallow_table.get(position));

        if let Some(hash_data) = hash_data {
            hash_data.writable_level() as i32
        } else {
            0
        }
    }

    /// Principal Variation Search at midgame depth.
    /// This is a principal variation search that returns a score from the player's perspective.
    ///
    /// The function handles positions with more than DEPTH_MIDGAME_TO_ENDGAME (15) empty squares.
    /// It uses principal variation search (PVS) which is an enhancement to alpha-beta pruning that
    /// assumes the first move is best and searches remaining moves with a null window.
    ///
    /// Like PVS_midgame() in Edax
    fn pvs_midgame(&mut self, alpha: i32, beta: i32, depth: i32, parent: Option<Arc<Node>>) -> i32 {
        if !self.is_running() {
            return alpha;
        }

        if self.state.n_empties() == 0 {
            return self.state.position().final_score_with_empty(0);
        }

        if depth < self.state.n_empties()
            && self.state.n_empties() < self.state.depth_pv_extension()
        {
            return self.pvs_midgame(alpha, beta, self.state.n_empties(), parent);
        }

        if depth == 2 && self.state.n_empties() > 2 {
            return self.state.eval_2(alpha, beta);
        }

        let mut cost = -(self.count_nodes() as i64);

        let mut move_list = MoveList::new(self.state.position());

        let node = Node::new(
            self.shared.clone(),
            alpha,
            beta,
            depth,
            parent,
            self.state.height(),
        );
        node.set_pv_node(true);

        if move_list.is_empty() {
            if self.state.position().opponent_has_moves() {
                self.state.update_pass_midgame();
                node.set_best_score(-self.route_pvs(-beta, -alpha, depth, Some(node.clone())));
                self.state.restore_pass_midgame();
                node.set_best_move(PASS as i32);
            } else {
                node.set_beta(SCORE_INF);
                node.set_alpha(-SCORE_INF);
                node.set_best_score(self.solve());
                node.set_best_move(NO_MOVE as i32);
            }
        } else {
            if move_list.len() > 1 {
                let hash_data = self
                    .pv_table
                    .get(self.state.position())
                    .unwrap_or_else(|| self.hash_table.get_or_default(self.state.position()));

                // Evaluate moves for sorting. For better move sorting, depth is artificially increased.

                self.evaluate_movelist(
                    &move_list,
                    &hash_data,
                    node.alpha(),
                    depth + INC_SORT_DEPTH[NodeType::PvNode as usize],
                );

                move_list.sort_by_score();
            }

            node.set_move_list(move_list);

            let (index, move_) = node.next_move().unwrap();
            self.state.update_midgame(&move_);
            self.node_type[self.state.height() as usize] = NodeType::PvNode;
            let score = -self.pvs_midgame(-beta, -alpha, depth - 1, Some(node.clone()));
            self.state.restore_midgame(&move_);

            node.set_move_score(index, score);
            node.update(index, self);

            while let Some((index, move_)) = node.next_move() {
                if !node.split(&move_) {
                    let alpha = node.alpha();
                    self.state.update_midgame(&move_);

                    // Edax doesn't set the node type here.
                    self.node_type[self.state.height() as usize] = NodeType::CutNode;

                    let mut score = -self.nws_midgame(-alpha - 1, depth - 1, Some(node.clone()));
                    if self.is_running() && alpha < score && score < beta {
                        self.node_type[self.state.height() as usize] = NodeType::PvNode;
                        score = -self.pvs_midgame(-beta, -alpha, depth - 1, Some(node.clone()));
                    }
                    self.state.restore_midgame(&move_);

                    node.set_move_score(index, score);
                    node.update(index, self);
                }
            }

            node.wait_slaves();
        }

        if self.is_running() {
            cost += self.count_nodes() as i64;

            let hash_selectivity =
                if self.state.n_empties() < depth && depth <= DEPTH_MIDGAME_TO_ENDGAME {
                    NO_SELECTIVITY
                } else {
                    self.config.selectivity
                };

            let store_args = StoreArgs {
                position: self.state.position(),
                depth,
                selectivity: hash_selectivity,
                cost: Self::ilog2(cost),
                alpha,
                beta,
                score: node.best_score(),
                move_: node.best_move(),
            };

            self.hash_table.store(&store_args);
            self.pv_table.store(&store_args);
        } else {
            node.set_best_score(alpha);
        }

        node.best_score()
    }

    /// Null Window Search at midgame depth.
    /// This is a null window search that returns a score from the player's perspective.
    ///
    /// The function handles positions in either of these cases:
    /// - More than DEPTH_MIDGAME_TO_ENDGAME (15) empty squares
    /// - Search depth is less than the number of empty squares
    ///
    /// Like NWS_midgame() in Edax
    fn nws_midgame(&mut self, alpha: i32, depth: i32, parent: Option<Arc<Node>>) -> i32 {
        let beta = alpha + 1;

        let mut cost = -(self.shared.n_nodes.load(Ordering::Relaxed) as i64)
            - (self.shared.child_nodes.load(Ordering::Relaxed) as i64);

        if !self.is_running() {
            return alpha;
        }

        if self.state.n_empties() == 0 {
            return self.state.eval_0();
        }

        if depth <= 3 && depth < self.state.n_empties() {
            return self.nws_shallow_with_hash_table(alpha, depth);
        }

        if self.state.n_empties() <= depth && depth < DEPTH_MIDGAME_TO_ENDGAME {
            return self.nws_endgame(alpha);
        }

        if let Some(score) = self.state.stability_cutoff_nws(alpha) {
            return score;
        }

        let hash_data = {
            let hash_data = self
                .hash_table
                .get(self.state.position())
                .or_else(|| self.shallow_table.get(self.state.position()));

            if let Some(hash_data) = hash_data {
                if let Some(score) = Self::transposition_cutoff_nws(
                    &hash_data,
                    depth,
                    self.config.selectivity,
                    alpha,
                ) {
                    return score;
                }
            }

            hash_data.unwrap_or_default()
        };

        let mut move_list = MoveList::new(self.state.position());

        let node;

        if move_list.is_empty() {
            node = Node::new(
                self.shared.clone(),
                alpha,
                beta,
                depth,
                parent,
                self.state.height(),
            );

            let score = if self.state.position().opponent_has_moves() {
                self.state.update_pass_midgame();
                let score = -self.nws_midgame(-node.beta(), depth, Some(node.clone()));
                self.state.restore_pass_midgame();

                score
            } else {
                self.solve()
            };

            node.set_best_score(score);
        } else {
            // Edax doesn't set the node type here.
            // However, all nodes within NWS are cut nodes.
            self.node_type[self.state.height() as usize] = NodeType::CutNode;

            if let Some(score) = self.probcut(alpha, depth, parent.clone()) {
                return score;
            }

            if move_list.len() > 1 {
                // Edax tries to get same item from hash_table again. We skip that here.

                let inc_sort_depth =
                    INC_SORT_DEPTH[self.node_type[self.state.height() as usize] as usize];
                self.evaluate_movelist(&move_list, &hash_data, alpha, depth + inc_sort_depth);

                move_list.sort_by_score();
            }

            if let Some(score) = self.etc_nws(&move_list, depth, self.config.selectivity, alpha) {
                return score;
            }

            node = Node::new(
                self.shared.clone(),
                alpha,
                beta,
                depth,
                parent,
                self.state.height(),
            );

            node.set_move_list(move_list);

            while let Some((index, move_)) = node.next_move() {
                if !node.split(&move_) {
                    self.state.update_midgame(&move_);
                    let score = -self.nws_midgame(-beta, depth - 1, Some(node.clone()));
                    self.state.restore_midgame(&move_);

                    node.set_move_score(index, score);
                    node.update(index, self);
                }
            }

            node.wait_slaves();
        }

        if self.is_running() {
            cost += self.shared.n_nodes.load(Ordering::Relaxed) as i64
                + self.shared.child_nodes.load(Ordering::Relaxed) as i64;

            let hash_selectivity =
                if self.state.n_empties() < depth && depth < DEPTH_MIDGAME_TO_ENDGAME {
                    NO_SELECTIVITY
                } else {
                    self.config.selectivity
                };

            let store_args = StoreArgs {
                position: self.state.position(),
                depth,
                selectivity: hash_selectivity,
                cost: Self::ilog2(cost),
                alpha,
                beta,
                score: node.best_score(),
                move_: node.best_move(),
            };

            if self.state.height() <= PV_HASH_HEIGHT {
                self.pv_table.store(&store_args);
            }

            self.hash_table.store(&store_args);
        }

        node.best_score()
    }

    // Probcut breaks some tests for nws_midgame(), so we disable it for now.
    // TODO #15 further optimization: re-enable probcut.
    #[allow(unreachable_code)]
    #[allow(unused_variables)]
    /// Probcut search.
    ///
    /// Like search_probcut() in Edax
    fn probcut(&mut self, alpha: i32, depth: i32, parent: Option<Arc<Node>>) -> Option<i32> {
        return None;

        debug_assert_ne!(
            self.node_type[self.state.height() as usize],
            NodeType::PvNode
        );

        // Edax also `checks depth >= options.probcut_d` where the latter is a double with value `0.25`.
        // We just inline that as `true` here, since depth is always >= 1.
        if self.config.selectivity < NO_SELECTIVITY && self.state.probcut_level() < 2 {
            let beta = alpha + 1;

            let t = SELECTIVITY_TABLE[self.config.selectivity as usize].t;
            let saved_selectivity = self.config.selectivity;
            let node_type = self.node_type[self.state.height() as usize];

            // compute reduced depth & associated error
            let mut probcut_depth = 2 * (PROBCUT_D * depth as f64).floor() as i32 + (depth & 1);
            if probcut_depth == 0 {
                probcut_depth = depth - 2;
            }

            debug_assert!(probcut_depth > 1);
            debug_assert!(probcut_depth <= depth - 2);
            debug_assert!((probcut_depth & 1) == (depth & 1));

            let probcut_error =
                (t * Eval::sigma(self.state.n_empties(), depth, probcut_depth) + RCD) as i32;

            // compute evaluation error (i.e. error at depth 0) averaged for both depths
            let eval_score = self.state.eval_0();
            let eval_error = (t
                * 0.5
                * (Eval::sigma(self.state.n_empties(), depth, 0)
                    + Eval::sigma(self.state.n_empties(), depth, probcut_depth))
                + RCD) as i32;

            // try a probable upper cut first
            let eval_beta = beta - eval_error;
            let probcut_beta = beta + probcut_error;
            let probcut_alpha = probcut_beta - 1;
            // check if trying a beta probcut is worth
            if eval_score >= eval_beta && probcut_beta < SCORE_MAX {
                self.update_probcut(NodeType::CutNode);
                let score = self.nws_midgame(probcut_alpha, probcut_depth, parent.clone());
                self.restore_probcut(node_type, saved_selectivity);
                if score >= probcut_beta {
                    return Some(beta);
                }
            }

            // try a probable lower cut if upper cut failed
            let eval_alpha = alpha + eval_error;
            let probcut_alpha = alpha - probcut_error;
            // check if trying an alpha probcut is worth
            if eval_score < eval_alpha && probcut_alpha > SCORE_MIN {
                self.update_probcut(NodeType::AllNode);
                let score = self.nws_midgame(probcut_alpha, probcut_depth, parent);
                self.restore_probcut(node_type, saved_selectivity);
                if score <= probcut_alpha {
                    return Some(alpha);
                }
            }
        }

        None
    }

    /// Update config for probcut search.
    ///
    /// Like search_update_probcut() in Edax
    fn update_probcut(&mut self, node_type: NodeType) {
        self.node_type[self.state.height() as usize] = node_type;
        self.state.set_probcut_level(self.state.probcut_level() + 1);
    }

    /// Restore config for probcut search.
    ///
    /// Like search_restore_probcut() in Edax
    fn restore_probcut(&mut self, node_type: NodeType, selectivity: i32) {
        // This argument is not used in Edax with default configuration.
        _ = selectivity;

        self.node_type[self.state.height() as usize] = node_type;
        self.state.set_probcut_level(self.state.probcut_level() - 1);
    }

    /// Enhanced Transposition Cutoff Null Window Search.
    ///
    /// Like search_ETC_NWS() in Edax
    fn etc_nws(
        &mut self,
        move_list: &MoveList,
        depth: i32,
        selectivity: i32,
        alpha: i32,
    ) -> Option<i32> {
        if depth > ETC_MIN_DEPTH {
            let etc_depth = depth - 1;
            let beta = alpha + 1;

            for move_ in move_list.iter() {
                let mut child = *self.state.position();
                move_.update(&mut child);

                if alpha <= -NWS_STABILITY_THRESHOLD[self.state.n_empties() as usize] {
                    let score = 2 * child.count_player_stable_discs() - SCORE_MAX;
                    if score > alpha {
                        self.hash_table.store(&StoreArgs {
                            position: self.state.position(),
                            depth,
                            selectivity,
                            cost: 0,
                            alpha,
                            beta,
                            score,
                            move_: move_.x,
                        });
                        return Some(score);
                    }
                }

                if let Some(hash_data) = self.hash_table.get(&child) {
                    if selectivity >= hash_data.selectivity as i32
                        && hash_data.depth as i32 >= etc_depth
                    {
                        let score = -hash_data.upper as i32;
                        if score > alpha {
                            self.hash_table.store(&StoreArgs {
                                position: self.state.position(),
                                depth,
                                selectivity,
                                cost: 0,
                                alpha,
                                beta,
                                score,
                                move_: move_.x,
                            });
                            return Some(score);
                        }
                    }
                }
            }
        }

        None
    }

    /// Null Window Search at endgame depth.
    /// This is a null window search that returns a score from the player's perspective.
    ///
    /// The function handles positions with at most DEPTH_MIDGAME_TO_ENDGAME (15) empty squares.
    ///
    /// Like NWS_endgame() in Edax
    fn nws_endgame(&mut self, alpha: i32) -> i32 {
        #[cfg(debug_assertions)]
        {
            let empty_count = self.state.position().empties().count_ones();
            debug_assert!(empty_count <= DEPTH_MIDGAME_TO_ENDGAME as u32);
        }

        let beta = alpha + 1;

        if !self.is_running() {
            return alpha;
        }

        if self.state.n_empties() <= DEPTH_TO_SHALLOW_SEARCH {
            return self.endgame_shallow(alpha);
        }

        if let Some(score) = self.state.stability_cutoff_nws(alpha) {
            return score;
        }

        let hash_data = {
            let hash_data = self.hash_table.get(self.state.position());

            if let Some(hash_data) = hash_data {
                if let Some(score) = Self::transposition_cutoff_nws(
                    &hash_data,
                    self.state.n_empties(),
                    NO_SELECTIVITY,
                    alpha,
                ) {
                    return score;
                }
            }

            hash_data.unwrap_or_default()
        };

        let move_list = MoveList::new(self.state.position());

        let mut cost = -(self.shared.n_nodes.load(Ordering::Relaxed) as i64);

        let best_move = if move_list.is_empty() {
            if self.state.position().opponent_has_moves() {
                self.state.pass_endgame();
                let score = -self.nws_endgame(-beta);
                self.state.pass_endgame();

                Move::new_pass_with_score(score)
            } else {
                let score = self.solve();
                Move::new_no_move_with_score(score)
            }
        } else {
            Self::evaluate_movelist(self, &move_list, &hash_data, alpha, 0);

            let mut best_move = Move::new_min_score();

            for move_ in move_list.iter() {
                self.state.update_endgame(move_);
                move_.score.set(-self.nws_endgame(-beta));
                self.state.restore_endgame(move_);

                if move_.score.get() > best_move.score.get() {
                    best_move = move_.clone();
                    if best_move.score.get() >= beta {
                        break;
                    }
                }
            }

            best_move
        };

        if self.is_running() {
            cost += self.shared.n_nodes.load(Ordering::Relaxed) as i64;

            self.hash_table.store(&StoreArgs {
                position: self.state.position(),
                depth: self.state.n_empties(),
                selectivity: NO_SELECTIVITY,
                cost: Self::ilog2(cost),
                alpha,
                beta,
                score: best_move.score.get(),
                move_: best_move.x,
            });

            return best_move.score.get();
        }

        alpha
    }

    /// Compute the integer logarithm base 2 of a number.
    /// Returns 0 if the number is 0.
    ///
    /// This is a helper function for computing the cost of a move.
    fn ilog2(n: i64) -> i32 {
        if n == 0 {
            return 0;
        }

        n.ilog2() as i32
    }

    /// Null Window Search at shallow endgame depth (when empty squares <= DEPTH_TO_SHALLOW_SEARCH).
    /// This is a null window search that returns a score from the player's perspective.
    ///
    /// The function is used for positions with up to DEPTH_TO_SHALLOW_SEARCH (7) empty squares.
    /// It uses parity-based move ordering to optimize the search:
    /// - Moves are sorted so quadrants with odd parity are searched first
    /// - This improves alpha-beta pruning since quadrants with odd parity tend to be more constrained
    ///
    /// Like search_shallow() in Edax
    fn endgame_shallow(&mut self, alpha: i32) -> i32 {
        #[cfg(debug_assertions)]
        {
            let empty_count = self.state.position().empties().count_ones();
            debug_assert!(empty_count <= DEPTH_TO_SHALLOW_SEARCH as u32);
        }

        let beta = alpha + 1;
        let mut best_score = -SCORE_INF;

        if let Some(score) = self.state.stability_cutoff_nws(alpha) {
            return score;
        }

        // TODO #15: Don't clone, this is probably slow.
        let empties = self.state.empties().clone();

        let parity = self.state.parity();

        if parity > 0 && parity < 15 {
            for empty in empties.iter_odd(parity).chain(empties.iter_even(parity)) {
                if NEIGHBOUR[empty.x as usize] & self.state.position().opponent() != 0 {
                    let move_ = Move::new(self.state.position(), empty.x);
                    if move_.flipped != 0 {
                        self.state.update_endgame(&move_);
                        let score = if self.state.n_empties() == 4 {
                            -self.solve_4(-beta)
                        } else {
                            -self.endgame_shallow(-beta)
                        };
                        self.state.restore_endgame(&move_);

                        if score >= beta {
                            return score;
                        } else if score > best_score {
                            best_score = score;
                        }
                    }
                }
            }
        } else {
            for empty in empties.iter() {
                if NEIGHBOUR[empty.x as usize] & self.state.position().opponent() != 0 {
                    let move_ = Move::new(self.state.position(), empty.x);
                    if move_.flipped != 0 {
                        self.state.update_endgame(&move_);
                        let score = if self.state.n_empties() == 4 {
                            -self.solve_4(-beta)
                        } else {
                            -self.endgame_shallow(-beta)
                        };
                        self.state.restore_endgame(&move_);

                        if score >= beta {
                            return score;
                        } else if score > best_score {
                            best_score = score;
                        }
                    }
                }
            }
        };

        if best_score == -SCORE_INF {
            if self.state.position().opponent_has_moves() {
                self.state.pass_endgame();
                best_score = -self.endgame_shallow(-beta);
                self.state.pass_endgame();
            } else {
                best_score = self.solve();
            }
        }

        best_score
    }

    /// Compute score for a position with 4 empty squares.
    /// This is a null window search that returns a score from the player's perspective.
    ///
    /// The function uses parity-based move ordering to optimize the search:
    /// - Moves are sorted so quadrants with odd parity are searched first
    /// - This improves alpha-beta pruning since quadrants with odd parity tend to be more constrained
    ///
    /// Like search_solve_4() in Edax
    fn solve_4(&mut self, alpha: i32) -> i32 {
        #[cfg(debug_assertions)]
        {
            let empties = self.state.position().empties();
            debug_assert_eq!(empties.count_ones(), 4);
            debug_assert_eq!(self.state.empties().len(), 4);
        }

        let beta = alpha + 1;

        // TODO #15 further optimization: add dedicated function for taking 4 empties using unwrap_unchecked()
        let (mut x1, mut x2, mut x3, mut x4) = {
            let mut iter = self.state.empties().iter();

            (
                iter.next().unwrap().x as usize,
                iter.next().unwrap().x as usize,
                iter.next().unwrap().x as usize,
                iter.next().unwrap().x as usize,
            )
        };

        #[cfg(debug_assertions)]
        {
            let empties = self.state.position().empties();
            debug_assert_eq!(empties.count_ones(), 4);
            debug_assert_ne!(empties & (1 << x1), 0);
            debug_assert_ne!(empties & (1 << x2), 0);
            debug_assert_ne!(empties & (1 << x3), 0);
            debug_assert_ne!(empties & (1 << x4), 0);
            debug_assert_ne!(x1, x2);
            debug_assert_ne!(x1, x3);
            debug_assert_ne!(x1, x4);
            debug_assert_ne!(x2, x3);
            debug_assert_ne!(x2, x4);
            debug_assert_ne!(x3, x4);
        }

        // TODO #15: Move this to top of function, since it doesn't depend on the empties.
        if let Some(score) = self.state.stability_cutoff_nws(alpha) {
            return score;
        }

        let parity = self.state.parity();

        // parity based move sorting.
        // The following hole sizes are possible:
        //    4 - 1 3 - 2 2 - 1 1 2 - 1 1 1 1
        // Only the 1 1 2 case needs move sorting.
        if parity & QUADRANT_ID[x1] == 0 {
            if parity & QUADRANT_ID[x2] != 0 {
                if parity & QUADRANT_ID[x3] != 0 {
                    (x1, x2, x3) = (x2, x3, x1); // case 1(x2) 1(x3) 2(x1 x4)
                } else {
                    (x1, x2, x3, x4) = (x2, x4, x1, x3); // case 1(x2) 1(x4) 2(x1 x3)
                }
            } else if parity & QUADRANT_ID[x3] != 0 {
                (x1, x2, x3, x4) = (x3, x4, x1, x2); // case 1(x3) 1(x4) 2(x1 x2)
            }
        } else if parity & QUADRANT_ID[x2] == 0 {
            if parity & QUADRANT_ID[x3] != 0 {
                (x2, x3) = (x3, x2); // case 1(x1) 1(x3) 2(x2 x4)
            } else {
                (x2, x3, x4) = (x4, x2, x3); // case 1(x1) 1(x4) 2(x2 x3)
            }
        }

        // After sorting either of these is true:
        // - x1 is in a quadrant with odd parity
        // - all quadrants have even parity
        #[cfg(debug_assertions)]
        {
            let x1_odd_parity = self.state.parity() & QUADRANT_ID[x1] != 0;
            let all_even_parity = self.state.parity() == 0;

            debug_assert!(x1_odd_parity || all_even_parity);
        }

        let mut best_score = -SCORE_INF;

        if NEIGHBOUR[x1] & self.state.position().opponent() != 0 {
            let move_ = Move::new(self.state.position(), x1 as i32);
            if move_.flipped != 0 {
                self.state.update_endgame(&move_);
                best_score = -self.solve_3(-beta);
                self.state.restore_endgame(&move_);

                if best_score >= beta {
                    return best_score;
                }
            }
        }

        if NEIGHBOUR[x2] & self.state.position().opponent() != 0 {
            let move_ = Move::new(self.state.position(), x2 as i32);
            if move_.flipped != 0 {
                self.state.update_endgame(&move_);
                let score = -self.solve_3(-beta);
                self.state.restore_endgame(&move_);

                if best_score >= beta {
                    return best_score;
                } else if score > best_score {
                    best_score = score;
                }
            }
        }

        if NEIGHBOUR[x3] & self.state.position().opponent() != 0 {
            let move_ = Move::new(self.state.position(), x3 as i32);
            if move_.flipped != 0 {
                self.state.update_endgame(&move_);
                let score = -self.solve_3(-beta);
                self.state.restore_endgame(&move_);

                if best_score >= beta {
                    return best_score;
                } else if score > best_score {
                    best_score = score;
                }
            }
        }

        if NEIGHBOUR[x4] & self.state.position().opponent() != 0 {
            let move_ = Move::new(self.state.position(), x4 as i32);
            if move_.flipped != 0 {
                self.state.update_endgame(&move_);
                let score = -self.solve_3(-beta);
                self.state.restore_endgame(&move_);

                if score > best_score {
                    best_score = score;
                }
            }
        }

        if best_score == -SCORE_INF {
            if self.state.position().opponent_has_moves() {
                self.state.pass_endgame();
                best_score = -self.solve_4(-beta);
                self.state.pass_endgame();
            } else {
                best_score = self.solve();
            }
        }

        best_score
    }

    /// Compute score for a position with 3 empty squares.
    /// This is a null window search that returns a score from the player's perspective.
    ///
    /// The function uses parity-based move ordering to optimize the search:
    /// - Moves are sorted so single-square quadrants are searched first
    /// - This improves alpha-beta pruning since single squares tend to be more constrained
    ///
    /// Like search_solve_3() in Edax
    fn solve_3(&self, alpha: i32) -> i32 {
        #[cfg(debug_assertions)]
        {
            let empties = self.state.position().empties();
            debug_assert_eq!(empties.count_ones(), 3);
            debug_assert_eq!(self.state.empties().len(), 3);
        }

        let beta = alpha + 1;

        // TODO #15 further optimization: add dedicated function for taking 3 empties using unwrap_unchecked()
        let (mut x1, mut x2, mut x3) = {
            let mut iter = self.state.empties().iter();

            (
                iter.next().unwrap().x as usize,
                iter.next().unwrap().x as usize,
                iter.next().unwrap().x as usize,
            )
        };

        #[cfg(debug_assertions)]
        {
            let empties = self.state.position().empties();
            debug_assert_eq!(empties.count_ones(), 3);
            debug_assert_ne!(empties & (1 << x1), 0);
            debug_assert_ne!(empties & (1 << x2), 0);
            debug_assert_ne!(empties & (1 << x3), 0);
            debug_assert_ne!(x1, x2);
            debug_assert_ne!(x1, x3);
            debug_assert_ne!(x2, x3);
        }

        let parity = self.state.parity();

        // parity based move sorting
        if parity & QUADRANT_ID[x1] == 0 {
            if parity & QUADRANT_ID[x2] != 0 {
                (x1, x2) = (x2, x1); // case 1(x2) 2(x1 x3)
            } else {
                (x1, x2, x3) = (x3, x1, x2); // case 1(x3) 2(x1 x2)
            }
        }

        #[cfg(debug_assertions)]
        {
            let q1 = QUADRANT_ID[x1];
            let q2 = QUADRANT_ID[x2];
            let q3 = QUADRANT_ID[x3];

            // If empties are within 2 quadrants:
            // x1 must be alone in a quadrant.
            // x2 and x3 must be in the other quadrant.
            if (q1 | q2 | q3).count_ones() == 2 {
                debug_assert_ne!(q1, q2);
                debug_assert_eq!(q2, q3);
            }
        }

        let mut best_score = -SCORE_INF;

        // TODO #15 Further optimization: try making Position::new_from_parent_and_move return Option<Position>

        if NEIGHBOUR[x1] & self.state.position().opponent() != 0 {
            let (next, flipped) = Position::new_from_parent_and_move(self.state.position(), x1);
            if flipped != 0 {
                best_score = -Self::solve_2(&next, -beta, x2, x3);
                if best_score >= beta {
                    return best_score;
                }
            }
        }

        if NEIGHBOUR[x2] & self.state.position().opponent() != 0 {
            let (next, flipped) = Position::new_from_parent_and_move(self.state.position(), x2);
            if flipped != 0 {
                let score = -Self::solve_2(&next, -beta, x1, x3);
                if score >= beta {
                    return score;
                } else if score > best_score {
                    best_score = score;
                }
            }
        }

        if NEIGHBOUR[x3] & self.state.position().opponent() != 0 {
            let (next, flipped) = Position::new_from_parent_and_move(self.state.position(), x3);
            if flipped != 0 {
                let score = -Self::solve_2(&next, -beta, x1, x2);
                if score > best_score {
                    best_score = score;
                }
            }
        }

        if best_score == -SCORE_INF {
            best_score = SCORE_INF;

            if NEIGHBOUR[x1] & self.state.position().player() != 0 {
                let (next, flipped) =
                    Position::new_from_parent_and_pass_and_move(self.state.position(), x1);
                if flipped != 0 {
                    best_score = Self::solve_2(&next, alpha, x2, x3);
                    if best_score <= alpha {
                        return best_score;
                    }
                }
            }

            if NEIGHBOUR[x2] & self.state.position().player() != 0 {
                let (next, flipped) =
                    Position::new_from_parent_and_pass_and_move(self.state.position(), x2);
                if flipped != 0 {
                    let score = Self::solve_2(&next, alpha, x1, x3);
                    if score <= alpha {
                        return score;
                    } else if score < best_score {
                        best_score = score;
                    }
                }
            }

            if NEIGHBOUR[x3] & self.state.position().player() != 0 {
                let (next, flipped) =
                    Position::new_from_parent_and_pass_and_move(self.state.position(), x3);
                if flipped != 0 {
                    let score = Self::solve_2(&next, alpha, x1, x2);
                    if score < best_score {
                        best_score = score;
                    }
                }
            }

            if best_score == SCORE_INF {
                best_score = self.state.position().final_score_with_empty(3);
            }
        }

        best_score
    }

    /// Compute score for a position with 2 empty squares.
    /// This is a null window search that returns a score from the player's perspective.
    ///
    /// The returned value may be inaccurate if it's > alpha, but this is fine since we only
    /// care whether the score exceeds alpha for pruning purposes. We don't use beta because
    /// we're only interested in alpha cutoffs - the exact score doesn't matter once we know
    /// it's high enough to cause a cutoff.
    ///
    /// Like board_solve_2() in Edax
    fn solve_2(position: &Position, alpha: i32, x1: usize, x2: usize) -> i32 {
        #[cfg(debug_assertions)]
        {
            let empties = position.empties();
            debug_assert_eq!(empties.count_ones(), 2);
            debug_assert_ne!(empties & (1 << x1), 0);
            debug_assert_ne!(empties & (1 << x2), 0);
            debug_assert_ne!(x1, x2);
        }

        let beta = alpha + 1;

        let mut best_score = -SCORE_INF;

        if NEIGHBOUR[x1] & position.opponent() != 0 {
            let (next, flipped) = Position::new_from_parent_and_move(position, x1);
            if flipped != 0 {
                best_score = Self::solve_1(&next, beta, x2);
            }
        }

        if best_score < beta {
            if NEIGHBOUR[x2] & position.opponent() != 0 {
                let (next, flipped) = Position::new_from_parent_and_move(position, x2);
                if flipped != 0 {
                    let score = Self::solve_1(&next, beta, x1);
                    if score > best_score {
                        best_score = score;
                    }
                }
            }

            if best_score == -SCORE_INF {
                // player has no moves
                debug_assert_eq!(position.get_moves(), 0);
                best_score = SCORE_INF;

                if NEIGHBOUR[x1] & position.player() != 0 {
                    let (next, flipped) = Position::new_from_parent_and_pass_and_move(position, x1);
                    if flipped != 0 {
                        best_score = -Self::solve_1(&next, -alpha, x2);
                    }
                }

                if best_score > alpha {
                    if NEIGHBOUR[x2] & position.player() != 0 {
                        let (next, flipped) =
                            Position::new_from_parent_and_pass_and_move(position, x2);
                        if flipped != 0 {
                            let score = -Self::solve_1(&next, -alpha, x1);
                            if score < best_score {
                                best_score = score;
                            }
                        }
                    }
                    if best_score == SCORE_INF {
                        best_score = position.final_score_with_empty(2);
                    }
                }
            }
        }

        best_score
    }

    /// Compute score for a position with 1 empty square.
    /// This is a null window search that returns a score from the opponent's perspective.
    ///
    /// The returned value may be inaccurate if it's >= beta, but this is fine since we only
    /// care whether the score exceeds beta for pruning purposes. We don't use alpha because
    /// we're only interested in beta cutoffs - the exact score doesn't matter once we know
    /// it's high enough to cause a cutoff.
    ///
    /// Like board_solve_1() in Edax
    fn solve_1(position: &Position, beta: i32, x: usize) -> i32 {
        #[cfg(debug_assertions)]
        {
            let empties = position.empties();
            debug_assert_eq!(empties.count_ones(), 1);
            debug_assert_ne!(empties & (1 << x), 0);
        }

        // Compute score from the opponent's perspective.
        let mut score = 2 * position.opponent().count_ones() as i32 - SCORE_MAX;

        // How many discs are flipped if player makes move `x`.
        let n_flips = count_last_flip(x, position.player()) as i32;

        // If discs are flipped, the move `x` is legal for player.
        if n_flips != 0 {
            // Subtract number of opponent's discs flipped.
            return score - n_flips;
        }

        // Check if opponent has a guaranteed draw or win, even if `x` is not a valid move for them.
        if score >= 0 {
            // Opponent has guaranteed draw or win, so:
            // - if `x` is a valid move, we will play it.
            // - if `x` is not a valid move, it will be counted to our final score anyway.
            //
            // In both cases we can add 2 to our score.
            score += 2;

            // Check for cut-off since computing flipped discs is expensive.
            // Score can only grow from here on. Returning an inaccurate value on beta cut-off is fine.
            if score < beta {
                // Compute the number of discs flipped for opponent if they make move `x`.
                let n_flips = count_last_flip(x, position.opponent()) as i32;

                // Add number of opponent's discs flipped.
                // Note that if n_flips is 0, this is still correct.
                score += n_flips;
            }
        } else {
            // Opponent has fewer discs than player, so they may lose.

            // Check for cut-off since computing flipped discs is expensive.
            // Score can only grow from here on. Returning an inaccurate value on beta cut-off is fine.
            if score < beta {
                // Compute the number of discs flipped for opponent if they make move `x`.
                let n_flips = count_last_flip(x, position.opponent()) as i32;

                // If discs are flipped, the move `x` is legal for opponent.
                if n_flips != 0 {
                    // Add number of opponent's discs flipped.
                    score += n_flips + 2;
                }

                // If no discs are flipped, nobody can move.
                // This means that opponent loses.
                // Note that score is already the correct value.
            }
        }

        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl Search {
        /// Change position, since creating a Search is slow.
        /// Requires a `level` argument, since SearchConfig does not save it.
        fn set_position(&mut self, position: &Position, level: i32) {
            self.state = SearchState::new(position);
            self.config = SearchConfig::new(level, position.count_empty() as i32);
        }
    }

    /// Solves a position using a naive alpha-beta search.
    fn solve_naive_internal(position: &mut Position, mut alpha: i32, beta: i32) -> i32 {
        let moves = position.iter_move_indices();

        // If no moves available
        if moves.is_empty() {
            // Check if the game is finished
            if position.get_opponent_moves() == 0 {
                // Game is over, return final evaluation
                return position.final_score() as i32;
            }

            // Recursively evaluate after passing
            position.pass();
            let score = -solve_naive_internal(position, -beta, -alpha);
            position.pass();
            return score;
        }

        for move_ in moves {
            let flipped = position.do_move(move_);
            let score = -solve_naive_internal(position, -beta, -alpha);
            position.undo_move(move_, flipped);

            if alpha >= beta {
                break; // Beta cutoff
            }

            alpha = alpha.max(score);
        }

        alpha
    }

    fn solve_naive(position: &Position) -> i32 {
        let mut position = *position;
        solve_naive_internal(&mut position, SCORE_MIN, SCORE_MAX)
    }

    #[test]
    fn test_solve_1() {
        struct Case {
            description: &'static str,
            player: u64,
            opponent: u64,

            // Index of only empty square
            x: usize,

            // Score from opponent's perspective
            // Some for known solutions, None for random positions
            score: Option<i32>,
        }

        let mut cases = vec![
            Case {
                player: 0xFFFFFFFFFFFFFFFE,
                opponent: 0x0000000000000000,
                x: 0,
                score: Some(-64),
                description: "nobody has moves, player wins",
            },
            Case {
                player: 0x0000000000000000,
                opponent: 0xFFFFFFFFFFFFFFFE,
                x: 0,
                score: Some(64),
                description: "nobody has moves, opponent wins",
            },
            // No moves and draw is impossible with 63 discs on the board.
            Case {
                player: 0xFFFFFFFFFFFFFFFC,
                opponent: 0x0000000000000002,
                x: 0,
                score: Some(-64),
                description: "player has move and wins",
            },
            Case {
                player: 0x00000000FFFFFCFC,
                opponent: 0xFFFFFFFF00000302,
                x: 0,
                score: Some(0),
                description: "player has move and draws",
            },
            Case {
                player: 0x0000000000000004,
                opponent: 0xFFFFFFFFFFFFFFFA,
                x: 0,
                score: Some(58),
                description: "player has move and loses",
            },
            Case {
                player: 0x0000000000000002,
                opponent: 0xFFFFFFFFFFFFFFFC,
                x: 0,
                score: Some(64),
                description: "opponent has move and wins",
            },
            Case {
                player: 0xFFFFFFFF00000302,
                opponent: 0x00000000FFFFFCFC,
                x: 0,
                score: Some(0),
                description: "opponent has move and draws",
            },
            Case {
                player: 0xFFFFFFFFFFFFFFFA,
                opponent: 0x0000000000000004,
                x: 0,
                score: Some(-58),
                description: "opponent has move and loses",
            },
        ];

        for _ in 0..1000 {
            let position = Position::new_random_with_empties(1);
            cases.push(Case {
                player: position.player(),
                opponent: position.opponent(),
                x: position.empties().trailing_zeros() as usize,
                score: None,
                description: "random position",
            });
        }

        for case in cases.iter() {
            let position = Position::new_from_bitboards(case.player, case.opponent);

            println!();
            println!("--- {} ---", case.description);
            println!();
            println!("{}", position);

            let empties = position.empties();
            assert_eq!(empties.count_ones(), 1);
            assert_eq!(empties, 1 << case.x);

            // Invert naive score, so we match the opponent's perspective of solve_1().
            let score = -solve_naive(&position);

            if let Some(expected) = case.score {
                assert_eq!(score, expected);
            }

            for beta in [
                SCORE_MIN,
                score - 4,
                score - 2,
                score,
                score + 2,
                score + 4,
                SCORE_MAX,
            ] {
                let beta = beta.clamp(SCORE_MIN, SCORE_MAX);

                let solved = Search::solve_1(&position, beta, case.x);

                // solve_1() may return inaccurate value on beta cut-off.
                let is_cutoff = solved >= beta;

                let ok = if is_cutoff {
                    // Cut-off found, check if score justifies it.
                    score >= beta
                } else {
                    // No cut-off, score should be accurate.
                    solved == score
                };

                if !ok {
                    println!("beta: {}", beta);
                    println!("expected: {}", score);
                    println!("found: {}", solved);
                    panic!("solve_1 returned incorrect result");
                }
            }
        }
    }

    #[test]
    fn test_solve_2() {
        struct Case {
            description: &'static str,
            player: u64,
            opponent: u64,

            // Indices of empty squares
            x1: usize,
            x2: usize,

            // Score from player's perspective
            // Some for known solutions, None for random positions
            score: Option<i32>,
        }

        let mut cases = vec![
            Case {
                player: 0xFFFFFFFFFFFFFFFC,
                opponent: 0x0000000000000000,
                x1: 0,
                x2: 1,
                score: Some(64),
                description: "nobody has moves, player wins",
            },
            Case {
                player: 0xFFFFFFFE00000000,
                opponent: 0x00000000FEFFFFFF,
                x1: 24,
                x2: 32,
                score: Some(0),
                description: "nobody has moves, draw",
            },
            Case {
                player: 0x0000000000000000,
                opponent: 0xFFFFFFFFFFFFFFFC,
                x1: 0,
                x2: 1,
                score: Some(-64),
                description: "nobody has moves, opponent wins",
            },
            Case {
                player: 0xFFFFFFFFFFFFFFF8,
                opponent: 0x0000000000000004,
                x1: 0,
                x2: 1,
                score: Some(64),
                description: "player has moves, player wins",
            },
            Case {
                player: 0xF8000000FFFFF8F8,
                opponent: 0x07FFFFFF00000704,
                x1: 0,
                x2: 1,
                score: Some(0),
                description: "player has moves, draw",
            },
            Case {
                player: 0x0000000000000005,
                opponent: 0xFFFFFFFFFFFFFFE8,
                x1: 1,
                x2: 4,
                score: Some(-62),
                description: "player has moves, opponent wins",
            },
            Case {
                player: 0xFFFFFFFFFFFFFFCE,
                opponent: 0x0000000000000001,
                x1: 4,
                x2: 5,
                score: Some(54),
                description: "opponent has moves, player wins",
            },
            Case {
                player: 0x07FFFFFF00000704,
                opponent: 0xF8000000FFFFF8F8,
                x1: 0,
                x2: 1,
                score: Some(0),
                description: "opponent has moves, draw",
            },
            Case {
                player: 0x0000000000000004,
                opponent: 0xFFFFFFFFFFFFFFF8,
                x1: 0,
                x2: 1,
                score: Some(-64),
                description: "opponent has moves, opponent wins",
            },
        ];

        for _ in 0..1000 {
            let position = Position::new_random_with_empties(2);

            let mut empties = position.empties();
            let x1 = empties.trailing_zeros() as usize;
            empties &= !(1 << x1);
            let x2 = empties.trailing_zeros() as usize;

            cases.push(Case {
                player: position.player(),
                opponent: position.opponent(),
                x1,
                x2,
                score: None,
                description: "random position",
            });
        }

        for case in cases.iter() {
            let position = Position::new_from_bitboards(case.player, case.opponent);

            println!();
            println!("--- {} ---", case.description);
            println!();
            println!("{}", position);

            let score = solve_naive(&position);

            if let Some(expected) = case.score {
                assert_eq!(score, expected);
            }

            for alpha in [
                SCORE_MIN,
                score - 4,
                score - 2,
                score,
                score + 2,
                score + 4,
                SCORE_MAX,
            ] {
                let alpha = alpha.clamp(SCORE_MIN, SCORE_MAX);

                let solved = Search::solve_2(&position, alpha, case.x1, case.x2);

                let ok = (solved > alpha) == (score > alpha);

                if !ok {
                    println!("alpha: {}", alpha);
                    println!("expected: {}", score);
                    println!("found: {}", solved);
                    panic!("solve_2 returned incorrect result");
                }
            }
        }
    }

    #[test]
    fn test_solve_3() {
        struct Case {
            description: &'static str,
            player: u64,
            opponent: u64,

            // Score from player's perspective
            // Some for known solutions, None for random positions
            score: Option<i32>,
        }

        let mut cases = vec![
            Case {
                player: 0xFFFFFFFFFFFFFFF8,
                opponent: 0x0000000000000000,
                score: Some(64),
                description: "nobody has moves, player wins",
            },
            // When there are no moves, it's impossible to draw with 61 discs on the board.
            Case {
                player: 0x0000000000000000,
                opponent: 0xFFFFFFFFFFFFFFF8,
                score: Some(-64),
                description: "nobody has moves, opponent wins",
            },
            Case {
                player: 0xFFFFFFFFFFFFFFF0,
                opponent: 0x0000000000000008,
                score: Some(64),
                description: "player has moves, player wins",
            },
            Case {
                player: 0xF3000000FFFFFFF0,
                opponent: 0x0CFFFFFF00000008,
                score: Some(0),
                description: "player has moves, draw",
            },
            Case {
                player: 0x7000000000000005,
                opponent: 0x8FFFFFFFFFFFFFC8,
                score: Some(-34),
                description: "player has moves, opponent wins",
            },
            Case {
                player: 0x8FFFFFFFFFFFFF88,
                opponent: 0x7000000000000007,
                score: Some(20),
                description: "opponent has moves, player wins",
            },
            Case {
                player: 0x8F0F0F0F0F0FFF88,
                opponent: 0x70F0F0F0F0F00007,
                score: Some(0),
                description: "opponent has moves, draw",
            },
            Case {
                player: 0x0000000000000008,
                opponent: 0xFFFFFFFFFFFFFFF0,
                score: Some(-64),
                description: "opponent has moves, opponent wins",
            },
            Case {
                player: 0x805AACF2FAFEFEFE,
                opponent: 0x7F25130D05010100,
                score: Some(6),
                description: "previously failing random position - player has no moves",
            },
        ];

        for _ in 0..1000 {
            let position = Position::new_random_with_empties(3);

            cases.push(Case {
                player: position.player(),
                opponent: position.opponent(),
                score: None,
                description: "random position",
            });
        }

        let mut search = Search::new(&Position::new(), 0, 0);

        for case in cases.iter() {
            let position = Position::new_from_bitboards(case.player, case.opponent);

            search.set_position(&position, 0);

            println!();
            println!("--- {} ---", case.description);
            println!();
            println!("{}", position);

            let score = solve_naive(&position);

            if let Some(expected) = case.score {
                assert_eq!(score, expected);
            }

            for alpha in [
                SCORE_MIN,
                score - 4,
                score - 2,
                score,
                score + 2,
                score + 4,
                SCORE_MAX,
            ] {
                let alpha = alpha.clamp(SCORE_MIN, SCORE_MAX);

                let solved = search.solve_3(alpha);

                let ok = (solved > alpha) == (score > alpha);

                if !ok {
                    println!("alpha: {}", alpha);
                    println!("expected: {}", score);
                    println!("found: {}", solved);
                    panic!("solve_3 returned incorrect result");
                }
            }
        }
    }

    #[test]
    fn test_solve_4() {
        struct Case {
            description: &'static str,
            player: u64,
            opponent: u64,

            // Score from player's perspective
            // Some for known solutions, None for random positions
            score: Option<i32>,
        }

        let mut cases = vec![
            Case {
                player: 0xFFFFFFFFFFFFFFF0,
                opponent: 0x0000000000000000,
                score: Some(64),
                description: "nobody has moves, player wins",
            },
            Case {
                player: 0xFFFFFF7E00000000,
                opponent: 0x000000007EFFFFFF,
                score: Some(0),
                description: "nobody has moves, draw",
            },
            Case {
                player: 0x0000000000000000,
                opponent: 0xFFFFFFFFFFFFFFF0,
                score: Some(-64),
                description: "nobody has moves, opponent wins",
            },
            Case {
                player: 0xFFFFFFFFFFFFFFE0,
                opponent: 0x0000000000000010,
                score: Some(64),
                description: "player has moves, player wins",
            },
            Case {
                player: 0x220000FFFFFFE0E0,
                opponent: 0xDDFFFF0000001F10,
                score: Some(0),
                description: "player has moves, draw",
            },
            Case {
                player: 0x020000FFFFFFE0E0,
                opponent: 0xFDFFFF0000001F10,
                score: Some(-2),
                description: "player has moves, opponent wins",
            },
            Case {
                player: 0xFDFFFF0000001F10,
                opponent: 0x020000FFFFFFE0E0,
                score: Some(2),
                description: "opponent has moves, player wins",
            },
            Case {
                player: 0xDDFFFF0000001F10,
                opponent: 0x220000FFFFFFE0E0,
                score: Some(0),
                description: "opponent has moves, draw",
            },
            Case {
                player: 0x0000000000000010,
                opponent: 0xFFFFFFFFFFFFFFE0,
                score: Some(-64),
                description: "opponent has moves, opponent wins",
            },
        ];

        for _ in 0..1000 {
            let position = Position::new_random_with_empties(4);

            cases.push(Case {
                player: position.player(),
                opponent: position.opponent(),
                score: None,
                description: "random position",
            });
        }

        let mut search = Search::new(&Position::new(), 0, 0);

        for case in cases.iter() {
            let position = Position::new_from_bitboards(case.player, case.opponent);

            search.set_position(&position, 0);

            println!();
            println!("--- {} ---", case.description);
            println!();
            println!("{}", position);

            let score = solve_naive(&position);

            if let Some(expected) = case.score {
                assert_eq!(score, expected);
            }

            for alpha in [
                SCORE_MIN,
                score - 4,
                score - 2,
                score,
                score + 2,
                score + 4,
                SCORE_MAX,
            ] {
                let alpha = alpha.clamp(SCORE_MIN, SCORE_MAX);

                let solved = search.solve_4(alpha);

                let ok = (solved > alpha) == (score > alpha);

                if !ok {
                    println!("alpha: {}", alpha);
                    println!("expected: {}", score);
                    println!("found: {}", solved);
                    panic!("solve_4 returned incorrect result");
                }
            }
        }
    }

    #[test]
    fn test_endgame_shallow() {
        struct Case {
            description: &'static str,
            player: u64,
            opponent: u64,
            empties: usize,

            // Score from player's perspective
            // Some for known solutions, None for random positions
            score: Option<i32>,
        }

        let mut cases = vec![
            Case {
                player: 0xFFFFFFFFFFFFFFE0,
                opponent: 0x0000000000000000,
                empties: 5,
                description: "5 empties, nobody has moves, player wins",
                score: Some(64),
            },
            // Nobody has moves, draw is not possible with 59 discs on the board
            Case {
                player: 0x0000000000000000,
                opponent: 0xFFFFFFFFFFFFFFE0,
                empties: 5,
                description: "5 empties, nobody has moves, opponent wins",
                score: Some(-64),
            },
            Case {
                player: 0xFFFFFFFFFFFFFFC0,
                opponent: 0x0000000000000020,
                empties: 5,
                description: "5 empties, player has moves, player wins",
                score: Some(64),
            },
            Case {
                player: 0xFFFC000000000000,
                opponent: 0x0003FFFFFFFFFEF0,
                empties: 5,
                description: "5 empties, player has moves, draw",
                score: Some(0),
            },
            Case {
                player: 0x0000000000000080,
                opponent: 0xFFFFFFFFFFFFFF41,
                empties: 5,
                description: "5 empties, player has moves, opponent wins",
                score: Some(-58),
            },
            Case {
                player: 0xFFFFFFFFFFFFFF41,
                opponent: 0x0000000000000080,
                empties: 5,
                description: "5 empties, opponent has moves, player wins",
                score: Some(58),
            },
            Case {
                player: 0xFFFFFF8000003F20,
                opponent: 0x0000007FFFFFC0C0,
                empties: 5,
                description: "5 empties, opponent has moves, draw",
                score: Some(0),
            },
            Case {
                player: 0x0000000000000020,
                opponent: 0xFFFFFFFFFFFFFFC0,
                empties: 5,
                description: "5 empties, opponent has moves, opponent wins",
                score: Some(-64),
            },
            Case {
                player: 0xFFFFFFFFFFFFFFC0,
                opponent: 0x0000000000000000,
                empties: 6,
                description: "6 empties, nobody has moves, player wins",
                score: Some(64),
            },
            Case {
                player: 0xFFFFFFF800000000,
                opponent: 0x00000000F8FFFFFF,
                empties: 6,
                description: "6 empties, nobody has moves, draw",
                score: Some(0),
            },
            Case {
                player: 0x0000000000000000,
                opponent: 0xFFFFFFFFFFFFFFC0,
                empties: 6,
                description: "6 empties, nobody has moves, opponent wins",
                score: Some(-64),
            },
            Case {
                player: 0xFFFFFFFFFFFFFF80,
                opponent: 0x0000000000000040,
                empties: 6,
                description: "6 empties, player has moves, player wins",
                score: Some(64),
            },
            Case {
                player: 0x7FE0000000000000,
                opponent: 0x801FFFFFFFFFFFC0,
                empties: 6,
                description: "6 empties, player has moves, draw",
                score: Some(0),
            },
            Case {
                player: 0x0000000000000080,
                opponent: 0xFFFFFFFFFFFFFF40,
                empties: 6,
                description: "6 empties, player has moves, opponent wins",
                score: Some(-58),
            },
            Case {
                player: 0xFFFFFFFFFFFFFF40,
                opponent: 0x0000000000000080,
                empties: 6,
                description: "6 empties, opponent has moves, player wins",
                score: Some(58),
            },
            Case {
                player: 0x801FFFFFFFFFFFC0,
                opponent: 0x7FE0000000000000,
                empties: 6,
                description: "6 empties, opponent has moves, draw",
                score: Some(0),
            },
            Case {
                player: 0x0000000000000040,
                opponent: 0xFFFFFFFFFFFFFF80,
                empties: 6,
                description: "6 empties, opponent has moves, opponent wins",
                score: Some(-64),
            },
        ];

        // Generate random positions with different numbers of empty squares
        {
            let min_empties = 0;
            let max_empties = DEPTH_TO_SHALLOW_SEARCH as usize;

            for i in 0..1000 {
                let empties = min_empties + (i % (max_empties - min_empties + 1));
                let position = Position::new_random_with_empties(empties);

                cases.push(Case {
                    player: position.player(),
                    opponent: position.opponent(),
                    empties,
                    score: None,
                    description: "random position",
                });
            }
        }

        let mut search = Search::new(&Position::new(), 0, 0);

        for case in cases.iter() {
            let position = Position::new_from_bitboards(case.player, case.opponent);

            search.set_position(&position, 0);

            println!();
            println!("--- {} ---", case.description);
            println!();
            println!("{}", position);

            let score = solve_naive(&position);

            if let Some(expected) = case.score {
                assert_eq!(score, expected);
            }

            assert_eq!(position.count_empty() as usize, case.empties);

            for alpha in [
                SCORE_MIN,
                score - 4,
                score - 2,
                score,
                score + 2,
                score + 4,
                SCORE_MAX,
            ] {
                let alpha = alpha.clamp(SCORE_MIN, SCORE_MAX);

                let solved = search.endgame_shallow(alpha);

                let ok = (solved > alpha) == (score > alpha);

                if !ok {
                    println!("alpha: {}", alpha);
                    println!("expected: {}", score);
                    println!("found: {}", solved);
                    panic!("endgame_shallow returned incorrect result");
                }
            }
        }
    }

    #[test]
    fn test_nws_endgame_fast() {
        struct Case {
            description: &'static str,
            player: u64,
            opponent: u64,
            empties: usize,

            // Score from player's perspective
            // Some for known solutions, None for random positions
            score: Option<i32>,
        }

        let mut cases = vec![Case {
            player: 0x0181B9A5B5A905A1,
            opponent: 0xFC7E465A4A566240,
            empties: 8,
            description: "8 empties, failing random position",
            score: Some(18),
        }];

        // Generate random positions with different numbers of empty squares
        {
            // nws_empties() handles up to 15 empties, but we only test up to 10 empties
            // because it takes too long to compute with the naive solver.

            // We have a separate test for up to 15 empties, which doesn't run by default.

            for empties in 7..=10 {
                for _ in 0..100 {
                    let position = Position::new_random_with_empties(empties);

                    cases.push(Case {
                        player: position.player(),
                        opponent: position.opponent(),
                        empties,
                        score: None,
                        description: "random position",
                    });
                }
            }
        }

        let mut search = Search::new(&Position::new(), 0, 0);

        // Pretend we've called Search::run(), otherwise nws_endgame() will return immediately
        search
            .shared
            .stop
            .store(Stop::Running as u8, Ordering::Release);

        for case in cases.iter() {
            let position = Position::new_from_bitboards(case.player, case.opponent);

            search.set_position(&position, 0);

            println!();
            println!("--- {} ---", case.description);
            println!();
            println!("{}", position);

            let score = solve_naive(&position);

            if let Some(expected) = case.score {
                assert_eq!(score, expected);
            }

            assert_eq!(position.count_empty() as usize, case.empties);

            for alpha in [
                SCORE_MIN,
                score - 4,
                score - 2,
                score,
                score + 2,
                score + 4,
                SCORE_MAX,
            ] {
                let alpha = alpha.clamp(SCORE_MIN, SCORE_MAX);

                let solved = search.nws_endgame(alpha);

                let ok = (solved > alpha) == (score > alpha);

                if !ok {
                    println!("alpha: {}", alpha);
                    println!("expected: {}", score);
                    println!("found: {}", solved);
                    panic!("nws_endgame returned incorrect result");
                }
            }
        }
    }

    #[test]
    fn test_nws_endgame_slow() {
        if std::env::var("RUN_NWS_ENDGAME_SLOW").is_err() {
            println!("Skipping slow nws_endgame tests. Set RUN_NWS_ENDGAME_SLOW environment variable to run them.");
            return;
        }

        struct Case {
            description: &'static str,
            player: u64,
            opponent: u64,
            empties: usize,

            // Score from player's perspective
            // Some for known solutions, None for random positions
            score: Option<i32>,
        }

        let mut cases = vec![];

        // Generate random positions with different numbers of empty squares
        {
            for empties in 11..=15 {
                for _ in 0..10 {
                    let position = Position::new_random_with_empties(empties);

                    cases.push(Case {
                        player: position.player(),
                        opponent: position.opponent(),
                        empties,
                        score: None,
                        description: "random position",
                    });
                }
            }
        }

        let mut search = Search::new(&Position::new(), 0, 0);

        // Pretend we've called Search::run(), otherwise nws_endgame() will return immediately
        search
            .shared
            .stop
            .store(Stop::Running as u8, Ordering::Release);

        for case in cases.iter() {
            let position = Position::new_from_bitboards(case.player, case.opponent);

            search.set_position(&position, 0);

            println!();
            println!("--- {} ---", case.description);
            println!();
            println!("{}", position);

            let score = solve_naive(&position);

            if let Some(expected) = case.score {
                assert_eq!(score, expected);
            }

            assert_eq!(position.count_empty() as usize, case.empties);

            for alpha in [
                SCORE_MIN,
                score - 4,
                score - 2,
                score,
                score + 2,
                score + 4,
                SCORE_MAX,
            ] {
                let alpha = alpha.clamp(SCORE_MIN, SCORE_MAX);

                let solved = search.nws_endgame(alpha);

                let ok = (solved > alpha) == (score > alpha);

                if !ok {
                    println!("alpha: {}", alpha);
                    println!("expected: {}", score);
                    println!("found: {}", solved);
                    panic!("nws_endgame returned incorrect result");
                }
            }
        }
    }

    fn test_nws_shallow<const USE_SHALLOW_TABLE: bool>() {
        let mut search = Search::new(&Position::new(), 0, 0);

        let mut positions = vec![];

        // Some regular positions, where player has moves
        for n_discs in 4..10 {
            positions.push(Position::new_random_with_discs(n_discs));
        }

        // Position where player has no moves, but opponent does
        let position = Position::new_from_bitboards(0x0000F818283E3800, 0x000000E0D0C0C0FC);
        assert_eq!(position.get_moves(), 0);
        assert_ne!(position.get_opponent_moves(), 0);
        positions.push(position);

        // Position where nobody has moves
        let position = Position::new_from_bitboards(0xFFFFFFFFFFFFFFFF, 0x0);
        assert_eq!(position.get_moves(), 0);
        assert_eq!(position.get_opponent_moves(), 0);
        positions.push(position);

        for position in positions.iter() {
            for depth in 2..=5 {
                println!();
                println!("---");
                println!();
                println!("{}", position);
                println!("depth: {}", depth);

                search.set_position(position, 0);

                let expected = search.state.eval_naive(depth, SCORE_MIN, SCORE_MAX);

                for alpha in [SCORE_MIN, expected - 1, expected, expected + 1, SCORE_MAX] {
                    // Clearing tables is required because the tables are reused for different positions and depths.
                    // Otherwise the results will pollute each other and the test will fail.
                    unsafe {
                        if USE_SHALLOW_TABLE {
                            search.shallow_table.clear_unchecked();
                        } else {
                            search.hash_table.clear_unchecked();
                        }
                    }

                    let score = search.nws_shallow::<USE_SHALLOW_TABLE>(alpha, depth);

                    let ok = (score > alpha) == (expected > alpha);

                    if !ok {
                        println!("alpha: {}", alpha);
                        println!("expected: {}", expected);
                        println!("found: {}", score);
                        panic!("nws_shallow returned incorrect result");
                    }
                }
            }
        }
    }

    #[test]
    fn test_nws_shallow_with_shallow_table() {
        test_nws_shallow::<true>();
    }

    #[test]
    fn test_nws_shallow_with_hash_table() {
        test_nws_shallow::<false>();
    }

    #[test]
    fn test_pvs_shallow() {
        let mut search = Search::new(&Position::new(), 0, 0);

        let mut positions = vec![];

        // Some regular positions, where player has moves
        for n_discs in 4..10 {
            positions.push(Position::new_random_with_discs(n_discs));
        }

        // Position where player has no moves, but opponent does
        let position = Position::new_from_bitboards(0x0000F818283E3800, 0x000000E0D0C0C0FC);
        assert_eq!(position.get_moves(), 0);
        assert_ne!(position.get_opponent_moves(), 0);
        positions.push(position);

        // Position where nobody has moves
        let position = Position::new_from_bitboards(0xFFFFFFFFFFFFFFFF, 0x0);
        assert_eq!(position.get_moves(), 0);
        assert_eq!(position.get_opponent_moves(), 0);
        positions.push(position);

        for position in positions.iter() {
            for depth in 2..=5 {
                println!();
                println!("---");
                println!();
                println!("{}", position);
                println!("depth: {}", depth);

                search.set_position(position, 0);

                let expected = search.state.eval_naive(depth, SCORE_MIN, SCORE_MAX);

                for (alpha, beta) in [
                    (SCORE_MIN, SCORE_MAX),
                    (-10, 0),
                    (0, 10),
                    (expected - 1, expected + 1),
                ] {
                    // Clearing tables is required because the tables are reused for different positions and depths.
                    // Otherwise the results will pollute each other and the test will fail.
                    unsafe {
                        search.shallow_table.clear_unchecked();
                    }
                    let score = search.pvs_shallow(alpha, beta, depth);

                    let ok = if expected < alpha {
                        score <= alpha
                    } else if expected > beta {
                        score >= beta
                    } else {
                        score == expected
                    };

                    if !ok {
                        println!("alpha: {}", alpha);
                        println!("beta: {}", beta);
                        println!("expected: {}", expected);
                        println!("found: {}", score);
                        panic!("pvs_shallow returned incorrect result");
                    }
                }
            }
        }
    }

    #[test]
    fn test_evaluate_move_wipeout() {
        let mut search = Search::new(&Position::new(), 0, 0);
        let hash_data = HashData::default();
        let position = Position::new_from_bitboards(0xFFFFFFFFFFFFFFFC, 0x2);
        let move_ = Move::new(&position, 0);
        assert!(move_.is_wipeout(&position));
        search.set_position(&position, 0);
        let score = search.evaluate_move(move_, &hash_data, 0, 0);
        assert_eq!(score, WEIGHT_WIPEOUT);
        assert_eq!(search.state.position(), &position);
    }

    #[test]
    fn test_evaluate_move_first_hash_move() {
        let mut search = Search::new(&Position::new(), 0, 0);
        let position = Position::new();
        let move_ = Move::new(&position, 19);
        let hash_data = HashData {
            move_: [19, 26],
            ..Default::default()
        };
        assert_eq!(move_.x, hash_data.move_[0] as i32);
        search.set_position(&position, 0);
        let score = search.evaluate_move(move_, &hash_data, 0, 0);
        assert_eq!(score, WEIGHT_FIRST_HASH_MOVE);
        assert_eq!(search.state.position(), &position);
    }

    #[test]
    fn test_evaluate_move_second_hash_move() {
        let mut search = Search::new(&Position::new(), 0, 0);
        let position = Position::new();
        let move_ = Move::new(&position, 26);
        let hash_data = HashData {
            move_: [19, 26],
            ..Default::default()
        };
        assert_eq!(move_.x, hash_data.move_[1] as i32);
        search.set_position(&position, 0);
        let score = search.evaluate_move(move_, &hash_data, 0, 0);
        assert_eq!(score, WEIGHT_SECOND_HASH_MOVE);
        assert_eq!(search.state.position(), &position);
    }

    #[test]
    fn test_evaluate_move_sort_depth_less_than_zero() {
        let mut search = Search::new(&Position::new(), 0, 0);
        let position = Position::new();
        let move_ = Move::new(&position, 19);
        let hash_data = HashData::default();
        search.set_position(&position, 0);
        let score = search.evaluate_move(move_.clone(), &hash_data, 0, -1);

        let expected_score = {
            let child = position.do_move_cloned(move_.x as usize);

            // Even parity before move, so no parity score
            assert_eq!(search.state.parity() & QUADRANT_ID[move_.x as usize], 1);
            let parity_score = 0;

            let depth_score = ((36 - child.potential_mobility()) * WEIGHT_POTENTIAL_MOBILITY)
                + (child.opponent_corner_stability() * WEIGHT_CORNER_STABILITY)
                + ((36 - child.weighted_mobility()) * WEIGHT_MOBILITY);

            let square_value = SQUARE_VALUE[move_.x as usize];

            parity_score + depth_score + square_value
        };

        assert_eq!(score, expected_score);
        assert_eq!(search.state.position(), &position);
    }

    #[test]
    fn test_evaluate_move_sort_depth_zero() {
        let mut search = Search::new(&Position::new(), 0, 0);
        let position = Position::new();
        let move_ = Move::new(&position, 19);
        let hash_data = HashData::default();
        search.set_position(&position, 0);
        let score = search.evaluate_move(move_.clone(), &hash_data, 0, 0);

        let expected_score = {
            let child = position.do_move_cloned(move_.x as usize);

            // Even parity before move, so no parity score
            assert_eq!(search.state.parity() & QUADRANT_ID[move_.x as usize], 1);
            let parity_score = 0;

            let mobility_score = ((36 - child.potential_mobility()) * WEIGHT_POTENTIAL_MOBILITY)
                + (child.opponent_edge_stability() * WEIGHT_EDGE_STABILITY)
                + ((36 - child.weighted_mobility()) * WEIGHT_MOBILITY);

            let square_value = SQUARE_VALUE[move_.x as usize];

            let eval = Eval::new(&child);
            let eval_score = ((SCORE_MAX - eval.heuristic()) >> 2) * WEIGHT_EVAL;

            parity_score + mobility_score + square_value + eval_score
        };

        assert_eq!(score, expected_score);
        assert_eq!(search.state.position(), &position);
    }

    #[test]
    fn test_evaluate_move_sort_depth_one() {
        let mut search = Search::new(&Position::new(), 0, 0);
        let position = Position::new();
        let move_ = Move::new(&position, 19);
        let hash_data = HashData::default();
        search.set_position(&position, 0);
        let sort_alpha = 0;
        let score = search.evaluate_move(move_.clone(), &hash_data, sort_alpha, 1);
        assert_eq!(search.state.position(), &position);

        let expected_score = {
            let child = position.do_move_cloned(move_.x as usize);

            // Even parity before move, so no parity score
            assert_eq!(search.state.parity() & QUADRANT_ID[move_.x as usize], 1);
            let parity_score = 0;

            let mobility_score = ((36 - child.potential_mobility()) * WEIGHT_POTENTIAL_MOBILITY)
                + (child.opponent_edge_stability() * WEIGHT_EDGE_STABILITY)
                + ((36 - child.weighted_mobility()) * WEIGHT_MOBILITY);

            let square_value = SQUARE_VALUE[move_.x as usize];

            search.set_position(&child, 0);

            let eval_score =
                ((SCORE_MAX - search.state.eval_1(SCORE_MIN, -sort_alpha)) >> 1) * WEIGHT_EVAL;

            parity_score + mobility_score + square_value + eval_score
        };

        assert_eq!(score, expected_score);
    }

    #[test]
    fn test_evaluate_move_sort_depth_two() {
        let mut search = Search::new(&Position::new(), 0, 0);
        let position = Position::new();
        let move_ = Move::new(&position, 19);
        let hash_data = HashData::default();
        search.set_position(&position, 0);
        let sort_alpha = 0;
        let score = search.evaluate_move(move_.clone(), &hash_data, sort_alpha, 2);
        assert_eq!(search.state.position(), &position);

        let expected_score = {
            let child = position.do_move_cloned(move_.x as usize);

            // Even parity before move, so no parity score
            assert_eq!(search.state.parity() & QUADRANT_ID[move_.x as usize], 1);
            let parity_score = 0;

            let mobility_score = ((36 - child.potential_mobility()) * WEIGHT_POTENTIAL_MOBILITY)
                + (child.opponent_edge_stability() * WEIGHT_EDGE_STABILITY)
                + ((36 - child.weighted_mobility()) * WEIGHT_MOBILITY);

            let square_value = SQUARE_VALUE[move_.x as usize];

            search.set_position(&child, 0);

            let eval_score =
                ((SCORE_MAX - search.state.eval_2(SCORE_MIN, -sort_alpha)) >> 1) * WEIGHT_EVAL;

            parity_score + mobility_score + square_value + eval_score
        };

        assert_eq!(score, expected_score);
    }

    #[test]
    fn test_evaluate_move_sort_depth_more_than_two() {
        let mut search = Search::new(&Position::new(), 0, 0);
        let position = Position::new();
        let move_ = Move::new(&position, 19);
        let hash_data = HashData::default();
        search.set_position(&position, 0);
        let sort_alpha = 0;
        let score = search.evaluate_move(move_.clone(), &hash_data, sort_alpha, 3);
        assert_eq!(search.state.position(), &position);

        let child = position.do_move_cloned(move_.x as usize);

        let expected_score = {
            // Even parity before move, so no parity score
            assert_eq!(search.state.parity() & QUADRANT_ID[move_.x as usize], 1);
            let parity_score = 0;

            let mobility_score = ((36 - child.potential_mobility()) * WEIGHT_POTENTIAL_MOBILITY)
                + (child.opponent_edge_stability() * WEIGHT_EDGE_STABILITY)
                + ((36 - child.weighted_mobility()) * WEIGHT_MOBILITY);

            let square_value = SQUARE_VALUE[move_.x as usize];

            search.set_position(&child, 0);

            let eval_score =
                (SCORE_MAX - search.pvs_shallow(SCORE_MIN, -sort_alpha, 3)) * WEIGHT_EVAL;

            parity_score + mobility_score + square_value + eval_score
        };

        assert_eq!(score, expected_score);

        let expected_score_with_hash = expected_score + WEIGHT_HASH;
        search.set_position(&position, 0);
        search.hash_table.store(&StoreArgs {
            position: &child,
            depth: 3,
            selectivity: 0,
            cost: 0,
            alpha: 0,
            beta: 0,
            score: 0,
            move_: 19,
        });

        let score_with_hash = search.evaluate_move(move_.clone(), &hash_data, sort_alpha, 3);
        assert_eq!(search.state.position(), &position);
        assert_eq!(score_with_hash, expected_score_with_hash);
    }

    #[test]
    fn test_evaluate_movelist() {
        let position = Position::new();

        let mut search = Search::new(&position, 0, 0);
        let move_list = MoveList::new(&position);

        for move_ in move_list.iter() {
            assert_eq!(move_.score.get(), 0);
        }

        let hash_data = HashData::default();
        let alpha = -10;
        let depth = 0;
        search.evaluate_movelist(&move_list, &hash_data, alpha, depth);

        let sort_alpha = SCORE_MIN.max(alpha - SORT_ALPHA_DELTA);
        let sort_depth = -1;

        for move_ in move_list.iter() {
            let expected_score =
                search.evaluate_move(move_.clone(), &hash_data, sort_alpha, sort_depth);
            assert_eq!(move_.score.get(), expected_score);

            assert_eq!(search.state.position(), &position);
        }
    }

    #[test]
    fn test_transposition_cutoff_nws() {
        let hash_data = HashData {
            depth: 2,
            selectivity: 1,
            cost: 0,
            date: 0,
            lower: -10,
            upper: 10,
            move_: [1, 2],
        };

        // Case 1: hash_data has lower selectivity
        assert_eq!(Search::transposition_cutoff_nws(&hash_data, 2, 3, 20), None);

        // Case 2: hash_data has lower depth
        assert_eq!(Search::transposition_cutoff_nws(&hash_data, 3, 1, 20), None);

        // Case 3: alpha is lower than lower
        assert_eq!(
            Search::transposition_cutoff_nws(&hash_data, 2, 1, -20),
            Some(-10)
        );

        // Case 4: alpha is greater than upper
        assert_eq!(
            Search::transposition_cutoff_nws(&hash_data, 2, 1, 20),
            Some(10)
        );

        // Case 5: alpha is within bounds
        assert_eq!(Search::transposition_cutoff_nws(&hash_data, 2, 1, 0), None);
    }

    #[test]
    fn test_nws_midgame() {
        let mut search = Search::new(&Position::new(), 0, 0);

        // Pretend we have entered via run()
        search
            .shared
            .stop
            .store(Stop::Running as u8, Ordering::Relaxed);

        let mut cases = vec![];

        // Case that fails because there are no moves for player
        cases.push((
            Position::new_from_bitboards(0x0000F818283E3800, 0x000000E0D0C0C0FC),
            4,
        ));

        // Case that fails when probcut is enabled
        cases.push((
            Position::new_from_bitboards(0x0000000030000000, 0x0000101808040000),
            4,
        ));

        for depth in 2..=5 {
            // Some regular positions, where player has moves
            for n_discs in 4..10 {
                let position = Position::new_random_with_discs(n_discs);
                cases.push((position, depth));
            }

            // Position where player has no moves, but opponent does
            let position = Position::new_from_bitboards(0x0000F818283E3800, 0x000000E0D0C0C0FC);
            cases.push((position, depth));

            // Position where nobody has moves
            let position = Position::new_from_bitboards(0xFFFFFFFFFFFFFFFF, 0x0);
            cases.push((position, depth));
        }

        for (position, depth) in cases {
            println!();
            println!("---");
            println!();
            println!("{}", position);
            println!("depth: {}", depth);

            search.set_position(&position, 0);

            let expected = search.state.eval_naive(depth, SCORE_MIN, SCORE_MAX);

            for alpha in [SCORE_MIN, expected - 1, expected, expected + 1, SCORE_MAX] {
                // Prevent integer underflow
                search.result.lock().unwrap().n_moves_left = position.count_moves();

                unsafe {
                    search.hash_table.clear_unchecked();
                    search.shallow_table.clear_unchecked();
                }

                let score = search.nws_midgame(alpha, depth, None);
                assert_eq!(*search.state.position(), position);

                let ok = (score > alpha) == (expected.clamp(SCORE_MIN + 1, SCORE_MAX - 1) > alpha);

                if !ok {
                    println!("alpha: {}", alpha);
                    println!("expected: {}", expected);
                    println!("found: {}", score);
                    panic!("nws_midgame returned incorrect result");
                }
            }
        }
    }

    #[test]
    fn test_pvs_midgame() {
        let mut search = Search::new(&Position::new(), 0, 0);

        // Pretend we have entered via run()
        search
            .shared
            .stop
            .store(Stop::Running as u8, Ordering::Relaxed);

        let mut positions = vec![];

        // Some regular positions, where player has moves
        for n_discs in 4..10 {
            positions.push(Position::new_random_with_discs(n_discs));
        }

        // Position where player has no moves, but opponent does
        let position = Position::new_from_bitboards(0x0000F818283E3800, 0x000000E0D0C0C0FC);
        assert_eq!(position.get_moves(), 0);
        assert_ne!(position.get_opponent_moves(), 0);
        positions.push(position);

        // Position where nobody has moves
        let position = Position::new_from_bitboards(0xFFFFFFFFFFFFFFFF, 0x0);
        assert_eq!(position.get_moves(), 0);
        assert_eq!(position.get_opponent_moves(), 0);
        positions.push(position);

        for position in positions.iter() {
            for depth in 2..=5 {
                println!();
                println!("---");
                println!();
                println!("{}", position);
                println!("depth: {}", depth);

                search.set_position(position, 0);

                let expected = {
                    let expected = search.state.eval_naive(depth, SCORE_MIN, SCORE_MAX);

                    if position.count_empty() == 0 {
                        expected
                    } else {
                        expected.clamp(SCORE_MIN + 1, SCORE_MAX - 1)
                    }
                };

                for (alpha, beta) in [
                    (SCORE_MIN, SCORE_MAX),
                    (-10, 0),
                    (0, 10),
                    (expected - 1, expected + 1),
                ] {
                    // Prevent integer underflow
                    search.result.lock().unwrap().n_moves_left = position.count_moves();

                    // Clearing tables is required because the tables are reused for different positions and depths.
                    // Otherwise the results will pollute each other and the test will fail.
                    unsafe {
                        search.shallow_table.clear_unchecked();
                    }
                    let score = search.pvs_midgame(alpha, beta, depth, None);

                    let ok = if expected < alpha {
                        score <= alpha
                    } else if expected > beta {
                        score >= beta
                    } else {
                        score == expected
                    };

                    if !ok {
                        println!("alpha: {}", alpha);
                        println!("beta: {}", beta);
                        println!("expected: {}", expected);
                        println!("found: {}", score);
                        panic!("pvs_midgame returned incorrect result");
                    }
                }
            }
        }
    }

    impl Search {
        fn route_pvs_naive(
            &mut self,
            alpha: i32,
            beta: i32,
            depth: i32,
            node: Option<Arc<Node>>,
        ) -> i32 {
            if self.state.n_empties() == depth {
                self.pvs_midgame(alpha, beta, depth, node)
            } else {
                self.state.eval_naive(depth, alpha, beta)
            }
        }
    }

    #[test]
    fn test_route_pvs() {
        let mut search = Search::new(&Position::new(), 0, 0);

        // Pretend we have entered via run()
        search
            .shared
            .stop
            .store(Stop::Running as u8, Ordering::Relaxed);

        let mut cases = vec![];

        // Case that caused overflow
        cases.push((
            Position::new_from_bitboards(0xFEEFD7B79D05093F, 0x0110284862F2F6C0),
            1,
        ));

        // Depth == 0 and depth == empty squares
        cases.push((Position::new_from_bitboards(0xFFFFFFFFFFFFFFFF, 0x0), 0));

        // Depth > 0 and depth == empty squares
        cases.push((Position::new_random_with_empties(1), 1));

        // Depth == 0 and depth < empty squares
        cases.push((Position::new_random_with_empties(10), 0));

        // Depth == 1 and depth < empty squares
        cases.push((Position::new_random_with_empties(10), 1));

        // Depth == 2 and depth < empty squares
        cases.push((Position::new_random_with_empties(10), 2));

        // Depth > 2 and depth < empty squares
        cases.push((Position::new_random_with_empties(10), 3));

        for (position, depth) in cases {
            println!();
            println!("---");
            println!();
            println!("{}", position);
            println!("depth: {}", depth);

            search.set_position(&position, 0);

            // Prevent integer underflow
            search.result.lock().unwrap().n_moves_left = position.count_moves();

            let expected = search.route_pvs_naive(SCORE_MIN, SCORE_MAX, depth, None);

            // Prevent integer underflow
            search.result.lock().unwrap().n_moves_left = position.count_moves();

            search.state.set_bound(SCORE_MAX, SCORE_MIN);

            let found = search.route_pvs(SCORE_MIN, SCORE_MAX, depth, None);
            assert_eq!(found, expected);
        }
    }

    // Testing of Search functions, in order of dependency
    //
    // Endgame search:
    // [x] Search::test_solve_1()
    // [x] Search::test_solve_2()
    // [x] Search::test_solve_3()
    // [x] Search::test_solve_4()
    // [x] Search::endgame_shallow()
    // [x] SearchState::stability_cutoff_nws()
    // [x] Search::nws_endgame()

    // Move ordering and shallow search
    // [x] SearchState::stability_cutoff_pvs()
    // [x] Search::nws_shallow_with_shallow_table()
    // [x] Search::pvs_shallow()
    // [x] Search::evaluate_move()
    // [x] Search::evaluate_movelist()

    // TODO: regular search
    // [x] Search::transposition_cutoff_nws()
    // [x] Search::nws_shallow_with_hash_table()
    // [ ] Search::probcut()
    // [x] Search::nws_midgame()
    // [x] Search::pvs_midgame()
    // [x] Search::route_pvs()
    // [ ] Search::pvs_root()
    // [ ] Search::aspiration_search()
    // [ ] Search::iterative_deepening()

    // TODO: other functions
    // [ ] Search::new()
    // [ ] Search::is_running()
    // [ ] Search::clock()
    // [ ] Search::count_nodes()
    // [ ] Search::sum_nodes()
    // [ ] Search::run()
    // [ ] Search::get_last_level()
    // [ ] Search::adjust_time()
    // [ ] Search::guess_move()
    // [ ] Search::get_time_spent()
    // [ ] Search::record_best_move()
    // [ ] Search::continue_search()
    // [ ] Search::is_depth_solving()
    // [ ] Search::is_pv_ok()
    // [ ] Search::solve()
    // [ ] Search::get_pv_cost()
    // [ ] Search::update_probcut()
    // [ ] Search::restore_probcut()
    // [ ] Search::etc_nws()
    // [ ] Search::ilog2()
}
