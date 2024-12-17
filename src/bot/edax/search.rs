use std::sync::atomic::{AtomicI64, AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::bot::edax::eval::Eval;
use crate::bot::edax::node::Node;
use crate::bot::edax::r#const::{
    DEPTH_TO_SHALLOW_SEARCH, ITERATIVE_MIN_EMPTIES, NEIGHBOUR, NO_SELECTIVITY,
    NWS_STABILITY_THRESHOLD, PROBCUT_D, QUADRANT_ID, RCD, SCORE_INF, SCORE_MAX, SCORE_MIN,
    SELECTIVITY_TABLE, SQUARE_VALUE,
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
            self.hash_table.clear();
            self.pv_table.clear();
            self.shallow_table.clear();
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
        let position = *self.state.position();
        let n_empties = self.state.n_empties();

        let mut min_depth = 9;
        if n_empties <= 27 {
            min_depth += (30 - n_empties) / 3;
        }

        let sort_depth = if depth >= min_depth {
            let mut sort_depth = (depth - 15) / 3;
            if let Some(hash_data) = self.pv_table.get(&position) {
                if (hash_data.upper as i32) < alpha {
                    sort_depth -= 2;
                }
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
        const WEIGHT_HASH: i32 = 1 << 15;
        const WEIGHT_EVAL: i32 = 1 << 15;
        const WEIGHT_MOBILITY: i32 = 1 << 15;
        const WEIGHT_CORNER_STABILITY: i32 = 1 << 11;
        const WEIGHT_EDGE_STABILITY: i32 = 1 << 11;
        const WEIGHT_POTENTIAL_MOBILITY: i32 = 1 << 5;
        const WEIGHT_LOW_PARITY: i32 = 1 << 3;
        const WEIGHT_MID_PARITY: i32 = 1 << 2;
        const WEIGHT_HIGH_PARITY: i32 = 1 << 1;

        let mut score;

        if move_.is_wipeout(self.state.position()) {
            score = 1 << 30;
        } else if move_.x == hash_data.move_[0] as i32 {
            score = 1 << 29;
        } else if move_.x == hash_data.move_[1] as i32 {
            score = 1 << 28;
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
            cost: cost.ilog2() as i32,
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
                bestscore = -self.nws_shallow::<USE_SHALLOW_TABLE>(beta, depth - 1);
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
            cost: cost.ilog2() as i32,
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

        if !self.is_running() {
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
            move_list.len() as i32,
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
            node.update(&move_, self);

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
                    node.update(&move_, self);
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
                    cost: cost.ilog2() as i32,
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
                    cost: cost.ilog2() as i32,
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

    /// Find out which tree search function to use and call it.
    ///
    /// Like search_route_PVS() in Edax
    fn route_pvs(&mut self, alpha: i32, beta: i32, depth: i32, node: Option<Arc<Node>>) -> i32 {
        let score = if depth == self.state.n_empties() {
            if depth == 0 {
                self.solve()
            } else {
                self.pvs_midgame(alpha, beta, depth, node)
            }
        } else if depth == 0 {
            self.state.eval_0()
        } else if depth == 1 {
            self.state.eval_1(alpha, beta)
        } else if depth == 2 {
            self.state.eval_2(alpha, beta)
        } else {
            self.pvs_midgame(alpha, beta, depth, node)
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
    ///
    /// Like PVS_midgame() in Edax
    fn pvs_midgame(&mut self, alpha: i32, beta: i32, depth: i32, parent: Option<Arc<Node>>) -> i32 {
        if !self.is_running() {
            return alpha;
        }

        if self.state.n_empties() == 0 {
            return self.state.eval_0();
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
            move_list.len() as i32,
            parent,
            self.state.height(),
        );
        node.set_pv_node(true);

        if move_list.is_empty() {
            if self.state.position().opponent_has_moves() {
                self.state.update_pass_midgame();
                node.set_best_score(self.route_pvs(-beta, -alpha, depth, Some(node.clone())));
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
            node.update(&move_, self);

            while let Some((index, move_)) = node.next_move() {
                if !node.split(&move_) {
                    let alpha = node.alpha();
                    self.state.update_midgame(&move_);
                    let mut score = -self.nws_midgame(-alpha - 1, depth - 1, Some(node.clone()));
                    if !self.is_running() && alpha < score && score < beta {
                        self.node_type[self.state.height() as usize] = NodeType::PvNode;
                        score = -self.pvs_midgame(-beta, -alpha, depth - 1, Some(node.clone()));
                    }
                    self.state.restore_midgame(&move_);

                    node.set_move_score(index, score);
                    node.update(&move_, self);
                }
            }

            node.wait_slaves();
        }

        if !self.is_running() {
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
                cost: cost.ilog2() as i32,
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

        if self.state.n_empties() <= depth && depth <= DEPTH_MIDGAME_TO_ENDGAME {
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
                0,
                parent,
                self.state.height(),
            );

            let move_ = if self.state.position().opponent_has_moves() {
                self.state.update_pass_midgame();
                let score = -self.nws_midgame(-node.beta(), depth, Some(node.clone()));
                self.state.restore_pass_midgame();

                Move::new_pass_with_score(score)
            } else {
                let score = self.solve();
                Move::new_no_move_with_score(score)
            };

            node.set_move_list(MoveList::new_one_move(move_));
        } else {
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
                move_list.len() as i32,
                parent,
                self.state.height(),
            );

            while let Some((index, move_)) = node.next_move() {
                if !node.split(&move_) {
                    self.state.update_midgame(&move_);
                    let score = -self.nws_midgame(-alpha - 1, depth - 1, Some(node.clone()));
                    self.state.restore_midgame(&move_);

                    node.set_move_score(index, score);
                    node.update(&move_, self);
                }
            }

            node.wait_slaves();
        };

        if !self.is_running() {
            cost += self.shared.n_nodes.load(Ordering::Relaxed) as i64
                + self.shared.child_nodes.load(Ordering::Relaxed) as i64;

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
                cost: cost.ilog2() as i32,
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

    /// Probcut search.
    ///
    /// Like search_probcut() in Edax
    fn probcut(&mut self, alpha: i32, depth: i32, parent: Option<Arc<Node>>) -> Option<i32> {
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
            let probcut_error = t * Eval::sigma(self.state.n_empties(), depth, probcut_depth) + RCD;

            // compute evaluation error (i.e. error at depth 0) averaged for both depths
            let eval_score = self.state.eval_0();
            let eval_error = t
                * 0.5
                * (Eval::sigma(self.state.n_empties(), depth, 0)
                    + Eval::sigma(self.state.n_empties(), depth, probcut_depth))
                + RCD;

            // try a probable upper cut first
            let eval_beta = beta - eval_error as i32;
            let probcut_beta = beta + probcut_error as i32;
            let probcut_alpha = probcut_beta - 1;
            if eval_score >= eval_beta && probcut_beta < SCORE_MAX {
                // check if trying a beta probcut is worth
                self.update_probcut(NodeType::CutNode);
                let score = self.nws_midgame(probcut_alpha, probcut_depth, parent.clone());
                self.restore_probcut(node_type, saved_selectivity);
                if score >= probcut_beta {
                    return Some(beta);
                }
            }

            // try a probable lower cut if upper cut failed
            let eval_alpha = alpha + eval_error as i32;
            let probcut_alpha = alpha - probcut_error as i32;
            if eval_score < eval_alpha && probcut_alpha > SCORE_MIN {
                // check if trying an alpha probcut is worth
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
    ///
    /// Like NWS_endgame() in Edax
    fn nws_endgame(&mut self, alpha: i32) -> i32 {
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
                self.state.update_pass_midgame(); // TODO should pass endgame here?
                let score = -self.nws_endgame(-beta);
                self.state.restore_pass_midgame();

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
                let score = -self.nws_endgame(-beta);
                self.state.restore_endgame(move_);

                if score > best_move.score.get() {
                    best_move = move_.clone();
                    if best_move.score.get() >= beta {
                        break;
                    }
                }
            }

            best_move
        };

        if !self.is_running() {
            cost += self.shared.n_nodes.load(Ordering::Relaxed) as i64;

            self.hash_table.store(&StoreArgs {
                position: self.state.position(),
                depth: self.state.n_empties(),
                selectivity: NO_SELECTIVITY,
                cost: cost.ilog2() as i32,
                alpha,
                beta,
                score: best_move.score.get(),
                move_: best_move.x,
            });

            return best_move.score.get();
        }

        alpha
    }

    /// Null Window Search to find exact score.
    ///
    /// Like search_shallow() in Edax
    fn endgame_shallow(&mut self, alpha: i32) -> i32 {
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
    ///
    /// Like search_solve_4() in Edax
    fn solve_4(&mut self, alpha: i32) -> i32 {
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
    ///
    /// Like search_solve_3() in Edax
    fn solve_3(&mut self, alpha: i32) -> i32 {
        let beta = alpha + 1;

        // TODO #15 further optimization: add dedicated function for taking 4 empties using unwrap_unchecked()
        let (mut x1, mut x2, mut x3) = {
            let mut iter = self.state.empties().iter();

            (
                iter.next().unwrap().x as usize,
                iter.next().unwrap().x as usize,
                iter.next().unwrap().x as usize,
            )
        };

        let parity = self.state.parity();

        // parity based move sorting
        if parity & QUADRANT_ID[x1] == 0 {
            if parity & QUADRANT_ID[x2] != 0 {
                (x1, x2) = (x2, x1); // case 1(x2) 2(x1 x3)
            } else {
                (x1, x2, x3) = (x3, x1, x2); // case 1(x3) 2(x1 x2)
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
            if NEIGHBOUR[x1] & self.state.position().player() != 0 {
                let mut next = *self.state.position();
                next.pass();
                best_score = -Self::solve_2(&next, alpha, x2, x3);
                if best_score <= alpha {
                    return best_score;
                }
            }

            if NEIGHBOUR[x2] & self.state.position().player() != 0 {
                let mut next = *self.state.position();
                next.pass();
                let score = -Self::solve_2(&next, alpha, x1, x3);
                if score <= alpha {
                    return score;
                } else if score < best_score {
                    best_score = score;
                }
            }

            if NEIGHBOUR[x3] & self.state.position().player() != 0 {
                let mut next = *self.state.position();
                next.pass();
                let score = -Self::solve_2(&next, alpha, x1, x2);
                if score < best_score {
                    best_score = score;
                }
            }

            if best_score == -SCORE_INF {
                best_score = self.state.position().final_score_with_empty(3);
            }
        }

        best_score
    }

    /// Compute score for a position with 2 empty squares.
    ///
    /// Like search_solve_2() in Edax
    fn solve_2(position: &Position, alpha: i32, x1: usize, x2: usize) -> i32 {
        let beta = alpha + 1;

        let mut best_score = -SCORE_INF;

        if NEIGHBOUR[x1] & position.opponent() != 0 {
            let (next, flipped) = Position::new_from_parent_and_move(position, x1);
            if flipped != 0 {
                best_score = -Self::solve_1(&next, beta, x2);
            }
        }

        if best_score < beta {
            if NEIGHBOUR[x2] & position.opponent() != 0 {
                let (next, flipped) = Position::new_from_parent_and_move(position, x2);
                if flipped != 0 {
                    let score = -Self::solve_1(&next, alpha, x1);
                    if score > best_score {
                        best_score = score;
                    }
                }
            }

            if best_score == -SCORE_INF {
                best_score = SCORE_INF;

                if NEIGHBOUR[x1] & position.player() != 0 {
                    let mut next = *position;
                    next.pass();
                    best_score = -Self::solve_1(&next, alpha, x2);
                }

                if best_score > alpha {
                    if NEIGHBOUR[x2] & position.player() != 0 {
                        let mut next = *position;
                        next.pass();
                        let score = -Self::solve_1(&next, alpha, x1);
                        if score < best_score {
                            best_score = score;
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
    ///
    /// Like search_solve_1() in Edax
    fn solve_1(position: &Position, beta: i32, x: usize) -> i32 {
        let mut score = 2 * position.opponent().count_ones() as i32 - SCORE_MAX;

        let mut n_flips = count_last_flip(x, position.player()) as i32;

        if n_flips != 0 {
            score -= n_flips;
        } else if score >= 0 {
            score += 2;
            if score < beta {
                n_flips = count_last_flip(x, position.opponent()) as i32;
                score += n_flips;
            }
        } else if score < beta {
            let n_flips = count_last_flip(x, position.opponent()) as i32;
            if n_flips != 0 {
                score += n_flips + 2;
            }
        }

        score
    }
}
