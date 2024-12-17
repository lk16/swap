use std::{ops::Index, ptr::NonNull};

use crate::bot::edax::r#const::QUADRANT_ID;

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

/// A node in the `EmptiesList`.
#[derive(Default, Clone)]
struct Node {
    // Data stored in this node
    data: Square,

    // Next node in list.
    next: Option<NonNull<Node>>,

    // Previous node in main list. Uninitialized when in free list.
    prev: Option<NonNull<Node>>,
}

/// A fixed-size doubly-linked list of Squares backed by a pre-allocated vector, optimized for
/// temporary removal and restoration operations.
///
/// This data structure combines the memory locality of a vector with the O(1) temporary removal
/// and restoration capabilities of a linked list. It is specifically designed for algorithms
/// that need to frequently remove and restore items, such as tree search algorithms.
///
/// # Capacity and Initialization
/// The list is initialized with a fixed set of Squares and cannot grow or shrink after creation.
/// The first entry in the underlying vector serves as a sentinel node for the circular list.
///
/// # Invariants
/// Once the list is created, the following invariants are maintained:
/// - No new squares can be added to or permanently removed from the list
/// - Only temporary removal and restoration of existing squares is possible
/// - Squares in the list cannot be modified
/// - Squares remain in their original memory location even when temporarily removed
///
/// # Performance
/// - Remove: O(1)
/// - Restore: O(1)
/// - Get: O(1)
/// - Iteration: O(n)
///
/// # Implementation Details
/// The list maintains a circular doubly-linked list containing the active elements, with a sentinel
/// node at index 0. When items are removed, they maintain their position in memory and their links,
/// allowing them to be efficiently restored later.
///
/// The list also maintains a mapping from Square coordinates to node indices for O(1) lookups.
///
/// # Methods
/// - `remove_by_x`: Temporarily removes a Square at the given x-coordinate
/// - `restore_by_x`: Restores a previously removed Square at the given x-coordinate
///
/// # Example Use Case
/// This is particularly useful in game tree search algorithms where you need to:
/// 1. Track available moves
/// 2. Temporarily remove moves when exploring a branch
/// 3. Restore moves when backtracking
/// 4. Efficiently look up moves by their coordinates
pub struct EmptiesList {
    /// Nodes in the list.
    nodes: Vec<Node>,

    /// Maps square to index in `nodes`.
    x_to_node: [usize; 64],
}

impl Clone for EmptiesList {
    fn clone(&self) -> Self {
        Self::from_iter_with_size(self.iter().cloned(), self.len())
    }
}

impl EmptiesList {
    /// Create a new `EmptiesList` from an iterator with a given size.
    /// This is the only way to create an `EmptiesList`.
    pub fn from_iter_with_size<I: IntoIterator<Item = Square>>(iter: I, size: usize) -> Self {
        let mut nodes = Vec::with_capacity(size + 1);

        let mut x_to_node = [0; 64];

        // Push sentinel node
        nodes.push(Node {
            data: Square::new(0),
            next: None,
            prev: None,
        });

        let base_ptr = nodes.as_mut_ptr();

        unsafe {
            // Push first node (sentinel)
            let mut prev = base_ptr;

            // Push nodes and set links
            for (i, square) in std::iter::zip(1..size + 1, iter) {
                x_to_node[square.x as usize] = i;

                let current_ptr = base_ptr.add(i);

                // Set up the new node
                let node = Node {
                    data: square,
                    next: Some(NonNull::new_unchecked(base_ptr.add(i + 1))),
                    prev: Some(NonNull::new_unchecked(prev)),
                };

                nodes.push(node);
                prev = current_ptr;
            }

            // Adjust last node
            let last = nodes.len() - 1;
            nodes[last].next = Some(NonNull::new_unchecked(base_ptr));

            // Set sentinel prev pointer to last node
            nodes[0].prev = Some(NonNull::new_unchecked(
                base_ptr.offset(nodes.len() as isize - 1),
            ));

            // Set sentinel next pointer to first node or self if list is empty.
            nodes[0].next = if nodes.len() == 1 {
                Some(NonNull::new_unchecked(base_ptr.offset(0)))
            } else {
                Some(NonNull::new_unchecked(base_ptr.offset(1)))
            };

            Self { nodes, x_to_node }
        }
    }

