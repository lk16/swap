use serde_json::json;
use std::fmt::{self, Display};

use crate::position::Position;

#[derive(Clone)]
pub struct Board {
    position: Position,
    black_to_move: bool,
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

    pub fn get_moves(&self) -> u64 {
        self.position.get_moves()
    }

    pub fn has_moves(&self) -> bool {
        self.position.has_moves()
    }

    pub fn is_valid_move(&self, index: usize) -> bool {
        self.position.is_valid_move(index) // TODO add tests
    }

    pub fn do_move(&mut self, index: usize) {
        self.position.do_move(index);
        self.black_to_move = !self.black_to_move;
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
}
