use lazy_static::lazy_static;
use rand::{rngs::ThreadRng, RngCore};
use serde_json::Value;
use std::fmt::{self, Display};
use std::fs::File;
use std::io::Read;
use std::path::Path;

use super::board::BLACK;
use super::count_stable::{count_stable, get_stable_edge};
use super::get_flipped::get_flipped;
use super::get_moves;

lazy_static! {
    /// XOT positions, loaded from json file when accessed the first time.
    pub static ref XOT_POSITIONS: Vec<Position> = {
        let path = Path::new("assets/xot.json");
        let mut file = File::open(path).expect("Failed to open XOT file");
        let mut json_str = String::new();
        file.read_to_string(&mut json_str)
            .expect("Failed to read XOT file");

        let json: Value = serde_json::from_str(&json_str).expect("Failed to parse JSON");

        fn parse_position(v: &Value) -> Position {
            let player = v["player"].as_str().unwrap().trim_start_matches("0x");
            let opponent = v["opponent"].as_str().unwrap().trim_start_matches("0x");
            Position {
                player: u64::from_str_radix(player, 16).unwrap(),
                opponent: u64::from_str_radix(opponent, 16).unwrap(),
            }
        }

        json.as_array()
            .expect("JSON is not an array")
            .iter()
            .map(parse_position)
            .collect()
    };
}

/// Print a bitset as an ASCII art board. Useful for debugging.
pub fn print_bitset(bitset: u64) {
    let mut output = String::new();
    output.push_str("+-A-B-C-D-E-F-G-H-+\n");
    for row in 0..8 {
        output.push_str(&format!("{} ", row + 1));
        for col in 0..8 {
            let index = row * 8 + col;
            let mask = 1u64 << index;
            if bitset & mask != 0 {
                output.push_str("● ");
            } else {
                output.push_str("  ");
            }
        }
        output.push_str(&format!("{}\n", row + 1));
    }
    output.push_str("+-A-B-C-D-E-F-G-H-+\n");
    println!("{}", output);
}

/// A position in the game of Othello.
/// This does not keep track of the player to move.
/// It is intended to be used by bots for tree search.
#[derive(Clone, Copy, Eq, PartialEq, Debug, Hash)]
pub struct Position {
    /// Bitboard of discs of the player to move.
    pub player: u64,

    /// Bitboard of discs of the opponent.
    pub opponent: u64,
}

impl Default for Position {
    fn default() -> Self {
        Self::new_empty()
    }
}

impl Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.ascii_art(BLACK))
    }
}

impl Position {
    /// Create a new starting position.
    pub fn new() -> Self {
        Self {
            player: 0x00000000810000000,
            opponent: 0x00000001008000000,
        }
    }

    /// Create a new random XOT position.
    pub fn new_xot() -> Self {
        let n = ThreadRng::default().next_u64() as usize;
        XOT_POSITIONS[n % XOT_POSITIONS.len()]
    }

    /// Create a new position from two bitboards.
    pub fn new_from_bitboards(player: u64, opponent: u64) -> Self {
        Self { player, opponent }
    }

    /// Create a new empty position.
    pub fn new_empty() -> Self {
        Self {
            player: 0,
            opponent: 0,
        }
    }

    /// Create a new random position with a given number of discs.
    pub fn new_random_with_discs(n_discs: usize) -> Self {
        assert!(
            (4..=64).contains(&n_discs),
            "Number of discs must be between 4 and 64"
        );

        let mut rng = ThreadRng::default();
        let mut position = Self::new();

        while position.count_discs() < n_discs as u32 {
            let moves = position.get_moves();
            if moves == 0 {
                position.pass();
                if position.get_moves() == 0 {
                    // No moves available for either player, start over
                    position = Self::new();
                }
                continue;
            }

            // Convert moves bitset to vector of indices
            let valid_moves: Vec<usize> = (0..64).filter(|&i| moves & (1u64 << i) != 0).collect();

            // Select random move
            let random_move = valid_moves[rng.next_u64() as usize % valid_moves.len()];
            position.do_move(random_move);
        }

        position
    }

