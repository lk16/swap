use std::ptr::NonNull;

#[derive(Default, Clone)]
struct Node<T> {
    // Data stored in this node
    data: T,

    // Next node in list
    next: Option<NonNull<Node<T>>>,
}

/// A fixed-capacity singly-linked list backed by a pre-allocated vector.
///
/// Similar to PoolList, but maintains only forward links, making it more memory
/// efficient. The list is initialized from an iterator and supports operations
/// like sorting and iteration.
///
/// # Capacity
/// The list has a fixed capacity `N` specified at compile time. It will not resize,
/// and attempting to initialize with more items than the capacity will panic.
pub struct ForwardPoolList<T: Default + PartialOrd, const N: usize> {
    // Underlying vector of nodes
    nodes: Vec<Node<T>>,

    // Head of the list
    head: Option<NonNull<Node<T>>>,

    // Number of items in the list
    length: usize,
}

impl<T, const N: usize> Clone for ForwardPoolList<T, N>
where
    T: Clone + Default + PartialOrd,
{
    fn clone(&self) -> Self {
        // Create a new list with cloned nodes data
        let mut nodes = self.nodes.clone();

        // Recreate the links using pointer arithmetic
        let base_ptr = nodes.as_mut_ptr();

        unsafe {
            // Update all next pointers in the nodes
            for node in nodes.iter_mut() {
                node.next = match node.next {
                    Some(ptr) => {
                        // Calculate offset from original base to get new index
                        let offset = ptr.as_ptr().offset_from(self.nodes.as_ptr()) as usize;
                        Some(NonNull::new_unchecked(base_ptr.add(offset)))
                    }
                    None => None,
                };
            }

            // Update head pointer
            let head = self.head.map(|ptr| {
                let offset = ptr.as_ptr().offset_from(self.nodes.as_ptr()) as usize;
                NonNull::new_unchecked(base_ptr.add(offset))
            });

            Self {
                nodes,
                head,
                length: self.length,
            }
        }
    }
}

