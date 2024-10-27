use std::fmt::{self, Display};

#[derive(Clone)]
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
        write!(f, "{}", self.ascii_art(true))
    }
}

impl Position {
    pub fn new() -> Self {
        Self {
            player: 0x00000000810000000,
            opponent: 0x00000001008000000,
        }
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
        self.get_moves() & (1u64 << index) != 0 // TODO add tests
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

    pub fn ascii_art(&self, black_to_move: bool) -> String {
        let (player_char, opponent_char) = if black_to_move {
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
}

#[cfg(test)]
mod tests {
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
        let result_black = position.ascii_art(true);
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
        let result_white = position.ascii_art(false);
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
}
