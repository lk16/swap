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
/// efficient but without the ability to restore nodes in arbitrary order.
///
/// # Capacity
/// The list has a fixed capacity `N` specified at compile time. It will not resize,
/// and attempting to push more items than the capacity will panic.
pub struct ForwardPoolList<T: Default + PartialOrd, const N: usize> {
    nodes: Vec<Node<T>>,

    // Head of main list
    head: Option<NonNull<Node<T>>>,

    // Head of free list
    free: Option<NonNull<Node<T>>>,

    // Number of items in the main list
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

            // Update free list pointer
            let free = self.free.map(|ptr| {
                let offset = ptr.as_ptr().offset_from(self.nodes.as_ptr()) as usize;
                NonNull::new_unchecked(base_ptr.add(offset))
            });

            Self {
                nodes,
                head,
                free,
                length: self.length,
            }
        }
    }
}

impl<T: Default + PartialOrd, const N: usize> Default for ForwardPoolList<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Default + PartialOrd, const N: usize> ForwardPoolList<T, N> {
    pub fn new() -> Self {
        // SAFETY: This implementation is safe because:
        // 1. All pointers are derived from nodes Vec, which lives as long as ForwardPoolList
        // 2. The Vec is never resized or moved after creation
        // 3. Node indices are always within bounds [0..N)
        let mut nodes: Vec<Node<T>> = Vec::from_iter((0..N).map(|_| Node::default()));

        unsafe {
            // Get pointer to first free node
            let free = NonNull::new_unchecked(nodes.as_mut_ptr());

            // Setup list of free nodes
            for i in 0..N - 1 {
                let node = free.as_ptr().add(i);
                (*node).next = Some(NonNull::new_unchecked(free.as_ptr().add(i + 1)));
            }

            // Setup free list end
            (*free.as_ptr().add(N - 1)).next = None;

            Self {
                nodes,
                head: None,
                free: Some(free),
                length: 0,
            }
        }
    }

    pub fn push(&mut self, data: T) -> usize {
        // Get the first free node
        let node = self.free.expect("No free nodes available");

        unsafe {
            // Update free list head to next free node
            self.free = (*node.as_ptr()).next;

            // Setup the new node's data and link
            (*node.as_ptr()).data = data;
            (*node.as_ptr()).next = self.head;

            // Update head to point to new node
            self.head = Some(node);
            self.length += 1;

            // Calculate index by pointer arithmetic
            node.as_ptr().offset_from(self.nodes.as_ptr()) as usize
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        let node = self.head?;

        unsafe {
            // Update head to next node
            self.head = (*node.as_ptr()).next;
            self.length -= 1;

            // Add node back to free list
            (*node.as_ptr()).next = self.free;
            self.free = Some(node);

            // Return the data
            Some(std::mem::take(&mut (*node.as_ptr()).data))
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
        // TODO add tests for this
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
        expected_free_count: usize,
    ) {
        // Count nodes in free list
        let mut free_count = 0;
        let mut current = list.free;

        while let Some(ptr) = current {
            current = unsafe { (*ptr.as_ptr()).next };
            free_count += 1;
        }

        let items = list.iter().copied().collect::<Vec<_>>();
        assert_eq!(items, expected_items);
        assert_eq!(free_count, expected_free_count);
        assert_eq!(list.len(), expected_items.len());
    }

    #[test]
    fn test_new_list_is_empty() {
        let list = ForwardPoolList::<i32, 4>::new();
        validate_list(&list, &[], 4);
    }

    #[test]
    fn test_push_and_iterate() {
        let mut list = ForwardPoolList::<i32, 4>::new();

        let index = list.push(111);
        assert_eq!(index, 0);
        validate_list(&list, &[111], 3);

        let index = list.push(222);
        assert_eq!(index, 1);
        validate_list(&list, &[222, 111], 2);

        let index = list.push(333);
        assert_eq!(index, 2);
        validate_list(&list, &[333, 222, 111], 1);
    }

    #[test]
    fn test_push_and_pop() {
        let mut list = ForwardPoolList::<i32, 4>::new();

        list.push(111);
        list.push(222);
        list.push(333);
        validate_list(&list, &[333, 222, 111], 1);

        assert_eq!(list.pop(), Some(333));
        validate_list(&list, &[222, 111], 2);

        assert_eq!(list.pop(), Some(222));
        validate_list(&list, &[111], 3);

        assert_eq!(list.pop(), Some(111));
        validate_list(&list, &[], 4);
    }

    #[test]
    fn test_empty_operations() {
        let mut list = ForwardPoolList::<i32, 4>::new();
        assert!(list.is_empty());

        list.push(1);
        assert!(!list.is_empty());

        list.pop();
        assert!(list.is_empty());
    }

    #[test]
    fn test_iter_mut() {
        let mut list = ForwardPoolList::<i32, 4>::new();
        list.push(1);
        list.push(2);
        list.push(3);

        // Multiply each element by 2
        for x in list.iter_mut() {
            *x *= 2;
        }

        validate_list(&list, &[6, 4, 2], 1);
    }

    #[test]
    fn test_sort() {
        let mut list = ForwardPoolList::<i32, 4>::new();
        list.push(4);
        list.push(2);
        list.push(1);
        list.push(3);

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
            let mut list = ForwardPoolList::<i32, 53>::new();

            for _ in 0..53 {
                list.push(rand::thread_rng().gen::<i32>());
            }

            list.sort();
            let items = list.iter().collect::<Vec<_>>();
            let mut sorted = items.clone();
            sorted.sort_by(|a, b| b.cmp(a));
            assert_eq!(items, sorted);
            assert_eq!(list.len(), 53);
        }
    }

    #[test]
    fn test_first() {
        let mut list = ForwardPoolList::<i32, 4>::new();
        assert_eq!(list.first(), None);

        list.push(111);
        assert_eq!(list.first(), Some(&111));

        list.push(222);
        assert_eq!(list.first(), Some(&222));

        list.pop();
        assert_eq!(list.first(), Some(&111));

        list.pop();
        assert_eq!(list.first(), None);
    }

    #[test]
    fn test_clone() {
        let mut original = ForwardPoolList::<i32, 4>::new();
        original.push(1);
        original.push(2);
        original.push(3);

        // Create clone and validate both lists
        let cloned = original.clone();
        validate_list(&original, &[3, 2, 1], 1);
        validate_list(&cloned, &[3, 2, 1], 1);

        // Modify original and verify clone is unchanged
        original.push(4);
        validate_list(&original, &[4, 3, 2, 1], 0);
        validate_list(&cloned, &[3, 2, 1], 1);

        // Verify clone can be modified independently
        let mut cloned = cloned;
        cloned.pop();
        validate_list(&original, &[4, 3, 2, 1], 0);
        validate_list(&cloned, &[2, 1], 2);

        // Drop original and verify cloned is still valid
        drop(original);
        validate_list(&cloned, &[2, 1], 2);
        assert_eq!(cloned.pop(), Some(2));
        assert_eq!(cloned.pop(), Some(1));
        assert_eq!(cloned.pop(), None);
    }
}
