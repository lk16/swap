use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::RwLock;

use crate::bot::edax::r#const::{SCORE_INF, SCORE_MAX, SCORE_MIN};
use crate::othello::position::Position;
use crate::othello::squares::NO_MOVE;

/// A fixed-size bucket for storing hash table entries
const BUCKET_SIZE: usize = 4;

/// Represents cached evaluation data for an Othello position
///
/// Stores evaluation scores, move history, and metadata for a position.
/// Like HashData in Edax.
#[derive(Clone, Copy)]
pub struct HashData {
    /// Depth of the search
    pub depth: u8,

    /// Selectivity level of the search
    pub selectivity: u8,

    /// Log2 of the cost of the search
    pub cost: u8,

    /// Date of the search, updated on every access.
    /// Used for replacement strategy. We want to keep the most recently accessed entries.
    pub date: u8,

    /// Lower bound of the score found by the search
    pub lower: i8,

    /// Upper bound of the score found by the search
    pub upper: i8,

    /// First and second best move found by the search
    pub move_: [u8; 2],
}

impl Default for HashData {
    /// Like HASH_DATA_INIT in Edax
    fn default() -> Self {
        Self {
            depth: 0,
            selectivity: 0,
            cost: 0,
            date: 0,
            lower: -SCORE_INF as i8,
            upper: SCORE_INF as i8,
            move_: [NO_MOVE as u8, NO_MOVE as u8],
        }
    }
}

impl HashData {
    /// Create a new `HashData` entry.
    ///
    /// Like data_new() in Edax
    fn new(date: u8, args: &StoreArgs) -> Self {
        let score = args.score as i8;
        let beta = args.beta as i8;
        let alpha = args.alpha as i8;
        let move_ = args.move_ as u8;
        let depth = args.depth as u8;
        let selectivity = args.selectivity as u8;
        let cost = args.cost as u8;

        Self {
            upper: if score < beta { score } else { SCORE_MAX as i8 },
            lower: if score > alpha {
                score
            } else {
                SCORE_MIN as i8
            },
            move_: [
                if score > alpha || score == SCORE_MIN as i8 {
                    move_
                } else {
                    NO_MOVE as u8
                },
                NO_MOVE as u8,
            ],
            depth,
            selectivity,
            cost,
            date,
        }
    }

    /// Calculate a priority level for replacement strategy based on the entry's metadata.
    ///
    /// Like writable_level() in Edax
    pub fn writable_level(&self) -> u32 {
        u32::from_le_bytes([self.depth, self.selectivity, self.cost, self.date])
    }

    /// Update the entry with new evaluation data.
    ///
    /// This is done when the level is the same as the previous storage.
    /// Best moves and bound scores are updated, other data are untouched.
    ///
    /// Like data_update() in Edax
    fn update(&mut self, args: &StoreArgs) {
        let score = args.score as i8;
        let move_ = args.move_ as u8;
        let alpha = args.alpha as i8;
        let beta = args.beta as i8;
        let cost = args.cost as u8;

        // Update upper bound if score is better (lower)
        if score < beta && score < self.upper {
            self.upper = score;
        }

        // Update lower bound if score is better (higher)
        if score > alpha && score > self.lower {
            self.lower = score;
        }

        // Update best moves if score beats alpha or is the minimum score
        if (score > alpha || score == SCORE_MIN as i8) && self.move_[0] != move_ {
            self.move_[1] = self.move_[0];
            self.move_[0] = move_;
        }

        // Update cost to maximum of current and new cost
        self.cost = cost.max(self.cost);
    }

    /// Upgrade the entry with new evaluation data.
    ///
    /// Upgrade is done when the search level increases.
    /// Best moves are updated, others data are reset to new value.
    ///
    /// Like data_upgrade() in Edax
    fn upgrade(&mut self, args: &StoreArgs) {
        let score = args.score as i8;
        let beta = args.beta as i8;
        let alpha = args.alpha as i8;
        let move_ = args.move_ as u8;
        let depth = args.depth as u8;
        let selectivity = args.selectivity as u8;
        let cost = args.cost as u8;

        // Update upper bound based on score vs beta
        self.upper = if score < beta { score } else { SCORE_MAX as i8 };

        // Update lower bound based on score vs alpha
        self.lower = if score > alpha {
            score
        } else {
            SCORE_MIN as i8
        };

        // Update move history if score beats alpha or is the minimum score
        if (score > alpha || score == SCORE_MIN as i8) && self.move_[0] != move_ {
            self.move_[1] = self.move_[0];
            self.move_[0] = move_;
        }

        // Update metadata
        self.depth = depth;
        self.selectivity = selectivity;
        self.cost = cost.max(self.cost);
    }
}

