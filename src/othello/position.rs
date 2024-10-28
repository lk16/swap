use lazy_static::lazy_static;
use rand::{rngs::ThreadRng, RngCore};
use serde_json::Value;
use std::fmt::{self, Display};
use std::fs::File;
use std::io::Read;
use std::path::Path;

use super::board::BLACK;

lazy_static! {
    static ref XOT_POSITIONS: Vec<Position> = {
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

#[derive(PartialEq, Debug)]
pub enum GameState {
    HasMoves,
    Passed,
    Finished,
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct Position {
    pub player: u64,
    pub opponent: u64,
}

impl Default for Position {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.ascii_art(BLACK))
    }
}

impl Position {
    pub fn new() -> Self {
        Self {
            player: 0x00000000810000000,
            opponent: 0x00000001008000000,
        }
    }

    pub fn new_xot() -> Self {
        let n = ThreadRng::default().next_u64() as usize;
        XOT_POSITIONS[n % XOT_POSITIONS.len()]
    }

    pub fn new_from_bitboards(player: u64, opponent: u64) -> Self {
        Self { player, opponent }
    }

    fn shift(bitboard: u64, dir: i32) -> u64 {
        match dir {
            -9 => (bitboard & 0xfefefefefefefefe) << 7,
            -8 => bitboard << 8,
            -7 => (bitboard & 0x7f7f7f7f7f7f7f7f) << 9,
            -1 => (bitboard & 0xfefefefefefefefe) >> 1,
            1 => (bitboard & 0x7f7f7f7f7f7f7f7f) << 1,
            7 => (bitboard & 0x7f7f7f7f7f7f7f7f) >> 7,
            8 => bitboard >> 8,
            9 => (bitboard & 0xfefefefefefefefe) >> 9,
            _ => panic!("Invalid direction"),
        }
    }

    pub fn get_moves(&self) -> u64 {
        let empty = !(self.player | self.opponent);
        let mut moves = 0;

        // Define direction offsets
        const DIRECTIONS: [i32; 8] = [-9, -8, -7, -1, 1, 7, 8, 9];

        for dir in DIRECTIONS {
            let mut candidates = Self::shift(self.player, dir) & self.opponent;

            while candidates != 0 {
                moves |= empty & Self::shift(candidates, dir);
                candidates = Self::shift(candidates, dir) & self.opponent;
            }
        }
        moves
    }

    pub fn has_moves(&self) -> bool {
        self.get_moves() != 0
    }

    pub fn is_valid_move(&self, index: usize) -> bool {
        if index >= 64 {
            return false;
        }

        self.get_moves() & (1u64 << index) != 0
    }

    pub fn pass(&mut self) {
        std::mem::swap(&mut self.player, &mut self.opponent);
    }

    pub fn game_state(&self) -> GameState {
        if self.has_moves() {
            return GameState::HasMoves;
        }

        let mut passed = *self;
        passed.pass();

        if passed.has_moves() {
            return GameState::Passed;
        }

        GameState::Finished
    }

    pub fn do_move(&mut self, index: usize) {
        let move_bit = 1u64 << index;
        self.player |= move_bit;

        // Define direction offsets
        const DIRECTIONS: [i32; 8] = [-9, -8, -7, -1, 1, 7, 8, 9];

        for &dir in &DIRECTIONS {
            let mut flip = 0u64;
            let mut edge = move_bit;

            loop {
                edge = Self::shift(edge, dir);
                if edge & self.opponent == 0 {
                    break;
                }
                flip |= edge;
            }

            if edge & self.player != 0 {
                self.player |= flip;
                self.opponent &= !flip;
            }
        }

        // Swap player and opponent
        std::mem::swap(&mut self.player, &mut self.opponent);
    }

    pub fn do_move_cloned(&self, index: usize) -> Self {
        let mut child = *self;
        child.do_move(index);
        child
    }

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

    pub fn count_discs(&self) -> u32 {
        self.player.count_ones() + self.opponent.count_ones()
    }

    pub fn count_empty(&self) -> u32 {
        64 - self.count_discs()
    }

    pub fn children_with_index(&self) -> Vec<(usize, Position)> {
        let moves = self.get_moves();

        (0..64)
            .filter(|i| moves & (1u64 << i) != 0)
            .map(|i| (i, self.do_move_cloned(i)))
            .collect()
    }

    pub fn children(&self) -> Vec<Position> {
        let moves = self.get_moves();

        (0..64)
            .filter(|i| moves & (1u64 << i) != 0)
            .map(|i| self.do_move_cloned(i))
            .collect()
    }

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
}

#[cfg(test)]
mod tests {
    use crate::othello::board::WHITE;

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
        position.do_move(19); // D3

        assert_eq!(position.player, 0x0000001000000000);
        assert_eq!(position.opponent, 0x0000000818080000);
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
    fn test_game_state() {
        // Test HasMoves
        let position = Position::new();
        assert_eq!(position.game_state(), GameState::HasMoves);

        // Test Passed
        let passed_position = Position {
            player: 0x2,
            opponent: 0x1,
        };
        assert_eq!(passed_position.game_state(), GameState::Passed);

        // Test Finished
        let finished_position = Position {
            player: 0xFFFFFFFFFFFFFFFF,
            opponent: 0x0000000000000000,
        };
        assert_eq!(finished_position.game_state(), GameState::Finished);
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
        assert_eq!(Position::default(), Position::new());
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
}