impl<T: Default + PartialOrd, const N: usize> ForwardPoolList<T, N> {
    /// Creates a new `ForwardPoolList` from an iterator with a known size.
    ///
    /// This is the primary way to construct a `ForwardPoolList`. The list will be initialized
    /// with the elements from the iterator in the order they appear.
    ///
    /// # Arguments
    /// * `iter` - An iterator that yields the elements to store in the list
    /// * `size` - The exact number of elements that will be taken from the iterator
    ///
    /// # Panics
    /// * If `size` exceeds the list's capacity `N`
    /// * If the iterator yields fewer elements than specified by `size`
    pub fn from_iter_with_size<I: IntoIterator<Item = T>>(iter: I, size: usize) -> Self {
        // TODO remove N and just allocate the vector with the size

        // TODO #15 further optimization: size should never be too large, but this check may be slow.
        assert!(size <= N, "Iterator size exceeds list capacity");

        // TODO #15 further optimization: size should never be zero, but the check for it may be slow.
        if size == 0 {
            return Self {
                nodes: Vec::new(),
                head: None,
                length: 0,
            };
        }

        let mut iter = iter.into_iter();

        // TODO #15 further optimization: find out if the allocation itself is the bottleneck.
        // If so, consider allocating on the stack with a fixed size.

        // Allocate the vector with the correct capacity
        let mut nodes: Vec<Node<T>> = Vec::with_capacity(N);

        // SAFETY: The following unsafe block maintains these invariants:
        // 1. All pointers are derived from nodes.as_mut_ptr() which is valid for the lifetime of nodes
        // 2. The vector is never resized after creation
        // 3. Each node points to a valid next node within the allocated memory
        // 4. The last node's next pointer is set to None
        // 5. All pointer arithmetic is bounds-checked via the size parameter
        // 6. We initialize all nodes up to 'size' with valid data
        // 7. The remaining nodes (size..N) remain uninitialized but are never accessed

        #[allow(clippy::uninit_vec)]
        unsafe {
            nodes.set_len(N);

            let base = NonNull::new_unchecked(nodes.as_mut_ptr());

            // Setup main list
            for i in 0..size {
                let node_ptr = base.as_ptr().add(i);

                let node = Node {
                    data: iter.next().unwrap(),
                    next: Some(NonNull::new_unchecked(node_ptr.add(1))),
                };

                // Write directly to memory
                std::ptr::write(node_ptr, node);
            }

            let last_node = base.as_ptr().add(size - 1);
            (*last_node).next = None;

            // Setup head
            let head = Some(NonNull::new_unchecked(base.as_ptr()));

            Self {
                nodes,
                head,
                length: size,
            }
        }
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            next: self.head,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut {
            next: self.head,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Returns the number of elements in the list.
    ///
    /// This is an O(1) operation.
    pub fn len(&self) -> usize {
        self.length
    }

    /// Returns true if the list is empty.
    ///
    /// This is an O(1) operation.
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Sorts the list in descending order using the >= operator.
    /// Uses an in-place merge sort algorithm.
    pub fn sort(&mut self) {
        if self.length <= 1 {
            return;
        }

        unsafe {
            self.head = Some(Self::merge_sort(self.head.unwrap()));
        }
    }

    pub fn first(&self) -> Option<&T> {
        self.head.map(|node| unsafe { &(*node.as_ptr()).data })
    }

    pub fn first_mut(&mut self) -> Option<&mut T> {
        self.head.map(|node| unsafe { &mut (*node.as_ptr()).data })
    }

    unsafe fn merge_sort(head: NonNull<Node<T>>) -> NonNull<Node<T>> {
        // List has only one node
        if (*head.as_ptr()).next.is_none() {
            return head;
        }

        let (first, second) = Self::split(head);
        let first = Self::merge_sort(first);
        let second = Self::merge_sort(second);
        Self::merge(first, second)
    }

    unsafe fn split(head: NonNull<Node<T>>) -> (NonNull<Node<T>>, NonNull<Node<T>>) {
        let mut slow = head;
        let mut fast = head;
        let mut prev = head;

        // Fast pointer moves twice as fast as slow pointer
        // When fast reaches end, slow will be at middle
        while let Some(next_fast) = (*fast.as_ptr()).next {
            fast = next_fast;

            // Move fast pointer again if possible
            if let Some(next_fast) = (*fast.as_ptr()).next {
                fast = next_fast;
                prev = slow;
                slow = (*slow.as_ptr()).next.unwrap();
            } else {
                // If we can't move fast twice, we're at the end
                prev = slow;
                slow = (*slow.as_ptr()).next.unwrap();
                break;
            }
        }

        // Split the list by setting prev.next to None
        (*prev.as_ptr()).next = None;

        (head, slow)
    }

    unsafe fn merge(mut left: NonNull<Node<T>>, mut right: NonNull<Node<T>>) -> NonNull<Node<T>> {
        // Create a dummy head node to simplify the merging process
        let mut dummy = Node::default();
        let mut tail = &mut dummy;

        // Compare and merge nodes until one list is exhausted
        loop {
            if (*left.as_ptr()).data >= (*right.as_ptr()).data {
                // Left node is greater or equal, add it to result
                let next = (*left.as_ptr()).next;
                tail.next = Some(left);
                tail = tail.next.unwrap().as_mut();
                left = match next {
                    Some(node) => node,
                    None => {
                        tail.next = Some(right);
                        return dummy.next.unwrap();
                    }
                };
            } else {
                // Right node is greater, add it to result
                let next = (*right.as_ptr()).next;
                tail.next = Some(right);
                tail = tail.next.unwrap().as_mut();
                right = match next {
                    Some(node) => node,
                    None => {
                        tail.next = Some(left);
                        return dummy.next.unwrap();
                    }
                };
            }
        }
    }
}

pub struct Iter<'a, T> {
    next: Option<NonNull<Node<T>>>,
    _phantom: std::marker::PhantomData<&'a T>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.next?;

        unsafe {
            self.next = (*current.as_ptr()).next;
            Some(&(*current.as_ptr()).data)
        }
    }
}

pub struct IterMut<'a, T> {
    next: Option<NonNull<Node<T>>>,
    _phantom: std::marker::PhantomData<&'a mut T>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.next?;

        unsafe {
            self.next = (*current.as_ptr()).next;
            Some(&mut (*current.as_ptr()).data)
        }
    }
}

