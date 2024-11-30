use std::ptr::NonNull;

#[derive(Default, Clone)]
struct Node<T> {
    // Data stored in this node
    data: T,

    // Next node in list.
    next: Option<NonNull<Node<T>>>,

    // Previous node in main list. Uninitialized when in free list.
    prev: Option<NonNull<Node<T>>>,
}

/// A fixed-capacity doubly-linked list backed by a pre-allocated vector, optimized for
/// tree search operations.
///
/// This data structure combines the memory locality of a vector with the O(1) insertion
/// and removal capabilities of a linked list. It is specifically designed for algorithms
/// that need to frequently remove and restore items in a stack-like manner, such as
/// tree search algorithms.
///
/// # Capacity
/// The list has a fixed capacity `N` specified at compile time. It will not resize,
/// and attempting to push more items than the capacity will panic with "No free nodes available".
///
/// # Performance
/// - Push: O(1)
/// - Remove: O(1)
/// - Restore: O(1)
/// - Get: O(1)
/// - Iteration: O(n)
///
/// # Implementation Details
/// The list maintains two internal structures:
/// - A main circular doubly-linked list containing the active elements, with a sentinel node
/// - A singly-linked free list for managing unused nodes
///
/// The sentinel node is always present at index 0 and serves as both the head and tail
/// of the circular list.
///
/// When items are removed, they maintain their position in memory and their links,
/// allowing them to be efficiently restored later in a last-in-first-out manner.
///
/// # Methods
/// - `push`: Adds an item to the end of the list and returns its index
/// - `remove`: Removes an item at the given index
/// - `restore`: Restores a previously removed item at the given index
/// - `get`: Returns a reference to the item at the given index (panics if out of bounds)
///
/// # Example Use Case
/// This is particularly useful in tree search algorithms where you need to:
/// 1. Add moves to a path
/// 2. Remove moves when backtracking
/// 3. Restore moves when returning to a previously explored branch
/// 4. Access moves by their indices
pub struct PoolList<T: Default, const N: usize> {
    nodes: Vec<Node<T>>,

    // Start and end of main list
    sentinel: NonNull<Node<T>>,

    // Pointer to first free node. `prev` links are not initialized.
    free: Option<NonNull<Node<T>>>,
}

impl<T: Default, const N: usize> Default for PoolList<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> Clone for PoolList<T, N>
where
    T: Clone + Default,
{
    fn clone(&self) -> Self {
        // Create a new empty list first to ensure proper initialization
        let mut new_list = Self::new();

        // Clone the data from the original list
        for item in self.iter() {
            new_list.push(item.clone());
        }

        new_list
    }
}

impl<T: Default, const N: usize> PoolList<T, N> {
    pub fn new() -> Self {
        // SAFETY: This implementation is safe because:
        // 1. All pointers are derived from nodes Vec, which lives as long as PoolList
        // 2. The Vec is never resized or moved after creation
        // 3. The linked list structure forms a valid circular doubly-linked list:
        //    - Sentinel node points to itself when empty
        //    - All nodes maintain valid prev/next pointers in the main list
        //    - Free list maintains a separate chain of unused nodes (prev pointers intentionally uninitialized)
        // 4. Node indices are always within bounds [0..N+1)
        let mut nodes: Vec<Node<T>> = Vec::from_iter((0..N + 1).map(|_| Node::default()));

        unsafe {
            // Get pointer to sentinel node
            let sentinel = NonNull::new_unchecked(nodes.as_mut_ptr());

            // Setup sentinel to point to itself (empty list)
            (*sentinel.as_ptr()).next = Some(sentinel);
            (*sentinel.as_ptr()).prev = Some(sentinel);

            // Get pointer to first free node
            let free = NonNull::new_unchecked(sentinel.as_ptr().add(1));

            // Setup list of free nodes
            for i in 0..N {
                let node = free.as_ptr().add(i);
                (*node).next = Some(NonNull::new_unchecked(free.as_ptr().add(i + 1)));
            }

            // Setup free list end
            (*free.as_ptr().add(N - 1)).next = None;

            Self {
                nodes,
                sentinel,
                free: Some(free),
            }
        }
    }

    pub fn push(&mut self, data: T) -> usize {
        // Get the first free node
        let node = self.free.expect("No free nodes available");

        unsafe {
            // Update free list head to next free node
            self.free = (*node.as_ptr()).next;

            // Setup the new node's data
            (*node.as_ptr()).data = data;

            // Insert before sentinel (at end of list)
            let before_sentinel = (*self.sentinel.as_ptr()).prev.unwrap();

            // Update new node's links
            (*node.as_ptr()).next = Some(self.sentinel);
            (*node.as_ptr()).prev = Some(before_sentinel);

            // Update surrounding nodes
            (*before_sentinel.as_ptr()).next = Some(node);
            (*self.sentinel.as_ptr()).prev = Some(node);

            // Calculate index by pointer arithmetic
            node.as_ptr().offset_from(self.nodes.as_ptr()) as usize
        }
    }