/// A single entry in the hash table.
///
/// Like Hash in Edax
#[derive(Default, Clone, Copy)]
struct Entry {
    /// The position being searched
    position: Position,

    /// The cached evaluation data
    hash_data: HashData,
}

impl Entry {
    /// Create a new `Entry`.
    ///
    /// Like hash_new() in Edax
    fn new(date: u8, args: &StoreArgs) -> Self {
        Self {
            position: *args.position,
            hash_data: HashData::new(date, args),
        }
    }

    /// Update the entry with new evaluation data.
    ///
    /// Returns whether the entry was updated.
    ///
    /// Like hash_update() in Edax
    fn update(&mut self, date: u8, args: &StoreArgs) -> bool {
        if self.position != *args.position {
            return false;
        }

        if self.hash_data.selectivity == args.selectivity as u8
            && self.hash_data.depth == args.depth as u8
        {
            self.hash_data.update(args);
        } else {
            self.hash_data.upgrade(args);
        }
        self.hash_data.date = date;

        if self.hash_data.lower > self.hash_data.upper {
            self.hash_data = HashData::new(date, args);
        }

        true
    }
}

type Bucket = [Entry; BUCKET_SIZE];

/// Arguments for storing position data in the hash table
pub struct StoreArgs<'a> {
    /// The position being searched
    pub position: &'a Position,

    /// Depth of the search
    pub depth: i32,

    /// Selectivity level
    pub selectivity: i32,

    /// Log2 of the cost of the search
    pub cost: i32,

    /// Lower bound for alpha-beta search
    pub alpha: i32,

    /// Upper bound for alpha-beta search
    pub beta: i32,

    /// Evaluation score for the position
    pub score: i32,

    /// Best move found so far
    pub move_: i32,
}

/// Thread-safe hash table implementation for storing position evaluations
///
/// Implements a fixed-size hash table with bucket-based collision handling.
/// Each bucket contains BUCKET_SIZE entries that are protected by RwLocks
/// for concurrent access.
pub struct HashTable {
    /// Buckets for storing entries
    buckets: Box<[RwLock<Bucket>]>,

    /// Mask for indexing into buckets
    mask: usize,

    /// Atomic counter for date updates
    date: AtomicU8,
}

impl HashTable {
    /// Creates a new hash table with the specified size (rounded up to next power of 2).
    pub fn new(size: usize) -> Self {
        // Round up to power of 2
        let size = size.next_power_of_two();
        let mask = size - 1;

        // Create buckets
        let buckets = (0..size)
            .map(|_| RwLock::new(Bucket::default()))
            .collect::<Vec<_>>()
            .into_boxed_slice();

        HashTable {
            buckets,
            mask,
            date: AtomicU8::new(0),
        }
    }

    /// Calculate the bucket index for a given position.
    fn get_bucket_index(&self, position: &Position) -> usize {
        let mut hasher = DefaultHasher::new();
        position.hash(&mut hasher);
        hasher.finish() as usize & self.mask
    }

    /// Stores a position evaluation in the hash table, replacing existing or least valuable entries if necessary
    ///
    /// Like hash_store() in Edax
    pub fn store(&self, args: &StoreArgs) {
        let bucket_idx = self.get_bucket_index(args.position);
        let mut bucket = self.buckets[bucket_idx].write().unwrap();
        let date = self.date.load(Ordering::Relaxed);

        // Try to update an existing entry first
        for entry in bucket.iter_mut() {
            if entry.update(date, args) {
                return;
            }
        }

        let entry = bucket
            .iter_mut()
            .min_by_key(|entry| entry.hash_data.writable_level())
            .unwrap();

        *entry = Entry::new(date, args);
    }

    /// Retrieves a HashData for a given position, or None if the position is not cached
    pub fn get(&self, position: &Position) -> Option<HashData> {
        let bucket_idx = self.get_bucket_index(position);
        let bucket = &self.buckets[bucket_idx];

        // Find the entry and update date with write lock
        let mut entries = bucket.write().unwrap();
        for entry in entries.iter_mut() {
            if entry.position == *position {
                entry.hash_data.date = self.date.load(Ordering::Relaxed);
                return Some(entry.hash_data);
            }
        }

        None
    }

    /// Get the cached evaluation data for a position, or default if not found.
    pub fn get_or_default(&self, position: &Position) -> HashData {
        self.get(position).unwrap_or_default()
    }

