pub mod edax_bitscan;
pub mod edax_slow;
pub mod simple;

use super::position::Position;

pub fn do_move(position: &mut Position, index: usize) -> u64 {
    simple::do_move_simple(position, index)
}