    /// Create a new position by applying a move to a position.
    /// This is potentially more efficient than calling `do_move` because it
    /// avoids the swap of the player and opponent bitboards.
    pub fn new_from_parent_and_move(parent: &Position, move_index: usize) -> (Self, u64) {
        let flipped = parent.get_flipped(move_index);
        let player = parent.opponent ^ flipped;

        let position = Self {
            opponent: parent.player ^ (flipped | (1u64 << move_index)),
            player,
        };

        (position, flipped)
    }

    /// Shift a bitset in a given direction.
    pub fn shift(bitset: u64, dir: i32) -> u64 {
        // TODO move this function and its tests into get_moves.rs
        match dir {
            -9 => (bitset & 0xfefefefefefefefe) << 7,
            -8 => bitset << 8,
            -7 => (bitset & 0x7f7f7f7f7f7f7f7f) << 9,
            -1 => (bitset & 0xfefefefefefefefe) >> 1,
            1 => (bitset & 0x7f7f7f7f7f7f7f7f) << 1,
            7 => (bitset & 0x7f7f7f7f7f7f7f7f) >> 7,
            8 => bitset >> 8,
            9 => (bitset & 0xfefefefefefefefe) >> 9,
            _ => panic!("Invalid direction"),
        }
    }

    /// Compute bitset of valid moves for the player to move.
    pub fn get_moves(&self) -> u64 {
        get_moves::get_moves(self.player, self.opponent)
    }

    /// Compute bitset of valid moves for the opponent.
    pub fn get_opponent_moves(&self) -> u64 {
        get_moves::get_moves(self.opponent, self.player)
    }

    /// Check if there are any moves for the player to move.
    pub fn has_moves(&self) -> bool {
        self.get_moves() != 0
    }

    /// Check if there are any moves for the opponent.
    pub fn opponent_has_moves(&self) -> bool {
        self.get_opponent_moves() != 0
    }

    /// Check if a move is valid.
    pub fn is_valid_move(&self, index: usize) -> bool {
        if index >= 64 {
            return false;
        }

        self.get_moves() & (1u64 << index) != 0
    }

    pub fn pass(&mut self) {
        std::mem::swap(&mut self.player, &mut self.opponent);
    }

    /// Apply a move to the position.
    pub fn do_move(&mut self, index: usize) -> u64 {
        let flipped = self.get_flipped(index);

        self.player |= flipped | (1u64 << index);
        self.opponent ^= flipped;

        std::mem::swap(&mut self.player, &mut self.opponent);

        flipped
    }

    /// Get bitset of flipped discs for a move.
    pub fn get_flipped(&self, index: usize) -> u64 {
        get_flipped(self.player, self.opponent, index)
    }

    /// Undo a move by reversing the effect of `do_move`.
    pub fn undo_move(&mut self, index: usize, flips: u64) {
        std::mem::swap(&mut self.player, &mut self.opponent);
        self.player &= !(flips | (1u64 << index));
        self.opponent |= flips;
    }

    /// Create a new position by applying a move to the current position.
    pub fn do_move_cloned(&self, index: usize) -> Self {
        let mut child = *self;
        child.do_move(index);
        child
    }

    /// Returns an ASCII art representation of the board.
    pub fn ascii_art(&self, turn: usize) -> String {
        let (player_char, opponent_char) = if turn == BLACK {
            ("○", "●")
        } else {
            ("●", "○")
        };
        let moves = self.get_moves();

        let mut output = String::new();
        output.push_str("+-A-B-C-D-E-F-G-H-+\n");
        for row in 0..8 {
            output.push_str(&format!("{} ", row + 1));
            for col in 0..8 {
                let index = row * 8 + col;
                let mask = 1u64 << index;
                if self.player & mask != 0 {
                    output.push_str(&format!("{} ", player_char));
                } else if self.opponent & mask != 0 {
                    output.push_str(&format!("{} ", opponent_char));
                } else if moves & mask != 0 {
                    output.push_str("· ");
                } else {
                    output.push_str("  ");
                }
            }
            output.push_str(&format!("{}\n", row + 1));
        }
        output.push_str("+-A-B-C-D-E-F-G-H-+\n");
        output
    }