    /// Completely clears the hash table by resetting all entries
    ///
    /// Like hash_cleanup() in Edax.
    pub fn clear(&self) {
        for bucket in self.buckets.iter() {
            let mut entries = bucket.write().unwrap();
            *entries = [Entry::default(); BUCKET_SIZE];
        }
    }

    /// Performs an optimized clear operation using a date-based strategy
    ///
    /// This method uses a date counter to track entry freshness:
    /// - Increments internal date counter
    /// - Keeps existing entries accessible
    /// - Updates entry dates on access
    /// - Performs full clear when date reaches 255
    ///
    /// Like hash_clear() in Edax
    pub fn soft_clear(&self) {
        let current_date = self.date.load(Ordering::Relaxed);
        if current_date == 255 {
            // Reset all entries
            self.clear();
            self.date.store(1, Ordering::Relaxed);
        } else {
            self.date.fetch_add(1, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl<'a> StoreArgs<'a> {
        fn from_pos_and_depth(position: &'a Position, depth: i32) -> Self {
            Self {
                position,
                depth,
                selectivity: 1,
                cost: 1,
                alpha: 0,
                beta: 0,
                score: 0,
                move_: 0,
            }
        }
    }

    #[test]
    fn test_new() {
        let table = HashTable::new(100);
        assert_eq!(table.mask, 127); // Should round up to 128 (next power of 2) - 1
        assert_eq!(table.buckets.len(), 128);
        assert_eq!(table.date.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_store_and_get() {
        let table = HashTable::new(16);
        let pos = Position::new();

        // Store the data
        table.store(&StoreArgs::from_pos_and_depth(&pos, 5));

        // Retrieve and verify
        let hash_data = table.get(&pos).expect("Should find stored position");
        assert_eq!(hash_data.depth, 5);
    }

    #[test]
    fn test_store_update_existing() {
        let table = HashTable::new(16);
        let pos = Position::new();

        table.store(&StoreArgs::from_pos_and_depth(&pos, 5));
        table.store(&StoreArgs::from_pos_and_depth(&pos, 10));

        let hash_data = table.get(&pos).expect("Should find stored position");
        assert_eq!(hash_data.depth, 10);
    }

    #[test]
    fn test_bucket_overflow() {
        let table = HashTable::new(1);

        // Fill a bucket completely
        for i in 0..BUCKET_SIZE {
            let pos = Position::new_random_with_discs(32);
            table.store(&StoreArgs::from_pos_and_depth(&pos, i as i32));
        }

        // Add one more - should replace the worst entry (lowest depth in this case)
        let new_pos = Position::new_random_with_discs(32);
        table.store(&StoreArgs::from_pos_and_depth(&new_pos, 42));

        // Verify the new entry exists
        let result = table.get(&new_pos);
        assert!(result.is_some());
    }

    #[test]
    fn test_clear() {
        let table = HashTable::new(16);
        let pos = Position::new();

        // Store and verify initial state
        table.store(&StoreArgs::from_pos_and_depth(&pos, 5));
        assert!(table.get(&pos).is_some());

        // Test date increment clear
        table.soft_clear();
        assert_eq!(table.date.load(Ordering::Relaxed), 1);
        assert!(table.get(&pos).is_some());

        // Test full clear when date reaches 255
        for _ in 0..254 {
            table.soft_clear();
        }
        assert_eq!(table.date.load(Ordering::Relaxed), 255);
        table.soft_clear(); // This should trigger full clear
        assert_eq!(table.date.load(Ordering::Relaxed), 1);
        assert!(table.get(&pos).is_none());
    }

    #[test]
    fn test_date_update() {
        let table = HashTable::new(16);
        let pos = Position::new();

        table.store(&StoreArgs::from_pos_and_depth(&pos, 5));
        table.date.store(42, Ordering::Relaxed);

        // Verify date gets updated on get
        let hash_data = table.get(&pos).expect("Should find stored position");
        assert_eq!(hash_data.date, 42);
    }

    #[test]
    fn test_hash_data_writable_level() {
        let data = HashData {
            depth: 5,
            selectivity: 2,
            cost: 3,
            date: 4,
            lower: 0,
            upper: 0,
            move_: [0, 0],
        };

        // level() combines fields as: (date << 24) + (cost << 16) + (selectivity << 8) + depth
        let expected = (4_u32 << 24) + (3_u32 << 16) + (2_u32 << 8) + 5_u32;
        assert_eq!(data.writable_level(), expected);
    }

    #[test]
    fn test_cleanup() {
        let table = HashTable::new(16);
        let pos = Position::new();

        // Store some data
        table.store(&StoreArgs::from_pos_and_depth(&pos, 5));
        assert!(table.get(&pos).is_some());

        // Cleanup should remove all entries
        table.clear();
        assert!(table.get(&pos).is_none());
    }

    #[test]
    fn test_concurrent_access() {
        use std::sync::Arc;
        use std::thread;

        let table = Arc::new(HashTable::new(16));
        let pos = Position::new();

        // Store initial data
        table.store(&StoreArgs::from_pos_and_depth(&pos, 5));

        // Spawn multiple threads to read/write
        let mut handles = vec![];
        for _ in 0..4 {
            let table_clone = Arc::clone(&table);
            let pos_clone = pos;

            handles.push(thread::spawn(move || {
                // Read operation
                if let Some(hash_data) = table_clone.get(&pos_clone) {
                    assert_eq!(hash_data.depth, 5);
                }
            }));
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_hash_data_new() {
        let pos = Position::new();
        let args = StoreArgs {
            position: &pos,
            depth: 5,
            selectivity: 2,
            cost: 3,
            alpha: -10,
            beta: 10,
            score: 5,
            move_: 42,
        };
        let data = HashData::new(1, &args);

        assert_eq!(data.depth, 5);
        assert_eq!(data.selectivity, 2);
        assert_eq!(data.cost, 3);
        assert_eq!(data.date, 1);
        assert_eq!(data.move_[0], 42);
    }

    #[test]
    fn test_hash_data_update() {
        let mut data = HashData::default();
        let pos = Position::new();
        let args = StoreArgs {
            position: &pos,
            depth: 5,
            selectivity: 2,
            cost: 3,
            alpha: -10,
            beta: 10,
            score: 5,
            move_: 42,
        };

        data.update(&args);
        // Add assertions for updated fields
    }

    #[test]
    fn test_get_or_default() {
        let table = HashTable::new(16);
        let pos = Position::new();

        // Test default case
        let data = table.get_or_default(&pos);
        assert_eq!(data.depth, 0);
        assert_eq!(data.selectivity, 0);

        // Test after storing
        table.store(&StoreArgs::from_pos_and_depth(&pos, 5));
        let data = table.get_or_default(&pos);
        assert_eq!(data.depth, 5);
    }

    #[test]
    fn test_store_hash_data_update() {
        let table = HashTable::new(16);
        let pos = Position::new();

        // Initial store
        let args = StoreArgs {
            position: &pos,
            depth: 5,
            selectivity: 2,
            cost: 3,
            alpha: -10,
            beta: 10,
            score: 0,
            move_: 42,
        };
        table.store(&args);

        // Update with same depth and selectivity but different score/move
        let update_args = StoreArgs {
            position: &pos,
            depth: 5,       // Same depth
            selectivity: 2, // Same selectivity
            cost: 4,
            alpha: -10,
            beta: 10,
            score: 5,
            move_: 43,
        };
        table.store(&update_args);

        let hash_data = table.get(&pos).expect("Should find stored position");
        assert_eq!(hash_data.depth, 5); // Should remain unchanged
        assert_eq!(hash_data.selectivity, 2); // Should remain unchanged
        assert_eq!(hash_data.cost, 4); // Should take max of old and new
        assert_eq!(hash_data.move_, [43, NO_MOVE as u8]); // Should update to new move
    }

    #[test]
    fn test_store_hash_data_upgrade() {
        let table = HashTable::new(16);
        let pos = Position::new();

        // Initial store
        let args = StoreArgs {
            position: &pos,
            depth: 5,
            selectivity: 2,
            cost: 3,
            alpha: -10,
            beta: 10,
            score: 0,
            move_: 42,
        };
        table.store(&args);

        // Upgrade with higher depth
        let upgrade_args = StoreArgs {
            position: &pos,
            depth: 8,       // Higher depth triggers upgrade
            selectivity: 3, // Different selectivity
            cost: 4,
            alpha: -10,
            beta: 10,
            score: 5,
            move_: 43,
        };
        table.store(&upgrade_args);

        let hash_data = table.get(&pos).expect("Should find stored position");
        assert_eq!(hash_data.depth, 8); // Should upgrade to new depth
        assert_eq!(hash_data.selectivity, 3); // Should upgrade to new selectivity
        assert_eq!(hash_data.cost, 4); // Should take max of old and new
        assert_eq!(hash_data.move_, [43, 42]); // Should prepend the new move
    }
}