    /// Remove a node from the list.
    /// The node is selected by index in the underlying vector.
    /// The removed node never gets deallocated or changed. This allows for efficient restoration.
    fn remove(&mut self, index: usize) {
        let node = &mut self.nodes[index];

        let prev = node.prev.unwrap();
        let next = node.next.unwrap();

        // SAFETY: This operation is safe because:
        // 1. prev and next pointers are valid as they come from an active node in the list
        // 2. The node being removed is part of the main list (has valid prev/next)
        // 3. We maintain list invariants by properly updating neighboring nodes
        unsafe {
            // Update neighboring nodes to skip over this one
            (*prev.as_ptr()).next = Some(next);
            (*next.as_ptr()).prev = Some(prev);
        }
    }

    /// Restore a previously removed node to the list.
    /// The node is selected by index in the underlying vector.
    fn restore(&mut self, index: usize) {
        let node = &mut self.nodes[index];

        let prev = node.prev.unwrap();
        let next = node.next.unwrap();

        let node_ptr = NonNull::from(node);

        // SAFETY: This operation is safe because:
        // 1. prev and next pointers are valid as they were preserved when the node was removed
        // 2. The node being restored has maintained its original prev/next pointers
        // 3. We maintain list invariants by properly updating neighboring nodes
        unsafe {
            // Restore neighboring nodes to point to this node again
            (*prev.as_ptr()).next = Some(node_ptr);
            (*next.as_ptr()).prev = Some(node_ptr);
        }
    }

    /// Iterate over all nodes in the list.
    pub fn iter(&self) -> Iter {
        // SAFETY: This is safe because:
        // 1. We're only using the sentinel pointer for comparison in the iterator
        // 2. We're not actually mutating anything through this pointer
        // 3. The pointer remains valid for the lifetime of the iterator due to the PhantomData marker
        unsafe {
            Iter {
                next: self.nodes[0].next,
                sentinel: NonNull::new_unchecked(self.nodes.as_ptr() as *mut Node),
                _phantom: std::marker::PhantomData,
            }
        }
    }

    /// Iterate over all squares with an even number of empties in their quadrant.
    pub fn iter_even(&self, parity: u32) -> impl Iterator<Item = &Square> {
        self.iter().filter(move |&s| parity & s.quadrant == 0)
    }

    /// Iterate over all squares with an odd number of empties in their quadrant.
    pub fn iter_odd(&self, parity: u32) -> impl Iterator<Item = &Square> {
        self.iter().filter(move |&s| parity & s.quadrant != 0)
    }

    /// Remove a square from the list.
    /// The square is selected by its x-coordinate.
    pub fn remove_by_x(&mut self, x: i32) {
        let index = self.x_to_node[x as usize];
        self.remove(index); // TODO inline this call
    }

    /// Restore a previously removed square to the list.
    /// The square is selected by the its x-coordinate.
    pub fn restore_by_x(&mut self, x: i32) {
        let index = self.x_to_node[x as usize];
        self.restore(index); // TODO inline this call
    }

    /// Get the number of squares in the list.
    pub fn len(&self) -> usize {
        self.nodes.len() - 1
    }

    /// Check if the list is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// An iterator over all squares in the list.
pub struct Iter<'a> {
    next: Option<NonNull<Node>>,
    sentinel: NonNull<Node>,
    _phantom: std::marker::PhantomData<&'a Square>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a Square;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.next?;

        // Stop if we've reached the sentinel
        if current.as_ptr() == self.sentinel.as_ptr() {
            return None;
        }

        unsafe {
            // Get the next node
            self.next = (*current.as_ptr()).next;
            Some(&(*current.as_ptr()).data)
        }
    }
}

// TODO this is not used, remove.
impl Index<usize> for EmptiesList {
    type Output = Square;

    fn index(&self, index: usize) -> &Self::Output {
        &self.nodes[index + 1].data
    }
}

#[cfg(test)]
mod tests {
    use crate::bot::edax::eval::tests::test_positions;

    use super::*;