    /// Count the number of discs on the board.
    pub fn count_discs(&self) -> u32 {
        // TODO binary or and count_ones once.
        self.player.count_ones() + self.opponent.count_ones()
    }

    /// Count the number of empty squares on the board.
    pub fn count_empty(&self) -> u32 {
        64 - self.count_discs()
    }

    /// Return iterator over move indices and resulting child positions.
    pub fn children_with_index(&self) -> Vec<(usize, Position)> {
        let moves = self.get_moves();

        (0..64)
            .filter(|i| moves & (1u64 << i) != 0)
            .map(|i| (i, self.do_move_cloned(i)))
            .collect()
    }

    /// Return iterator over child positions.
    pub fn children(&self) -> Vec<Position> {
        let moves = self.get_moves();

        (0..64)
            .filter(|i| moves & (1u64 << i) != 0)
            .map(|i| self.do_move_cloned(i))
            .collect()
    }

    /// Compute final score of a position.
    pub fn final_score(&self) -> isize {
        let player = self.player.count_ones() as isize;
        let opponent = self.opponent.count_ones() as isize;

        if player > opponent {
            // Player wins
            return 64 - (2 * opponent);
        }

        if opponent > player {
            // Opponent wins
            return -64 + (2 * player);
        }

        // Draw
        0
    }

    /// Computes final score knowing the number of empty squares.
    ///
    /// Like board_solve() in Edax
    pub fn final_score_with_empty(&self, empty_count: i32) -> i32 {
        let player_disc_count = self.player.count_ones() as i32;
        let opponent_disc_count = 64 - empty_count - player_disc_count;
        let mut score = player_disc_count - opponent_disc_count;

        #[allow(clippy::comparison_chain)]
        if score < 0 {
            score -= empty_count;
        } else if score > 0 {
            score += empty_count;
        }

        score
    }

    /// Get the color of a square: 0 for player, 1 for opponent, 2 for empty
    pub fn get_square_color(&self, index: usize) -> usize {
        let color = 2 - 2 * ((self.player >> index) & 1) - ((self.opponent >> index) & 1);
        color as usize
    }

    /// Return iterator over move indices.
    pub fn iter_move_indices(&self) -> MoveIndices {
        MoveIndices::new(self.get_moves())
    }

    /// Count the number of stable discs for the player.
    ///
    /// Like get_stability() in Edax
    pub fn count_player_stable_discs(&self) -> i32 {
        count_stable(self.player, self.opponent)
    }

    /// Count the number of stable discs for the opponent.
    pub fn count_opponent_stable_discs(&self) -> i32 {
        count_stable(self.opponent, self.player)
    }

    /// Compute bitset of potential moves for the opponent.
    /// Used for computing potential moves.
    ///
    /// Like get_potential_moves() in Edax
    fn potential_moves(&self) -> u64 {
        fn get_some_potential_moves(p: u64, dir: i32) -> u64 {
            (p << dir) | (p >> dir)
        }

        let potential_moves = get_some_potential_moves(self.opponent & 0x7E7E7E7E7E7E7E7E, 1) // horizontal
            | get_some_potential_moves(self.opponent & 0x00FFFFFFFFFFFF00, 8) // vertical
            | get_some_potential_moves(self.opponent & 0x007E7E7E7E7E7E00, 7) // diagonals
            | get_some_potential_moves(self.opponent & 0x007E7E7E7E7E7E00, 9);

        let empties = !(self.player | self.opponent);

        potential_moves & empties
    }