    pub fn remove(&mut self, index: usize) {
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

    pub fn restore(&mut self, index: usize) {
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

    pub fn iter(&self) -> Iter<'_, T> {
        unsafe {
            Iter {
                next: (*self.sentinel.as_ptr()).next,
                sentinel: self.sentinel,
                _phantom: std::marker::PhantomData,
            }
        }
    }

    pub fn get(&self, index: usize) -> &T {
        &self.nodes[index].data
    }
}

pub struct Iter<'a, T> {
    next: Option<NonNull<Node<T>>>,
    sentinel: NonNull<Node<T>>,
    _phantom: std::marker::PhantomData<&'a T>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

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

#[cfg(test)]
mod tests {
    use super::*;

    fn validate_list(list: &PoolList<i32, 4>, expected_items: &[i32], expected_free_count: usize) {
        unsafe {
            // Validate sentinel's circular links
            assert!(
                (*list.sentinel.as_ptr()).next.is_some(),
                "Sentinel should have next"
            );
            assert!(
                (*list.sentinel.as_ptr()).prev.is_some(),
                "Sentinel should have prev"
            );

            // Count nodes in main list and validate circular structure
            let mut current = (*list.sentinel.as_ptr()).next.unwrap();
            let mut prev = list.sentinel;
            let mut count = 0;

            // Traverse main list until we reach sentinel again
            while current != list.sentinel {
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

            // Count nodes in free list
            let mut free_count = 0;
            let mut current = list.free;

            while let Some(ptr) = current {
                current = (*ptr.as_ptr()).next;
                free_count += 1;
            }

            let items = list.iter().copied().collect::<Vec<_>>();
            assert_eq!(items, expected_items);
            assert_eq!(free_count, expected_free_count);
        }
    }

    #[test]
    fn test_new_list_is_empty() {
        let list = PoolList::<i32, 4>::new();
        validate_list(&list, &[], 4);
    }

    #[test]
    fn test_push_and_iterate() {
        let mut list = PoolList::<i32, 4>::new();

        let index = list.push(111);
        assert_eq!(index, 1);
        validate_list(&list, &[111], 3);

        let index = list.push(222);
        assert_eq!(index, 2);
        validate_list(&list, &[111, 222], 2);

        let index = list.push(333);
        assert_eq!(index, 3);
        validate_list(&list, &[111, 222, 333], 1);

        let index = list.push(444);
        assert_eq!(index, 4);
        validate_list(&list, &[111, 222, 333, 444], 0);
    }

    #[test]
    fn test_remove_and_restore_first() {
        let mut list = PoolList::<i32, 4>::new();
        list.push(111);
        list.push(222);
        list.push(333);

        list.remove(1);
        validate_list(&list, &[222, 333], 1);

        list.restore(1);
        validate_list(&list, &[111, 222, 333], 1);
    }

    #[test]
    fn test_remove_and_restore_last() {
        let mut list = PoolList::<i32, 4>::new();
        list.push(111);
        list.push(222);
        list.push(333);

        list.remove(3);
        validate_list(&list, &[111, 222], 1);

        list.restore(3);
        validate_list(&list, &[111, 222, 333], 1);
    }

    #[test]
    fn test_remove_and_restore_middle() {
        let mut list = PoolList::<i32, 4>::new();
        list.push(111);
        list.push(222);
        list.push(333);

        list.remove(2);
        validate_list(&list, &[111, 333], 1);

        list.restore(2);
        validate_list(&list, &[111, 222, 333], 1);
    }

    #[test]
    fn test_remove_and_restore_many() {
        let mut list = PoolList::<i32, 4>::new();
        list.push(111);
        list.push(222);
        list.push(333);
        list.push(444);

        validate_list(&list, &[111, 222, 333, 444], 0);

        list.remove(2); // Remove 222
        validate_list(&list, &[111, 333, 444], 0);

        list.remove(3); // Remove 333
        validate_list(&list, &[111, 444], 0);

        list.remove(1); // Remove 111
        validate_list(&list, &[444], 0);

        list.remove(4); // Remove 444
        validate_list(&list, &[], 0);

        list.restore(4); // Add 444
        validate_list(&list, &[444], 0);

        list.restore(1); // Add 111
        validate_list(&list, &[111, 444], 0);

        list.restore(3); // Add 333
        validate_list(&list, &[111, 333, 444], 0);

        list.restore(2); // Add 222
        validate_list(&list, &[111, 222, 333, 444], 0);
    }

    #[test]
    fn test_get() {
        let mut list = PoolList::<i32, 4>::new();
        let idx1 = list.push(111);
        let idx2 = list.push(222);
        let idx3 = list.push(333);

        assert_eq!(*list.get(idx1), 111);
        assert_eq!(*list.get(idx2), 222);
        assert_eq!(*list.get(idx3), 333);
    }

    #[test]
    fn test_clone() {
        let mut original = PoolList::<i32, 4>::new();
        original.push(111);
        original.push(222);

        // Validate original before clone
        validate_list(&original, &[111, 222], 2);

        // Clone and validate
        let mut cloned = original.clone();
        validate_list(&cloned, &[111, 222], 2);

        // Modify clone
        cloned.push(333);
        cloned.push(444);

        // Validate modified clone
        validate_list(&cloned, &[111, 222, 333, 444], 0);

        // Verify original is unchanged
        validate_list(&original, &[111, 222], 2);
    }
}
