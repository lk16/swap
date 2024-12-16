pub mod edax_bitscan;
pub mod edax_slow;
pub mod simple;

/// Compute flipped discs when moving to a given square.
///
/// `player` is the bitboard of the player's discs.
/// `opponent` is the bitboard of the opponent's discs.
/// `index` is the index of the square of the move
pub fn get_flipped(player: u64, opponent: u64, index: usize) -> u64 {
    edax_bitscan::get_flipped_edax_bitscan(player, opponent, index)
}

// TODO move edax_bitscan here, rename to get_flipped.rs, move other implementation into tests
