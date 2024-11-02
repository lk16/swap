use std::time::Duration;

use edax::bot::EdaxBot;
use random::RandomBot;
use squared::bot::SquaredBot;

use crate::othello::position::Position;

pub mod edax;
pub mod random;
pub mod squared;

pub trait Bot: Send {
    // Returns the index of a valid move
    fn get_move(&mut self, position: &Position) -> usize;
}

pub fn get_bot(name: &str) -> Option<Box<dyn Bot>> {
    match name {
        "random" => Some(Box::new(RandomBot)),
        "squared" => Some(Box::new(SquaredBot)),
        "edax" => Some(Box::new(EdaxBot)),
        _ => None,
    }
}

pub fn print_search_header(name: &'static str, is_endgame: bool, depth: u32) {
    let search = if is_endgame { "endgame" } else { "midgame" };
    println!("{} searching {} at depth {}", name, search, depth);
}

pub fn print_move_stats(
    nodes: u64,
    current_move: usize,
    total_moves: usize,
    score: isize,
    alpha: isize,
    duration: Duration,
) {
    println!(
        "Move {:2}/{:2}: score {} {:6} | {:6}n / {:.3}s = {:6}n/s",
        current_move + 1,
        total_moves,
        if score > alpha { "==" } else { "<=" },
        score,
        nodes,
        duration.as_secs_f64(),
        (nodes as f64 / duration.as_secs_f64()) as usize
    );
}

pub fn print_total_stats(total_nodes: u64, total_duration: Duration) {
    println!(
        "Total nodes: {:6} | Total duration: {:.3}s | Total n/s: {:6}",
        total_nodes,
        total_duration.as_secs_f64(),
        (total_nodes as f64 / total_duration.as_secs_f64()) as usize
    );
    println!();
}
