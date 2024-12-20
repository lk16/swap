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

pub fn format_score(score: isize) -> String {
    let mut result = String::new();

    // Handle negative sign
    if score < 0 {
        result.push('-');
    }

    // Get absolute value for processing
    let abs_score = score.abs();

    // Format with 'k' suffix if > 1000
    if abs_score > 1000 {
        result.push_str(&format!("{}k", abs_score / 1000));
    } else {
        result.push_str(&abs_score.to_string());
    }

    // Pad with spaces to ensure 4 characters
    format!("{:>4}", result)
}

pub fn format_nodes(nodes: u64) -> String {
    let suffixes = [" ", "k", "M", "G", "T", "P", "E"];
    let mut value = nodes as f64;
    let mut suffix_idx = 0;

    // Find appropriate suffix
    while value >= 999.5 && suffix_idx < suffixes.len() - 1 {
        value /= 1000.0;
        suffix_idx += 1;
    }

    // Format the number part with suffix + 'n'
    if suffix_idx == 0 {
        format!("{:4}{}n", nodes, suffixes[suffix_idx])
    } else if value > 99.5 {
        format!(" {:3}{}n", value as usize, suffixes[suffix_idx])
    } else if value > 9.95 {
        format!("{:3.1}{}n", value, suffixes[suffix_idx])
    } else {
        format!("{:3.2}{}n", value, suffixes[suffix_idx])
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_score() {
        assert_eq!(format_score(0), "   0");
        assert_eq!(format_score(42), "  42");
        assert_eq!(format_score(-42), " -42");
        assert_eq!(format_score(999), " 999");
        assert_eq!(format_score(1000), "1000");
        assert_eq!(format_score(1500), "  1k");
        assert_eq!(format_score(2000), "  2k");
        assert_eq!(format_score(-1500), " -1k");
        assert_eq!(format_score(-2000), " -2k");
    }

    #[test]
    fn test_format_nodes() {
        // Zero and exact powers of 1000
        assert_eq!(format_nodes(0), "   0 n");
        assert_eq!(format_nodes(1000), "1.00kn");
        assert_eq!(format_nodes(1000000), "1.00Mn");
        assert_eq!(format_nodes(1000000000), "1.00Gn");
        assert_eq!(format_nodes(1000000000000), "1.00Tn");
        assert_eq!(format_nodes(1000000000000000), "1.00Pn");
        assert_eq!(format_nodes(1000000000000000000), "1.00En");

        // Intermediate values
        assert_eq!(format_nodes(1234), "1.23kn");
        assert_eq!(format_nodes(12345), "12.3kn");
        assert_eq!(format_nodes(123456), " 123kn");
        assert_eq!(format_nodes(1234567), "1.23Mn");
        assert_eq!(format_nodes(12345678), "12.3Mn");
        assert_eq!(format_nodes(123456789), " 123Mn");
        assert_eq!(format_nodes(1234567890), "1.23Gn");
    }
}
