use crate::othello::position::Position;

pub fn do_move_edax_slow(position: &mut Position, index: usize) -> u64 {
    let flips = get_flips(position.player, position.opponent, index as i32);

    position.player |= flips | (1u64 << index);
    position.opponent ^= flips;

    std::mem::swap(&mut position.player, &mut position.opponent);

    flips
}

fn get_flips(p: u64, o: u64, x0: i32) -> u64 {
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
        if (x_to_bit(x0) & EDGE[d]) == 0 {
            let mut f = 0;
            let mut x = x0 + DIR[d];
            while (o & x_to_bit(x)) != 0 && (x_to_bit(x) & EDGE[d]) == 0 {
                f |= x_to_bit(x);
                x += DIR[d];
            }
            if (p & x_to_bit(x)) != 0 {
                flipped |= f;
            }
        }
    }

    flipped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::othello::{do_move::simple::do_move_simple, get_moves::tests::move_test_cases};

    #[test]
    fn test_do_move_edax_slow() {
        let test_cases = move_test_cases();

        for position in &test_cases {
            let mut remaining_moves = position.get_moves();

            while remaining_moves != 0 {
                println!("---\n\n\n");

                let move_ = remaining_moves.trailing_zeros() as usize;
                remaining_moves &= remaining_moves - 1;

                if move_ == 27 || move_ == 28 || move_ == 35 || move_ == 36 {
                    continue;
                }

                let mut simple_after = *position;
                let simple_flipped = do_move_simple(&mut simple_after, move_);

                let mut edax_slow_after = *position;
                let edax_slow_flipped = do_move_edax_slow(&mut edax_slow_after, move_);

                if simple_after != edax_slow_after || simple_flipped != edax_slow_flipped {
                    println!("move = {}", move_);

                    println!("position:");
                    println!("{}", position);

                    println!("simple:");
                    println!("{}", simple_after);

                    println!("edax_slow:");
                    println!("{}", edax_slow_after);

                    assert!(false);
                }
            }
        }
    }
}
