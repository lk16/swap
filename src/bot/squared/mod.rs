use std::time::Duration;

use crate::bot::{format_nodes, format_score};

pub mod bot;
pub mod endgame;
pub mod midgame;

/// Print a search header.
pub fn print_search_header(name: &'static str, is_endgame: bool, depth: u32) {
    let search = if is_endgame { "endgame" } else { "midgame" };
    println!("{} searching {} at depth {}", name, search, depth);
}

/// Print move stats.
pub fn print_move_stats(
    nodes: u64,
    current_move: usize,
    total_moves: usize,
    score: isize,
    alpha: isize,
    duration: Duration,
) {
    let speed = (nodes as f64 / duration.as_secs_f64()) as u64;

    println!(
        "Move {:2}/{:2}: score {} {} | {} / {:.3}s = {}/s",
        current_move + 1,
        total_moves,
        if score > alpha { "==" } else { "<=" },
        format_score(score),
        format_nodes(nodes),
        duration.as_secs_f64(),
        format_nodes(speed),
    );
}

/// Print total stats.
pub fn print_total_stats(total_nodes: u64, total_duration: Duration) {
    let speed = (total_nodes as f64 / total_duration.as_secs_f64()) as u64;

    println!(
        "     Total:               | {} / {:.3}s = {}/s",
        format_nodes(total_nodes),
        total_duration.as_secs_f64(),
        format_nodes(speed),
    );
    println!();
}
