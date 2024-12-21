use edax::bot::EdaxBot;
use random::RandomBot;
use squared::bot::SquaredBot;

use crate::othello::position::Position;

pub mod edax;
pub mod random;
pub mod squared;

/// A bot that can compute the best move for a position.
pub trait Bot: Send {
    /// Computes the best move for a position.
    /// At least one move must be available in `position`.
    /// Returns the index of the move.
    fn get_move(&mut self, position: &Position) -> usize;
}

/// Get a bot by name or `None` if not found`.
pub fn get_bot(name: &str) -> Option<Box<dyn Bot>> {
    match name {
        "random" => Some(Box::new(RandomBot)),
        "squared" => Some(Box::new(SquaredBot)),
        "edax" => Some(Box::new(EdaxBot)),
        _ => None,
    }
}

/// Format a score to be 4 characters wide.
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

/// Format a number of nodes to be 8 characters wide.
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