    /// Compute bitset of potential moves.
    ///
    /// Like get_potential_mobility() in Edax
    pub fn potential_mobility(&self) -> i32 {
        const CORNERS: u64 = 0x8100000000000081;

        let potential_moves = self.potential_moves();

        (potential_moves.count_ones() as i32) + (potential_moves & CORNERS).count_ones() as i32
    }

    fn corner_stability_internal(player: u64) -> i32 {
        let stable = (((0x0100000000000001 & player) << 1)
            | ((0x8000000000000080 & player) >> 1)
            | ((0x0000000000000081 & player) << 8)
            | ((0x8100000000000000 & player) >> 8)
            | 0x8100000000000081)
            & player;

        stable.count_ones() as i32
    }

    /// Count the number of stable discs around the corner that belong to the player to move.
    ///
    /// Like get_corner_stability() in Edax
    pub fn corner_stability(&self) -> i32 {
        Self::corner_stability_internal(self.player)
    }

    /// Count the number of stable discs around the corner that belong to the opponent.
    ///
    /// Similar to get_corner_stability() in Edax but for the opponent
    pub fn opponent_corner_stability(&self) -> i32 {
        Self::corner_stability_internal(self.opponent)
    }

    /// Compute weighted mobility.
    ///
    /// Like get_weighted_mobility() in Edax
    pub fn weighted_mobility(&self) -> i32 {
        const CORNERS: u64 = 0x8100000000000081;

        let moves = self.get_moves();

        (moves.count_ones() as i32) + (moves & CORNERS).count_ones() as i32
    }

    /// Estimate the number of stable discs on the edges for the player to move.
    ///
    /// Like get_edge_stability() in Edax
    pub fn edge_stability(&self) -> i32 {
        let stable = get_stable_edge(self.player, self.opponent);
        stable.count_ones() as i32
    }

    /// Estimate the number of stable discs on the edges for the opponent.
    ///
    /// Similar to get_edge_stability() in Edax but for the opponent
    pub fn opponent_edge_stability(&self) -> i32 {
        let stable = get_stable_edge(self.opponent, self.player);
        stable.count_ones() as i32
    }

    /// Count the number of moves for the player to move.
    pub fn count_moves(&self) -> usize {
        self.get_moves().count_ones() as usize
    }

    /// Check if the game is finished.
    pub fn is_game_end(&self) -> bool {
        !self.has_moves() && !self.opponent_has_moves()
    }
}

pub struct MoveIndices {
    remaining_moves: u64,
}

impl MoveIndices {
    pub fn new(remaining_moves: u64) -> Self {
        Self { remaining_moves }
    }

    pub fn is_empty(&self) -> bool {
        self.remaining_moves == 0
    }
}

impl Iterator for MoveIndices {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining_moves == 0 {
            return None;
        }

        let index = self.remaining_moves.trailing_zeros() as usize;
        self.remaining_moves &= self.remaining_moves - 1;
        Some(index)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.remaining_moves.count_ones() as usize;
        (size, Some(size))
    }
}

#[cfg(test)]
mod tests {
    use crate::{bot::edax::eval::tests::test_positions, othello::board::WHITE};

    use super::*;

    #[test]
    fn test_new_board() {
        let position = Position::new();
        assert_eq!(position.player, 0x00000000810000000);
        assert_eq!(position.opponent, 0x00000001008000000);
    }

    #[test]
    #[should_panic(expected = "Invalid direction")]
    fn test_shift_invalid_direction() {
        Position::shift(0x0000000810000000, 0);
    }

    #[test]
    fn test_get_moves() {
        let position = Position::new();
        assert_eq!(position.get_moves(), 0x0000102004080000);
    }

    #[test]
    fn test_has_moves() {
        let position = Position::new();
        assert!(position.has_moves());

        let no_moves_position = Position {
            player: 0xFFFFFFFFFFFFFFFF,
            opponent: 0,
        };
        assert!(!no_moves_position.has_moves());
    }

