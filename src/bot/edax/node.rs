use std::sync::{atomic::Ordering, Arc, Mutex};

use crate::{
    collections::move_list::{Move, MoveList},
    othello::squares::NO_MOVE,
};

use super::{
    r#const::Stop,
    search::{Search, Shared},
};

/// Contents of Node.
struct NodeInner {
    /// Upper bound on the score of the best move.
    alpha: i32,

    /// Lower bound on the score of the best move.
    beta: i32,

    /// Depth of the node in the search tree.
    depth: i32,

    /// Height of the node in the search tree.
    height: i32,

    /// Whether this node is a PV node.
    pv_node: bool,

    /// Number of moves left to evaluate.
    n_moves_todo: i32,

    /// Number of moves already evaluated.
    n_moves_done: i32,

    /// Pointer to the parent node.
    #[allow(unused)] // TODO #8 Concurrent search: use this field
    parent: Option<Arc<Node>>,

    /// Shared search state.
    search: Arc<Shared>,

    /// Index of the best move.
    best_move: i32,

    /// Score of the best move.
    best_score: i32,

    /// List of moves to evaluate.
    /// Edax just stores a pointer to a move list kept elsewhere.
    /// We own the entire move list for simplicity and to obey borrowing rules.
    move_list: MoveList,
}

/// Maintains thread-safe details about a position while doing concurrent tree search.
///
/// Like Node in Edax.
pub struct Node {
    // TODO #8 Concurrent search: consider using RwLock instead of Mutex
    inner: Mutex<NodeInner>,
}

impl Node {
    /// Create a new node.
    ///
    /// Like node_init() in Edax, except we send `height` as extra argument
    pub fn new(
        search: Arc<Shared>,
        alpha: i32,
        beta: i32,
        depth: i32,
        n_moves: i32,
        parent: Option<Arc<Node>>,
        height: i32,
    ) -> Arc<Self> {
        let inner = NodeInner {
            alpha,
            beta,
            depth,
            height,
            pv_node: false,
            n_moves_todo: n_moves,
            n_moves_done: 0,
            parent,
            search,
            best_move: NO_MOVE as i32,
            best_score: 0,
            move_list: MoveList::new_empty(),
        };

        Arc::new(Self {
            inner: Mutex::new(inner),
        })
    }

    /// Like node_update() in Edax, but we pass `search`,
    /// because we need to modify fields that are not in `self.shared`.
    pub fn update(&self, move_: &Move, search: &mut Search) {
        let mut inner = self.inner.lock().unwrap();

        let score = move_.score.get();

        if inner.search.stop.load(Ordering::Relaxed) == Stop::Running as u8
            && score > inner.best_score
        {
            inner.best_score = score;
            inner.best_move = move_.x;

            if inner.height == 0 {
                search.record_best_move(
                    search.state.position(),
                    move_,
                    inner.alpha,
                    inner.beta,
                    inner.depth,
                );

                search.result.lock().unwrap().n_moves_left -= 1;
            }

            inner.alpha = inner.alpha.max(score);
        }

        if inner.alpha >= inner.beta {
            // TODO #8 concurrent search: stop slaves
        }
    }

    /// Try to start a new thread to search children.
    /// Returns true if we started a new thread.
    ///
    /// Like node_split() in Edax
    pub fn split(&self, _move_: &Move) -> bool {
        // TODO #8 Concurrent search: heart of the YBWC algorithm
        false
    }

    /// Wait for slave threads to finish.
    ///
    /// Like node_wait_slaves() in Edax
    pub fn wait_slaves(&self) {
        todo!() // TODO #8 Concurrent search: wait for slaves to finish
    }

    /// Set whether this node is a PV node.
    pub fn set_pv_node(&self, pv_node: bool) {
        let mut inner = self.inner.lock().unwrap();
        inner.pv_node = pv_node;
    }

