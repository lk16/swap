pub mod edax_bitscan;
pub mod edax_slow;
pub mod simple;

use super::position::Position;

pub fn do_move(position: &mut Position, index: usize) -> u64 {
    edax_bitscan::do_move_edax_bitscan(position, index)
}