/// Implements `Send` for `ForwardPoolList`, allowing it to be transferred across thread boundaries.
///
/// # Safety
/// This implementation is safe because:
/// - All internal pointers (`NonNull`) are derived from the `nodes` Vec, which never changes location
/// - The Vec is never resized after creation
/// - All access to shared data is properly synchronized through mutable/shared references
/// - The underlying type T must be both Send and Sync
unsafe impl<T: Default + PartialOrd + Send + Sync, const N: usize> Send for ForwardPoolList<T, N> {}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    fn validate_list(
        list: &ForwardPoolList<i32, 4>,
        expected_items: &[i32],
        _expected_free_count: usize, // TODO remove argument
    ) {
        let items = list.iter().copied().collect::<Vec<_>>();
        assert_eq!(items, expected_items);
        assert_eq!(list.len(), expected_items.len());
    }

    // Construct a ForwardPoolList from an array. It is here because:
    // - the only way to construct a ForwardPoolList is from an iterator
    // - the function name is long and this is a shorthand
    // - the size should be correct, we handle that here
    fn pool_list_from_array(array: &[i32]) -> ForwardPoolList<i32, 4> {
        let size = array.len();
        ForwardPoolList::<i32, 4>::from_iter_with_size(array.iter().copied(), size)
    }

    // Construct a ForwardPoolList from a larger array.
    // Used to test sorting.
    fn pool_list_from_large_array(array: &[i32]) -> ForwardPoolList<i32, 53> {
        let size = array.len();
        ForwardPoolList::<i32, 53>::from_iter_with_size(array.iter().copied(), size)
    }

    #[test]
    fn test_new_list_is_empty() {
        let list = pool_list_from_array(&[]);
        validate_list(&list, &[], 4);
    }

    #[test]
    fn test_empty_operations() {
        let list = pool_list_from_array(&[]);
        assert!(list.is_empty());

        let list = pool_list_from_array(&[1]);
        assert!(!list.is_empty());
    }

    #[test]
    fn test_iter_mut() {
        let mut list = pool_list_from_array(&[1, 2, 3]);

        // Multiply each element by 2
        for x in list.iter_mut() {
            *x *= 2;
        }

        validate_list(&list, &[2, 4, 6], 1);
    }

    #[test]
    fn test_sort() {
        let mut list = pool_list_from_array(&[4, 2, 1, 3]);

        validate_list(&list, &[4, 2, 1, 3], 0);

        assert_eq!(
            list.nodes.iter().map(|n| n.data).collect::<Vec<_>>(),
            vec![4, 2, 1, 3]
        );

        list.sort();
        validate_list(&list, &[4, 3, 2, 1], 0);

        // Nodes should be in the same order in the underlying vector
        assert_eq!(
            list.nodes.iter().map(|n| n.data).collect::<Vec<_>>(),
            vec![4, 2, 1, 3]
        );
    }

    #[test]
    fn test_sort_large_list() {
        for _ in 0..1000 {
            let array = (0..53)
                .map(|_| rand::thread_rng().gen::<i32>())
                .collect::<Vec<_>>();

            let mut list = pool_list_from_large_array(&array);

            list.sort();
            let items = list.iter().cloned().collect::<Vec<_>>();
            let mut sorted = items.clone();
            sorted.sort_by(|a, b| b.cmp(a));
            assert_eq!(items, sorted);
            assert_eq!(list.len(), 53);
        }
    }

    #[test]
    fn test_first() {
        let list = pool_list_from_array(&[]);
        assert_eq!(list.first(), None);

        let list = pool_list_from_array(&[111]);
        assert_eq!(list.first(), Some(&111));
    }

    #[test]
    fn test_first_mut() {
        let mut list = pool_list_from_array(&[]);
        assert_eq!(list.first_mut(), None);

        let mut list = pool_list_from_array(&[111]);
        assert_eq!(list.first_mut(), Some(&mut 111));

        // Modify value through mutable reference
        if let Some(value) = list.first_mut() {
            *value = 999;
        }
        assert_eq!(list.first(), Some(&999));
    }

    #[test]
    fn test_clone() {
        let mut original = pool_list_from_array(&[1, 2, 3]);

        // Create clone and validate both lists
        let mut cloned = original.clone();
        validate_list(&original, &[1, 2, 3], 1);
        validate_list(&cloned, &[1, 2, 3], 1);

        // Modify original and verify clone is unchanged
        *(original.first_mut().unwrap()) = 4;
        validate_list(&original, &[4, 2, 3], 1);
        validate_list(&cloned, &[1, 2, 3], 1);

        // Verify clone can be modified independently
        *(cloned.first_mut().unwrap()) = 5;
        validate_list(&original, &[4, 2, 3], 1);
        validate_list(&cloned, &[5, 2, 3], 1);

        // Drop original and verify cloned is still valid
        drop(original);
        validate_list(&cloned, &[5, 2, 3], 1);
    }

    #[test]
    fn test_from_iter_with_size() {
        let list = ForwardPoolList::from_iter_with_size(0..0, 0);
        validate_list(&list, &[], 4);

        let list = ForwardPoolList::from_iter_with_size(0..1, 1);
        validate_list(&list, &[0], 3);

        let list = ForwardPoolList::from_iter_with_size(0..2, 2);
        validate_list(&list, &[0, 1], 2);

        let list = ForwardPoolList::from_iter_with_size(0..3, 3);
        validate_list(&list, &[0, 1, 2], 1);

        let list = ForwardPoolList::from_iter_with_size(0..4, 4);
        validate_list(&list, &[0, 1, 2, 3], 0);
    }
}