    #[test]
    fn test_do_move() {
        let mut position = Position::new();
        let flips = position.do_move(19); // D3

        assert_eq!(position.player, 0x0000001000000000);
        assert_eq!(position.opponent, 0x0000000818080000);
        assert_eq!(flips, 0x0000000008000000); // Verify the flipped disc at D4
    }

    #[test]
    fn test_ascii_art_black() {
        let position = Position::new();

        // Test ascii_art with black to move
        let result_black = position.ascii_art(BLACK);
        let expected_output_black = "\
+-A-B-C-D-E-F-G-H-+
1                 1
2                 2
3       ·         3
4     · ● ○       4
5       ○ ● ·     5
6         ·       6
7                 7
8                 8
+-A-B-C-D-E-F-G-H-+
";
        assert_eq!(result_black, expected_output_black);
    }

    #[test]
    fn test_ascii_art_white() {
        let mut position = Position::new();
        position.do_move(19); // D3

        // Test ascii_art with white to move
        let result_white = position.ascii_art(WHITE);
        let expected_output_white = "\
+-A-B-C-D-E-F-G-H-+
1                 1
2                 2
3     · ○ ·       3
4       ○ ○       4
5     · ○ ●       5
6                 6
7                 7
8                 8
+-A-B-C-D-E-F-G-H-+
";
        println!("{}", result_white);
        assert_eq!(result_white, expected_output_white);
    }

    #[test]
    fn test_new_from_bitboards() {
        let position = Position::new_from_bitboards(0x1, 0x2);
        assert_eq!(position.player, 0x1);
        assert_eq!(position.opponent, 0x2);
    }

    #[test]
    fn test_pass() {
        let mut position = Position::new();
        let original_player = position.player;
        let original_opponent = position.opponent;

        position.pass();
        assert_eq!(position.player, original_opponent);
        assert_eq!(position.opponent, original_player);

        // Test double pass returns to original position
        position.pass();
        assert_eq!(position.player, original_player);
        assert_eq!(position.opponent, original_opponent);
    }

    #[test]
    fn test_is_valid_move() {
        let position = Position::new();

        // Test valid moves for initial position
        assert!(position.is_valid_move(19)); // D3
        assert!(position.is_valid_move(26)); // E3
        assert!(position.is_valid_move(37)); // F4
        assert!(position.is_valid_move(44)); // E5

        // Test invalid moves
        assert!(!position.is_valid_move(0)); // A1
        assert!(!position.is_valid_move(64)); // Out of bounds
    }

    #[test]
    fn test_count_discs() {
        let position = Position::new();
        assert_eq!(position.count_discs(), 4);

        let full_position = Position {
            player: 0xFFFFFFFFFFFFFFFF,
            opponent: 0x0000000000000000,
        };
        assert_eq!(full_position.count_discs(), 64);
    }

    #[test]
    fn test_default() {
        assert_eq!(Position::default(), Position::new_empty());
    }

    #[test]
    fn test_new_xot() {
        let position = Position::new_xot();
        assert_eq!(position.count_discs(), 12);
    }

    #[test]
    fn test_count_empty() {
        let position = Position::new();
        assert_eq!(position.count_empty(), 60); // 64 - 4 initial discs

        let full_position = Position {
            player: 0xFFFFFFFFFFFFFFFF,
            opponent: 0x0000000000000000,
        };
        assert_eq!(full_position.count_empty(), 0);
    }

    #[test]
    fn test_children_with_index() {
        let position = Position::new();
        let children = position.children_with_index();

        // Initial position should have 4 valid moves
        assert_eq!(children.len(), 4);

        // Verify the indices are correct for initial position
        let indices: Vec<usize> = children.iter().map(|(i, _)| *i).collect();
        assert!(indices.contains(&19)); // D3
        assert!(indices.contains(&26)); // E3
        assert!(indices.contains(&37)); // F4
        assert!(indices.contains(&44)); // E5
    }

