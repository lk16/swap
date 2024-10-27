use serde_json::json;
use std::fmt::{self, Display};

use super::position::{GameState, Position};

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct Board {
    pub position: Position,
    pub black_to_move: bool,
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ascii_art = self.position.ascii_art(self.black_to_move);
        write!(f, "{}", ascii_art)
    }
}

impl Board {
    pub fn new() -> Self {
        Self {
            position: Position::new(),
            black_to_move: true,
        }
    }

    pub fn new_xot() -> Self {
        Self {
            position: Position::new_xot(),
            black_to_move: true,
        }
    }

    pub fn new_from_bitboards(black: u64, white: u64, black_to_move: bool) -> Self {
        let (player, opponent) = if black_to_move {
            (black, white)
        } else {
            (white, black)
        };

        let position = Position::new_from_bitboards(player, opponent);
        Self::combine(position, black_to_move)
    }

    pub fn combine(position: Position, black_to_move: bool) -> Self {
        Self {
            position,
            black_to_move,
        }
    }

    pub fn get_moves(&self) -> u64 {
        self.position.get_moves()
    }

    pub fn has_moves(&self) -> bool {
        self.position.has_moves()
    }

    pub fn is_valid_move(&self, index: usize) -> bool {
        self.position.is_valid_move(index)
    }

    pub fn do_move(&mut self, index: usize) {
        self.position.do_move(index);
        self.black_to_move = !self.black_to_move;
    }

    pub fn pass(&mut self) {
        self.black_to_move = !self.black_to_move;
        self.position.pass();
    }

    pub fn game_state(&self) -> GameState {
        self.position.game_state()
    }

    pub fn ascii_art(&self) -> String {
        self.position.ascii_art(self.black_to_move)
    }

    pub fn as_ws_message(&self) -> String {
        let mut black = Vec::new();
        let mut white = Vec::new();
        let mut moves = Vec::new();

        for i in 0..64 {
            let mask = 1u64 << i;
            if self.position.player & mask != 0 {
                if self.black_to_move {
                    black.push(i);
                } else {
                    white.push(i);
                }
            } else if self.position.opponent & mask != 0 {
                if self.black_to_move {
                    white.push(i);
                } else {
                    black.push(i);
                }
            }
        }

        let valid_moves = self.get_moves();
        for i in 0..64 {
            if valid_moves & (1u64 << i) != 0 {
                moves.push(i);
            }
        }

        let turn = if self.black_to_move { "black" } else { "white" };

        json!({
            "black": black,
            "white": white,
            "turn": turn,
            "moves": moves,
        })
        .to_string()
    }

    pub fn count_discs(&self) -> u32 {
        self.position.count_discs()
    }

    pub fn black_discs(&self) -> u64 {
        if self.black_to_move {
            self.position.player
        } else {
            self.position.opponent
        }
    }

    pub fn white_discs(&self) -> u64 {
        if self.black_to_move {
            self.position.opponent
        } else {
            self.position.player
        }
    }

