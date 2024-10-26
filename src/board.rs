use std::{io, io::Write};

pub struct Board {
    pub player: u64,
    pub opponent: u64,
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

impl Board {
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

    pub fn do_move(&mut self, index: usize) {
        let move_bit = 1u64 << index;
        self.player |= move_bit;

        // Define direction offsets
        const DIRECTIONS: [i32; 8] = [-9, -8, -7, -1, 1, 7, 8, 9];

        for dir in DIRECTIONS {
            let mut flip = 0u64;
            let mut candidates = Self::shift(move_bit, dir) & self.opponent;

            while candidates != 0 {
                flip |= candidates;
                candidates = Self::shift(candidates, dir) & self.opponent;
            }

            if Self::shift(candidates, dir) & self.player != 0 {
                self.player |= flip;
                self.opponent &= !flip;
            }
        }

        // Swap player and opponent
        std::mem::swap(&mut self.player, &mut self.opponent);
    }

    pub fn print<W: Write>(&self, w: &mut W, black_to_move: bool) -> io::Result<()> {
        let player_char = if black_to_move { "○" } else { "●" };
        let opponent_char = if black_to_move { "●" } else { "○" };
        let moves = self.get_moves();

        writeln!(w, "+-A-B-C-D-E-F-G-H-+")?;
        for row in 0..8 {
            write!(w, "{} ", row + 1)?;
            for col in 0..8 {
                let index = row * 8 + col;
                let mask = 1u64 << index;
                if self.player & mask != 0 {
                    write!(w, "{} ", player_char)?;
                } else if self.opponent & mask != 0 {
                    write!(w, "{} ", opponent_char)?;
                } else if moves & mask != 0 {
                    write!(w, "· ")?;
                } else {
                    write!(w, "  ")?;
                }
            }
            writeln!(w, "{}", row + 1)?;
        }
        writeln!(w, "+-A-B-C-D-E-F-G-H-+")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_board() {
        let board = Board::new();
        assert_eq!(board.player, 0x00000000810000000);
        assert_eq!(board.opponent, 0x00000001008000000);
    }

    #[test]
    #[should_panic(expected = "Invalid direction")]
    fn test_shift_invalid_direction() {
        Board::shift(0x0000000810000000, 0);
    }

    #[test]
    fn test_get_moves() {
        let board = Board::new();
        let expected_moves = 0x0000102004080000;
        assert_eq!(board.get_moves(), expected_moves);
    }

    #[test]
    fn test_has_moves() {
        let board = Board::new();
        assert!(board.has_moves());

        let no_moves_board = Board {
            player: 0xFFFFFFFFFFFFFFFF,
            opponent: 0,
        };
        assert!(!no_moves_board.has_moves());
    }

    #[test]
    fn test_do_move() {
        let mut board = Board::new();
        board.do_move(26); // Move to D3

        let expected_player = 0x00000001008000000;
        let expected_opponent = 0x00000000814000000;

        assert_eq!(board.player, expected_player);
        assert_eq!(board.opponent, expected_opponent);
    }

    #[test]
    fn test_print_black() {
        let board = Board::new();
        let mut output = Vec::new();

        // Test printing with black to move
        board.print(&mut output, true).unwrap();
        let result_black = String::from_utf8(output).unwrap();
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
    fn test_print_white() {
        let board = Board::new();
        let mut output = Vec::new();

        // Test printing with white to move
        board.print(&mut output, false).unwrap();
        let result_white = String::from_utf8(output).unwrap();
        let expected_output_white = "\
+-A-B-C-D-E-F-G-H-+
1                 1
2                 2
3       ·         3
4     · ○ ●       4
5       ● ○ ·     5
6         ·       6
7                 7
8                 8
+-A-B-C-D-E-F-G-H-+
";
        assert_eq!(result_white, expected_output_white);
    }
}
