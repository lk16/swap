use std::fmt::{self, Display};

use crate::position::Position;

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

    pub fn get_moves(&self) -> u64 {
        self.position.get_moves()
    }

    pub fn has_moves(&self) -> bool {
        self.position.has_moves()
    }

    pub fn do_move(&mut self, index: usize) {
        self.position.do_move(index);
        self.black_to_move = !self.black_to_move;
    }

    pub fn ascii_art(&self) -> String {
        self.position.ascii_art(self.black_to_move)
    }

    pub fn as_ws_message(&self) -> String {
        let mut result = String::with_capacity(66);

        let (player_char, opponent_char) = if self.black_to_move {
            ('b', 'w')
        } else {
            ('w', 'b')
        };

        for i in 0..64 {
            let mask = 1u64 << i;
            if self.position.player & mask != 0 {
                result.push(player_char);
            } else if self.position.opponent & mask != 0 {
                result.push(opponent_char);
            } else {
                result.push('.');
            }
        }

        // Add a space and the current player indicator
        result.push(' ');
        result.push(player_char);

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let expected = "...........................wb......bw........................... b";
        assert_eq!(board.as_ws_message(), expected);
    }

    #[test]
    fn test_as_ws_message_white() {
        let mut board = Board::new();
        board.do_move(19); // D3
        let expected_after_move =
            "...................b.......bb......bw........................... w";
        assert_eq!(board.as_ws_message(), expected_after_move);
    }
}
