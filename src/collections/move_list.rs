use crate::{bot::edax::r#move::Move, othello::position::Position};

use super::hashtable::HashData;

use std::ops::Index;

/// A fixed-size collection of moves for a position.
///
/// `MoveList` maintains these invariants:
/// - After creation, no moves can be added or removed
/// - Only sorting and modifying move properties (score, cost) is allowed
///
/// Currently implemented as a wrapper around `Vec<Move>`.
///
/// # TODO #15 Further optimization: In the future, this may be
/// changed to a singly-linked list backed by a vector for better memory efficiency,
/// similar to Edax's implementation.
pub struct MoveList {
    inner: Vec<Move>,
}

// TODO #15 Further optimization: remove Clone or potentially only support it for tests.
impl Clone for MoveList {
    fn clone(&self) -> Self {
        MoveList {
            inner: self.inner.clone(),
        }
    }
}

impl MoveList {
    /// Like search_get_movelist() in Edax
    pub fn new(position: &Position) -> Self {
        let moves = position.iter_move_indices();

        // We know that the lower_bound on size_hint() is giving exact size for MoveIndices
        let size = moves.size_hint().0;

        let mut inner = Vec::with_capacity(size);
        inner.extend(moves.map(|x| Move::new(position, x as i32)));

        Self { inner }
    }

    pub fn new_empty() -> Self {
        Self { inner: vec![] }
    }

    pub fn new_one_move(move_: Move) -> Self {
        Self { inner: vec![move_] }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Move> {
        self.inner.iter()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Sorts moves by score in descending order.
    ///
    /// Like movelist_sort in Edax
    pub fn sort_by_score(&mut self) {
        self.inner.sort_by(|a, b| b.score.cmp(&a.score));
    }

    /// Like movelist_sort_cost in Edax
    pub fn sort_by_cost(&mut self, hash_data: &HashData) {
        for move_ in self.inner.iter() {
            if move_.x == hash_data.move_[0] as i32 {
                move_.cost.set(u32::MAX);
            } else if move_.x == hash_data.move_[1] as i32 {
                move_.cost.set(u32::MAX - 1);
            }
        }

        self.inner.sort_by(|a, b| b.cost.get().cmp(&a.cost.get()));
    }

    /// Put the move for a given square at the front.
    /// If no such move is found, do nothing.
    ///
    /// Like movelist_sort_bestmove in Edax
    pub fn set_first_move(&mut self, x: i32) {
        if let Some(pos) = self.inner.iter().position(|m| m.x == x) {
            self.inner.swap(0, pos);
        }
    }

    pub fn first(&self) -> Option<&Move> {
        self.inner.first()
    }

    pub fn set_score(&mut self, i: usize, score: i32) {
        self.inner[i].score.set(score);
    }

    pub fn set_score_and_cost(&mut self, i: usize, score: i32, cost: u32) {
        self.inner[i].score.set(score);
        self.inner[i].cost.set(cost);
    }
}

impl Index<usize> for MoveList {
    type Output = Move;

    fn index(&self, index: usize) -> &Self::Output {
        &self.inner[index]
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;
    use rand::Rng;

    fn validate_list(list: &MoveList, expected_items: &[i32]) {
        let items = list.iter().map(|m| m.x).collect::<Vec<_>>();
        assert_eq!(items, expected_items);
        assert_eq!(list.len(), expected_items.len());
    }

    // Construct a MoveList from an array. It is here because
    // we convert i32 to a Move. i32 is used to keep tests readable.
    fn pool_list_from_array(array: &[i32]) -> MoveList {
        let inner = array
            .iter()
            .map(|x| Move {
                x: *x,
                score: Cell::new(*x),
                flipped: 0,
                cost: Cell::new(0),
            })
            .collect::<Vec<_>>();

        MoveList { inner }
    }

    #[test]
    fn test_new_list_is_empty() {
        let list = pool_list_from_array(&[]);
        validate_list(&list, &[]);
    }

    #[test]
    fn test_empty_operations() {
        let list = pool_list_from_array(&[]);
        assert!(list.is_empty());

        let list = pool_list_from_array(&[1]);
        assert!(!list.is_empty());
    }

    #[test]
    fn test_sort_by_score() {
        let mut list = pool_list_from_array(&[4, 2, 1, 3]);

        validate_list(&list, &[4, 2, 1, 3]);

        assert_eq!(
            list.inner.iter().map(|n| n.x).collect::<Vec<_>>(),
            vec![4, 2, 1, 3]
        );

        list.sort_by_score();
        validate_list(&list, &[4, 3, 2, 1]);
    }

    #[test]
    fn test_sort_large_list() {
        for _ in 0..1000 {
            let array = (0..53)
                .map(|_| rand::thread_rng().gen::<i32>())
                .collect::<Vec<_>>();

            let mut list = pool_list_from_array(&array);

            list.sort_by_score();
            let items = list.iter().cloned().collect::<Vec<_>>();
            let mut sorted = items.clone();
            sorted.sort_by(|a, b| b.score.cmp(&a.score));
            assert_eq!(items, sorted);
            assert_eq!(list.len(), 53);
        }
    }

    #[test]
    fn test_first() {
        let list = pool_list_from_array(&[]);
        assert_eq!(list.first().map(|m| m.x), None);

        let list = pool_list_from_array(&[111]);
        assert_eq!(list.first().map(|m| m.x), Some(111));
    }

    #[test]
    fn test_clone() {
        let original = pool_list_from_array(&[1, 2, 3]);

        // Create clone and validate both lists
        let cloned = original.clone();
        validate_list(&original, &[1, 2, 3]);
        validate_list(&cloned, &[1, 2, 3]);

        // Drop original and verify cloned is still valid
        drop(original);
        validate_list(&cloned, &[1, 2, 3]);
    }

    #[test]
    fn test_from_iter_with_size() {
        let list = pool_list_from_array(&[]);
        validate_list(&list, &[]);

        let list = pool_list_from_array(&[0]);
        validate_list(&list, &[0]);

        let list = pool_list_from_array(&[0, 1]);
        validate_list(&list, &[0, 1]);

        let list = pool_list_from_array(&[0, 1, 2]);
        validate_list(&list, &[0, 1, 2]);

        let list = pool_list_from_array(&[0, 1, 2, 3]);
        validate_list(&list, &[0, 1, 2, 3]);
    }

    #[test]
    fn test_index() {
        let list = pool_list_from_array(&[10, 20, 30]);
        assert_eq!(list[0].x, 10);
        assert_eq!(list[1].x, 20);
        assert_eq!(list[2].x, 30);
    }

    #[test]
    #[should_panic]
    fn test_index_out_of_bounds() {
        let list = pool_list_from_array(&[1, 2, 3]);
        let _value = &list[3]; // This should panic
    }

    #[test]
    fn test_set_score() {
        let mut list = pool_list_from_array(&[1, 2, 3]);

        // Set a new score for the second element
        list.set_score(1, 100);

        // Verify the score was updated
        assert_eq!(list[1].score.get(), 100);

        // Verify other scores remain unchanged
        assert_eq!(list[0].score.get(), 1);
        assert_eq!(list[2].score.get(), 3);
    }

    #[test]
    fn test_new() {
        use crate::othello::position::Position;

        // Create a position with some legal moves
        let position = Position::new();
        let list = MoveList::new(&position);

        // The initial position should have 4 legal moves
        assert_eq!(list.len(), 4);

        // Verify that all moves are valid for the initial position
        // The moves should be at squares 19, 26, 37, and 44 (standard initial position moves)
        let moves: Vec<i32> = list.iter().map(|m| m.x).collect();
        assert!(moves.contains(&19));
        assert!(moves.contains(&26));
        assert!(moves.contains(&37));
        assert!(moves.contains(&44));
    }

    #[test]
    fn test_new_empty() {
        let list = MoveList::new_empty();
        assert!(list.is_empty());
        assert_eq!(list.len(), 0);
        assert_eq!(list.first(), None);
    }

    #[test]
    fn test_new_one_move() {
        let move_ = Move {
            x: 42,
            score: Cell::new(100),
            flipped: 0,
            cost: Cell::new(0),
        };

        let list = MoveList::new_one_move(move_);

        assert_eq!(list.len(), 1);
        assert!(!list.is_empty());
        assert_eq!(list[0].x, 42);
        assert_eq!(list[0].score.get(), 100);
        assert_eq!(list.first().unwrap().x, 42);
    }
}