    #[test]
    fn test_children() {
        let position = Position::new();
        let children = position.children();

        // Initial position should have 4 valid moves
        assert_eq!(children.len(), 4);

        // Each child should be a valid position
        for child in children {
            assert!(child.count_discs() > position.count_discs());
        }
    }

    #[test]
    fn test_final_score() {
        // Test player win
        let player_wins = Position {
            player: 0x0000000000000007,   // 3 discs
            opponent: 0x0000000000000001, // 1 disc
        };
        assert_eq!(player_wins.final_score(), 62); // 64 - (2 * 1)

        // Test opponent win
        let opponent_wins = Position {
            player: 0x0000000000000001,   // 1 disc
            opponent: 0x0000000000000007, // 3 discs
        };
        assert_eq!(opponent_wins.final_score(), -62); // -64 + (2 * 1)

        // Test draw
        let draw = Position {
            player: 0x0000000000000003,   // 2 discs
            opponent: 0x0000000000000003, // 2 discs
        };
        assert_eq!(draw.final_score(), 0);
    }

    #[test]
    fn test_do_move_cloned() {
        let position = Position::new();
        let child = position.do_move_cloned(19); // D3

        // Original position should remain unchanged
        assert_eq!(position, Position::new());

        // Child position should be modified
        assert_ne!(child, position);
        assert_eq!(child.player, 0x0000001000000000);
        assert_eq!(child.opponent, 0x0000000818080000);
    }

    #[test]
    fn test_get_square_color() {
        let position = Position::new();

        // Test initial position squares
        assert_eq!(position.get_square_color(28), 0); // Player disc at D4
        assert_eq!(position.get_square_color(36), 1); // Opponent disc at D5
        assert_eq!(position.get_square_color(0), 2); // Empty square at A1
    }

    #[test]
    fn test_undo_move() {
        let mut position = Position::new();
        let original = position;

        // Do a move and store the flips
        let flips = position.do_move(19); // D3
        assert_ne!(position, original);

        // Undo the move
        position.undo_move(19, flips);

        // Position should be back to original state
        assert_eq!(position, original);
    }

    #[test]
    fn test_undo_move_multiple() {
        let mut position = Position::new();

        // Do and undo several moves
        let moves_and_flips = vec![
            (19, position.do_move(19)), // D3
            (26, position.do_move(26)), // E3
            (20, position.do_move(20)), // E2
        ];

        // Undo moves in reverse order
        for (index, flips) in moves_and_flips.into_iter().rev() {
            position.undo_move(index, flips);
        }

        // Position should be back to initial state
        assert_eq!(position, Position::new());
    }

    #[test]
    fn test_get_opponent_moves() {
        let mut position = Position::new();

        let found = position.get_opponent_moves();

        position.pass();
        let expected = position.get_moves();

        assert_eq!(found, expected);
    }

    #[test]
    fn test_new_random_with_discs() {
        // Test minimum discs (4)
        let position = Position::new_random_with_discs(4);
        assert_eq!(position.count_discs(), 4);

        // Test some middle value
        let position = Position::new_random_with_discs(20);
        assert_eq!(position.count_discs(), 20);

        // Test maximum discs (64)
        let position = Position::new_random_with_discs(64);
        assert_eq!(position.count_discs(), 64);
    }

    #[test]
    #[should_panic(expected = "Number of discs must be between 4 and 64")]
    fn test_new_random_with_discs_too_few() {
        Position::new_random_with_discs(3);
    }

    #[test]
    #[should_panic(expected = "Number of discs must be between 4 and 64")]
    fn test_new_random_with_discs_too_many() {
        Position::new_random_with_discs(65);
    }

