use crate::othello::position::Position;

pub fn do_move_simple(position: &mut Position, index: usize) -> u64 {
    let move_x = index % 8;
    let move_y = index / 8;

    const DIRECTIONS: [(i32, i32); 8] = [
        (-1, -1),
        (-1, 0),
        (-1, 1),
        (0, -1),
        (0, 1),
        (1, -1),
        (1, 0),
        (1, 1),
    ];

    let mut total_flips = 0u64;
    let move_mask = 1u64 << index;

    for (dx, dy) in DIRECTIONS {
        let mut direction_flips = 0u64;

        for distance in 1..=7 {
            let square_x = move_x as i32 + distance * dx;
            let square_y = move_y as i32 + distance * dy;

            if !(0..8).contains(&square_x) || !(0..8).contains(&square_y) {
                break;
            }

            let square_mask = 1u64 << (square_y * 8 + square_x);

            // Square has opponent's piece
            if square_mask & position.opponent != 0 {
                direction_flips |= square_mask;
                continue;
            }

            // Square has player's piece
            if square_mask & position.player != 0 {
                total_flips |= direction_flips;
                break;
            }

            // Square is empty
            break;
        }
    }

    // Actually perform the move
    position.player ^= total_flips | move_mask;
    position.opponent ^= total_flips;

    // swap the player and opponent
    std::mem::swap(&mut position.player, &mut position.opponent);

    total_flips
}