    /// Get alpha
    pub fn alpha(&self) -> i32 {
        let inner = self.inner.lock().unwrap();
        inner.alpha
    }

    /// Get beta
    pub fn beta(&self) -> i32 {
        let inner = self.inner.lock().unwrap();
        inner.beta
    }

    /// Set best score
    pub fn set_best_score(&self, score: i32) {
        let mut inner = self.inner.lock().unwrap();
        inner.best_score = score;
    }

    /// Get best move
    pub fn best_move(&self) -> i32 {
        let inner = self.inner.lock().unwrap();
        inner.best_move
    }

    /// Get best score
    pub fn best_score(&self) -> i32 {
        let inner = self.inner.lock().unwrap();
        inner.best_score
    }

    /// Set alpha
    pub fn set_alpha(&self, alpha: i32) {
        let mut inner = self.inner.lock().unwrap();
        inner.alpha = alpha;
    }

    /// Set beta
    pub fn set_beta(&self, beta: i32) {
        let mut inner = self.inner.lock().unwrap();
        inner.beta = beta;
    }

    /// Set best move
    pub fn set_best_move(&self, move_: i32) {
        let mut inner = self.inner.lock().unwrap();
        inner.best_move = move_;
    }

    /// Set the move list and prepare for iteration.
    pub fn set_move_list(&self, move_list: MoveList) {
        let mut inner = self.inner.lock().unwrap();
        inner.n_moves_todo = move_list.len() as i32;
        inner.n_moves_done = 0;
        inner.move_list = move_list;
    }

    /// Get the next move, based on `self.inner.n_moves_todo`.
    /// We also return the index, so we can update the score later by index.
    ///
    /// Like node_next_move in Edax
    pub fn next_move(&self) -> Option<(usize, Move)> {
        // TODO #15 further optimization: ideally we would send a reference here so we can update in place.
        // However, Move is not Send, so we cannot do that right now.

        let mut inner = self.inner.lock().unwrap();

        if inner.n_moves_todo == 0 {
            return None;
        }

        let i = inner.n_moves_done;
        inner.n_moves_done += 1;
        inner.n_moves_todo -= 1;

        let index = i as usize;
        let move_ = inner.move_list[index].clone();

        Some((index, move_))
    }

    /// Set the score of a move by index in move_list.
    pub fn set_move_score(&self, index: usize, score: i32) {
        let mut inner = self.inner.lock().unwrap();
        inner.move_list.set_score(index, score);
    }

    /// Set the score and cost of a move by index in move_list.
    pub fn set_move_score_and_cost(&self, index: usize, score: i32, cost: u32) {
        let mut inner = self.inner.lock().unwrap();
        inner.move_list.set_score_and_cost(index, score, cost);
    }
}

#[cfg(test)]
mod tests {
    use crate::othello::position::Position;

    use super::*;

    #[test]
    fn test_next_move() {
        let position = Position::new();
        let move_list = MoveList::new(&position);

        let search = Search::new(&position, 0, 0);
        let node = Node::new(search.shared, 0, 0, 0, 0, None, 0);

        node.set_move_list(move_list.clone());

        assert_eq!(node.next_move(), Some((0, move_list[0].clone())));
        assert_eq!(node.next_move(), Some((1, move_list[1].clone())));
        assert_eq!(node.next_move(), Some((2, move_list[2].clone())));
        assert_eq!(node.next_move(), Some((3, move_list[3].clone())));
        assert_eq!(node.next_move(), None);
    }

    #[test]
    fn test_set_move_score() {
        let position = Position::new();
        let move_list = MoveList::new(&position);

        let search = Search::new(&position, 0, 0);
        let node = Node::new(search.shared, 0, 0, 0, 0, None, 0);

        node.set_move_list(move_list.clone());

        node.set_move_score(0, 33);
        assert_eq!(node.inner.lock().unwrap().move_list[0].score.get(), 33);
    }

    // TODO #8: add more tests once we use this with concurrent search
}