    fn validate_list(list: &EmptiesList, expected_items: &[i32]) {
        unsafe {
            let sentinel = NonNull::new_unchecked(list.nodes.as_ptr() as *mut Node);

            // Validate sentinel's circular links
            assert!(
                (*sentinel.as_ptr()).next.is_some(),
                "Sentinel should have next"
            );
            assert!(
                (*sentinel.as_ptr()).prev.is_some(),
                "Sentinel should have prev"
            );

            // Count nodes in main list and validate circular structure
            let mut current = (*sentinel.as_ptr()).next.unwrap();
            let mut prev = sentinel;
            let mut count = 0;

            // Traverse main list until we reach sentinel again
            while current != sentinel {
                // Validate bidirectional links
                assert_eq!(
                    (*current.as_ptr()).prev,
                    Some(prev),
                    "Node's prev should point to previous node"
                );
                assert!((*current.as_ptr()).next.is_some(), "Node should have next");
                assert_eq!(
                    (*(*current.as_ptr()).next.unwrap().as_ptr()).prev,
                    Some(current),
                    "Next node's prev should point back"
                );

                prev = current;
                current = (*current.as_ptr()).next.unwrap();
                count += 1;
            }

            assert_eq!(count, expected_items.len(), "List length mismatch");

            let items = list.iter().map(|x| x.x).collect::<Vec<_>>();
            assert_eq!(items, expected_items);
        }

        for (i, &x_to_node) in list.x_to_node.iter().enumerate() {
            let index = x_to_node;

            if index == 0 {
                continue;
            }

            assert_eq!(list.nodes[index].data.x, i as i32);
        }
    }

    fn from_int_array(arr: &[i32]) -> EmptiesList {
        let size = arr.len();
        EmptiesList::from_iter_with_size(arr.iter().map(|x| Square::new(*x as usize)), size)
    }

    #[test]
    fn test_from_iter_and_iterate() {
        let list = from_int_array(&[]);
        validate_list(&list, &[]);

        let list = from_int_array(&[11]);
        validate_list(&list, &[11]);

        let list = from_int_array(&[11, 22]);
        validate_list(&list, &[11, 22]);

        let list = from_int_array(&[11, 22, 33]);
        validate_list(&list, &[11, 22, 33]);

        let list = from_int_array(&[11, 22, 33, 44]);
        validate_list(&list, &[11, 22, 33, 44]);
    }

    #[test]
    fn test_remove_and_restore_first() {
        let mut list = from_int_array(&[11, 22, 33]);

        list.remove(1);
        validate_list(&list, &[22, 33]);

        list.restore(1);
        validate_list(&list, &[11, 22, 33]);
    }

    #[test]
    fn test_remove_and_restore_last() {
        let mut list = from_int_array(&[11, 22, 33]);

        list.remove(3);
        validate_list(&list, &[11, 22]);

        list.restore(3);
        validate_list(&list, &[11, 22, 33]);
    }

    #[test]
    fn test_remove_and_restore_middle() {
        let mut list = from_int_array(&[11, 22, 33]);

        list.remove(2);
        validate_list(&list, &[11, 33]);

        list.restore(2);
        validate_list(&list, &[11, 22, 33]);
    }

    #[test]
    fn test_remove_and_restore_many() {
        let mut list = from_int_array(&[11, 22, 33, 44]);

        validate_list(&list, &[11, 22, 33, 44]);

        list.remove(2); // Remove 22
        validate_list(&list, &[11, 33, 44]);

        list.remove(3); // Remove 33
        validate_list(&list, &[11, 44]);

        list.remove(1); // Remove 11
        validate_list(&list, &[44]);

        list.remove(4); // Remove 44
        validate_list(&list, &[]);

        list.restore(4); // Add 44
        validate_list(&list, &[44]);

        list.restore(1); // Add 11
        validate_list(&list, &[11, 44]);

        list.restore(3); // Add 33
        validate_list(&list, &[11, 33, 44]);

        list.restore(2); // Add 22
        validate_list(&list, &[11, 22, 33, 44]);
    }

