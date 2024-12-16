/// Compute flipped discs when moving to a given square.
///
/// `player` is the bitboard of the player's discs.
/// `opponent` is the bitboard of the opponent's discs.
/// `index` is the index of the square of the move
pub fn get_flipped_simple(player: u64, opponent: u64, index: usize) -> u64 {
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

    let mut flipped = 0u64;

    for (dx, dy) in DIRECTIONS {
        let mut direction_flipped = 0u64;

        for distance in 1..=7 {
            let square_x = move_x as i32 + distance * dx;
            let square_y = move_y as i32 + distance * dy;

            if !(0..8).contains(&square_x) || !(0..8).contains(&square_y) {
                break;
            }

            let square_mask = 1u64 << (square_y * 8 + square_x);

            // Square has opponent's piece
            if square_mask & opponent != 0 {
                direction_flipped |= square_mask;
                continue;
            }

            // Square has player's piece
            if square_mask & player != 0 {
                flipped |= direction_flipped;
                break;
            }

            // Square is empty
            break;
        }
    }

    flipped
}