    pub fn do_move_cloned(&self, index: usize) -> Self {
        let position = self.position.do_move_cloned(index);
        Self::combine(position, !self.black_to_move)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn test_new_board() {
        let board = Board::new();
        assert!(board.black_to_move);
        assert_eq!(board.position.player, 0x0000000810000000);
        assert_eq!(board.position.opponent, 0x0000001008000000);
    }

    #[test]
    fn test_has_moves() {
        let board = Board::new();
        assert!(board.has_moves());
    }

    #[test]
    fn test_do_move() {
        let mut board = Board::new();
        board.do_move(19); // D3
        assert!(!board.black_to_move);

        assert_eq!(board.position.player, 0x0000001000000000);
        assert_eq!(board.position.opponent, 0x0000000818080000);
    }

    #[test]
    fn test_get_moves() {
        let board = Board::new();
        let moves = board.get_moves();
        assert_eq!(moves, 0x0000102004080000);
    }

    #[test]
    fn test_ascii_art_black() {
        let board = Board::new();

        // Test ascii_art with black to move
        let result_black = board.ascii_art();
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
        let mut board = Board::new();
        board.do_move(19); // D3

        // Test ascii_art with white to move
        let result_white = board.ascii_art();
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
    fn test_as_ws_message_black() {
        let board = Board::new();
        let message = board.as_ws_message();
        let json: Value = serde_json::from_str(&message).unwrap();

        assert_eq!(json["black"], json!([28, 35]));
        assert_eq!(json["white"], json!([27, 36]));
        assert_eq!(json["turn"], "black");
        assert_eq!(json["moves"], json!([19, 26, 37, 44]));
    }

    #[test]
    fn test_as_ws_message_white() {
        let mut board = Board::new();
        board.do_move(19); // D3
        let message = board.as_ws_message();
        let json: Value = serde_json::from_str(&message).unwrap();

        assert_eq!(json["black"], json!([19, 27, 28, 35]));
        assert_eq!(json["white"], json!([36]));
        assert_eq!(json["turn"], "white");
        assert_eq!(json["moves"], json!([18, 20, 34]));
    }

    #[test]
    fn test_is_valid_move() {
        let board = Board::new();

        // Test valid moves for initial position
        assert!(board.is_valid_move(19)); // D3
        assert!(board.is_valid_move(26)); // C4
        assert!(board.is_valid_move(37)); // E6
        assert!(board.is_valid_move(44)); // F5

        // Test invalid moves
        assert!(!board.is_valid_move(0)); // A1 (corner)
        assert!(!board.is_valid_move(27)); // D4 (occupied)
        assert!(!board.is_valid_move(28)); // E4 (occupied)
        assert!(!board.is_valid_move(64)); // Out of bounds
    }

    #[test]
    fn test_is_valid_move_after_move() {
        let mut board = Board::new();
        board.do_move(19); // D3

        // Test valid moves for white after black plays D3
        assert!(board.is_valid_move(18)); // C3
        assert!(board.is_valid_move(20)); // E3
        assert!(board.is_valid_move(34)); // C5

        // Test invalid moves
        assert!(!board.is_valid_move(19)); // D3 (occupied)
        assert!(!board.is_valid_move(44)); // F5 (not valid for white)
    }

    #[test]
    fn test_new_xot() {
        let board = Board::new_xot();
        assert!(board.black_to_move);
        assert_eq!(board.count_discs(), 12);
    }

    #[test]
    fn test_combine() {
        let position = Position::new();
        let board = Board::combine(position, false);
        assert!(!board.black_to_move);
        assert_eq!(board.position.player, 0x0000000810000000);
        assert_eq!(board.position.opponent, 0x0000001008000000);
    }

    #[test]
    fn test_pass() {
        let board = Board::new();

        let mut passed = board.clone();
        passed.pass();

        assert_eq!(passed.black_discs(), board.black_discs());
        assert_eq!(passed.white_discs(), board.white_discs());
        assert!(!passed.black_to_move);
    }

    #[test]
    fn test_pass_twice() {
        let board = Board::new();

        let mut passed = board.clone();
        passed.pass();
        passed.pass();

        assert_eq!(passed, board);
    }

    #[test]
    fn test_game_state() {
        // Initial position
        let board = Board::new();
        assert_eq!(board.game_state(), GameState::HasMoves);

        // Empty board, nobody can move
        let board = Board::new_from_bitboards(0, 0, false);
        assert_eq!(board.game_state(), GameState::Finished);

        // Black has no moves, white has one move
        let board = Board::new_from_bitboards(0x2, 0x1, true);
        assert_eq!(board.game_state(), GameState::Passed);
    }

    #[test]
    fn test_new_from_bitboards() {
        let black = 0x0000000810000000; // Standard initial black position
        let white = 0x0000001008000000; // Standard initial white position
        let board = Board::new_from_bitboards(black, white, true);

        assert!(board.black_to_move);
        assert_eq!(board.black_discs(), black);
        assert_eq!(board.white_discs(), white);

        // Test with different turn
        let board_white = Board::new_from_bitboards(black, white, false);
        assert!(!board_white.black_to_move);
        assert_eq!(board_white.white_discs(), white);
        assert_eq!(board_white.black_discs(), black);
    }

    #[test]
    fn test_count_discs() {
        // Test initial board position
        let board = Board::new();
        assert_eq!(board.count_discs(), 4);

        // Test custom position with more discs
        let player = 0x000000FF00000000;
        let opponent = 0x0000000000FF0000;
        let board = Board::new_from_bitboards(player, opponent, true);
        assert_eq!(board.count_discs(), 16);
    }

    #[test]
    fn test_default() {
        assert_eq!(Board::default(), Board::new());
    }

    #[test]
    fn test_display() {
        let board = Board::new();
        let display_output = format!("{}", board);
        let ascii_art_output = board.ascii_art();

        assert_eq!(display_output, ascii_art_output);

        // Test after a move to ensure Display works in different states
        let mut moved_board = Board::new();
        moved_board.do_move(19); // D3
        let display_output_after_move = format!("{}", moved_board);
        let ascii_art_output_after_move = moved_board.ascii_art();

        assert_eq!(display_output_after_move, ascii_art_output_after_move);
    }

    #[test]
    fn test_do_move_cloned() {
        let board = Board::new();
        let cloned = board.do_move_cloned(19); // D3

        // Original board should be unchanged
        assert!(board.black_to_move);
        assert_eq!(board.position.player, 0x0000000810000000);
        assert_eq!(board.position.opponent, 0x0000001008000000);

        // Cloned board should reflect the move
        assert!(!cloned.black_to_move);
        assert_eq!(cloned.position.player, 0x0000001000000000);
        assert_eq!(cloned.position.opponent, 0x0000000818080000);
    }
}