    #[test]
    fn test_new_random_with_discs_valid_positions() {
        for _ in 0..10 {
            // Test multiple random positions
            let position = Position::new_random_with_discs(20);

            // Check that all discs are properly placed (no overlaps)
            assert_eq!(
                (position.player | position.opponent).count_ones(),
                position.count_discs()
            );

            // Check that player and opponent don't overlap
            assert_eq!(position.player & position.opponent, 0);
        }
    }

    #[test]
    fn test_iter_move_indices() {
        // Test starting position
        let position = Position::new();
        let moves: Vec<usize> = position.iter_move_indices().collect();

        // Starting position should have 4 valid moves: D3, C4, F5, E6
        assert_eq!(moves.len(), 4);
        assert!(moves.contains(&19)); // D3
        assert!(moves.contains(&26)); // E3
        assert!(moves.contains(&37)); // F4
        assert!(moves.contains(&44)); // E5

        // Test position with no moves
        let no_moves_position = Position {
            player: 0xFFFFFFFFFFFFFFFF,
            opponent: 0,
        };
        let moves: Vec<usize> = no_moves_position.iter_move_indices().collect();
        assert!(moves.is_empty());
    }

    #[test]
    fn test_new_empty() {
        let position = Position::new_empty();
        assert_eq!(position.player, 0);
        assert_eq!(position.opponent, 0);
    }

    #[test]
    fn test_final_score_with_empty() {
        // Test player winning with empty squares
        let player_wins = Position {
            player: 0x0000000000000007,   // 3 discs
            opponent: 0x0000000000000001, // 1 disc
        };
        assert_eq!(player_wins.final_score_with_empty(60), 62); // 64 - (2 * 1)

        // Test opponent winning with empty squares
        let opponent_wins = Position {
            player: 0x0000000000000001,   // 1 disc
            opponent: 0x0000000000000007, // 3 discs
        };
        assert_eq!(opponent_wins.final_score_with_empty(60), -62); // -64 + (2 * 1)

        // Test draw with empty squares
        let draw = Position {
            player: 0x0000000000000003,   // 2 discs
            opponent: 0x0000000000000003, // 2 discs
        };
        assert_eq!(draw.final_score_with_empty(60), 0);

        // Test with no empty squares
        let full_player_wins = Position {
            player: 0xFFFFFFFFFFFFFFFE,   // 63 discs
            opponent: 0x0000000000000001, // 1 disc
        };
        assert_eq!(full_player_wins.final_score_with_empty(0), 62); // 64 - (2 * 1)
    }

    #[test]
    fn test_opponent_has_moves() {
        // Test initial position - opponent should have moves
        let position = Position::new();
        assert!(position.opponent_has_moves());

        // Test position where opponent has no moves
        let no_moves_position = Position {
            player: 0x0000000000000000,
            opponent: 0xFFFFFFFFFFFFFFFF,
        };
        assert!(!no_moves_position.opponent_has_moves());
    }

    #[test]
    fn test_count_moves() {
        // Test initial position (should have 4 moves)
        let position = Position::new();
        assert_eq!(position.count_moves(), 4);

        // Test position with no moves
        let no_moves_position = Position {
            player: 0xFFFFFFFFFFFFFFFF,
            opponent: 0,
        };
        assert_eq!(no_moves_position.count_moves(), 0);

        // Test position with single move
        let single_move_position = Position {
            player: 0x0000000800000000,   // Single disc in center
            opponent: 0x0000001000000000, // Adjacent disc
        };
        assert_eq!(single_move_position.count_moves(), 1);
    }

    #[test]
    fn test_new_from_parent_and_move() {
        for position in test_positions() {
            for index in position.iter_move_indices() {
                let (child, flipped) = Position::new_from_parent_and_move(&position, index);

                let mut expected_child = position;
                let expected_flipped = expected_child.do_move(index);
                assert_eq!(child, expected_child);
                assert_eq!(flipped, expected_flipped);
            }
        }
    }
}
