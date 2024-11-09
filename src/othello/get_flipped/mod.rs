pub mod edax_bitscan;
pub mod edax_slow;
pub mod simple;

pub fn get_flipped(player: u64, opponent: u64, index: usize) -> u64 {
    edax_bitscan::get_flipped_edax_bitscan(player, opponent, index)
}