    #[test]
    fn test_clone() {
        let original = from_int_array(&[11, 22]);

        // Validate original before clone
        validate_list(&original, &[11, 22]);

        // Clone and validate
        let mut cloned = original.clone();
        validate_list(&cloned, &[11, 22]);

        // Modify clone
        cloned.remove_by_x(11);

        // Validate modified clone
        validate_list(&cloned, &[22]);

        // Verify original is unchanged
        validate_list(&original, &[11, 22]);
    }

    #[test]
    fn test_remove_and_restore_by_x() {
        let mut list = from_int_array(&[11, 22, 33, 44]);

        list.remove_by_x(22);
        validate_list(&list, &[11, 33, 44]);

        list.remove_by_x(11);
        validate_list(&list, &[33, 44]);

        list.remove_by_x(44);
        validate_list(&list, &[33]);

        list.remove_by_x(33);
        validate_list(&list, &[]);

        list.restore_by_x(33);
        validate_list(&list, &[33]);

        list.restore_by_x(44);
        validate_list(&list, &[33, 44]);

        list.restore_by_x(11);
        validate_list(&list, &[11, 33, 44]);

        list.restore_by_x(22);
        validate_list(&list, &[11, 22, 33, 44]);
    }

    #[test]
    fn test_x_node_map() {
        let mut list = from_int_array(&[11, 22, 33, 44]);

        // Test that the x_to_node map is correct
        assert_eq!(list.x_to_node[11], 1);
        assert_eq!(list.x_to_node[22], 2);
        assert_eq!(list.x_to_node[33], 3);
        assert_eq!(list.x_to_node[44], 4);

        // Test missing x
        assert_eq!(list.x_to_node[55], 0);

        // Test that removing a node does not change the x_to_node map
        list.remove_by_x(22);
        assert_eq!(list.x_to_node[22], 2);

        // Test that restoring a node does not change the x_to_node map
        list.restore_by_x(22);
        assert_eq!(list.x_to_node[22], 2);
    }

    #[test]
    fn test_len_and_is_empty() {
        let list = from_int_array(&[]);
        assert_eq!(list.len(), 0);
        assert!(list.is_empty());

        let list = from_int_array(&[11]);
        assert_eq!(list.len(), 1);
        assert!(!list.is_empty());
    }

    #[test]
    fn test_iter_even_odd() {
        for position in test_positions() {
            let empties = (0..64)
                .filter(|&x| position.is_empty(x))
                .map(Square::new)
                .collect::<Vec<_>>();

            let list = EmptiesList::from_iter_with_size(
                empties.into_iter(),
                position.count_empty() as usize,
            );

            let parity = {
                let mut parity = 0;
                for square in list.iter() {
                    parity ^= square.quadrant;
                }
                parity
            };

            // Check that even squares have correct parity
            for square in list.iter_even(parity) {
                assert_eq!(
                    parity & square.quadrant,
                    0,
                    "Square {} with quadrant {} should be even for parity {}",
                    square.x,
                    square.quadrant,
                    parity
                );
            }

            // Check that odd squares have correct parity
            for square in list.iter_odd(parity) {
                assert_ne!(
                    parity & square.quadrant,
                    0,
                    "Square {} with quadrant {} should be odd for parity {}",
                    square.x,
                    square.quadrant,
                    parity
                );
            }

            // Collect all squares from both iterators
            let mut combined_squares: Vec<_> = list
                .iter_even(parity)
                .chain(list.iter_odd(parity))
                .collect();
            combined_squares.sort_by_key(|s| s.x);

            // Collect all squares from the main iterator
            let mut all_squares: Vec<_> = list.iter().collect();
            all_squares.sort_by_key(|s| s.x);

            // Verify that the combined iterators contain the same elements as the main iterator
            assert_eq!(
                combined_squares, all_squares,
                "Combined even and odd iterators should contain all squares"
            );
        }
    }

    #[test]
    fn test_index() {
        let list = from_int_array(&[11, 22, 33, 44]);

        // Test valid indices
        assert_eq!(list[0].x, 11);
        assert_eq!(list[1].x, 22);
        assert_eq!(list[2].x, 33);
        assert_eq!(list[3].x, 44);
    }

    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn test_index_panic() {
        let list = from_int_array(&[11, 22, 33, 44]);
        let _should_panic = &list[4]; // This should panic as it's out of bounds
    }
}
