/// Compute flipped discs when moving to a given square.
///
/// `player` is the bitboard of the player's discs.
/// `opponent` is the bitboard of the opponent's discs.
/// `index` is the index of the square of the move
pub fn get_flipped_edax_slow(player: u64, opponent: u64, index: usize) -> u64 {
    fn x_to_bit(x: i32) -> u64 {
        1u64 << (x as usize)
    }

    const DIR: [i32; 8] = [-9, -8, -7, -1, 1, 7, 8, 9];

    const EDGE: [u64; 8] = [
        0x01010101010101ff,
        0x00000000000000ff,
        0x80808080808080ff,
        0x0101010101010101,
        0x8080808080808080,
        0xff01010101010101,
        0xff00000000000000,
        0xff80808080808080,
    ];

    let mut flipped = 0;

    for d in 0..8 {
        if (x_to_bit(index as i32) & EDGE[d]) == 0 {
            let mut f = 0;
            let mut x = index as i32 + DIR[d];
            while (opponent & x_to_bit(x)) != 0 && (x_to_bit(x) & EDGE[d]) == 0 {
                f |= x_to_bit(x);
                x += DIR[d];
            }
            if (player & x_to_bit(x)) != 0 {
                flipped |= f;
            }
        }
    }

    flipped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::othello::{
        get_flipped::simple::get_flipped_simple, get_moves::tests::move_test_cases,
        position::print_bitset,
    };

    #[test]
    fn test_do_move_edax_slow() {
        let test_cases = move_test_cases();

        for position in &test_cases {
            for move_ in position.iter_move_indices() {
                if move_ == 27 || move_ == 28 || move_ == 35 || move_ == 36 {
                    continue;
                }

                let player = position.player;
                let opponent = position.opponent;

                let simple = get_flipped_simple(player, opponent, move_);
                let edax_slow = get_flipped_edax_slow(player, opponent, move_);

                if simple != edax_slow {
                    println!("move = {}", move_);

                    println!("position:");
                    println!("{}", position);

                    println!("simple:");
                    print_bitset(simple);

                    println!("edax_slow:");
                    print_bitset(edax_slow);

                    panic!();
                }
            }
        }
    }
}
