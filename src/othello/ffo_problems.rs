#[cfg(test)]
pub mod tests {
    use crate::othello::squares::*;

    use std::collections::HashMap;
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    use crate::othello::position::Position;

    pub struct Problem {
        pub line_number: usize,
        pub position: Position,
        pub depth: u32,
        pub solutions: HashMap<usize, isize>,
    }

    pub fn parse_ffo_problems() -> Vec<Problem> {
        let file = File::open("assets/ffo_problems.txt").expect("Failed to open FFO problems file");
        let reader = BufReader::new(file);
        let mut problems = Vec::new();

        for (line_idx, line) in reader.lines().enumerate() {
            let line = line.expect("Failed to read line");

            // Split the line into position and solutions
            let parts: Vec<&str> = line.split(';').collect();
            if parts.len() < 2 {
                continue;
            }

            // Parse the position part (format: "BOARD_STATE COLOR")
            let pos_parts: Vec<&str> = parts[0].split_whitespace().collect();
            if pos_parts.len() != 2 {
                continue;
            }

            // Convert board string to bitboards
            let board = pos_parts[0];
            let mut player = 0u64;
            let mut opponent = 0u64;

            for (i, c) in board.chars().enumerate() {
                if i >= 64 {
                    break;
                }
                let mask = 1u64 << i;
                match c {
                    'X' => player |= mask,
                    'O' => opponent |= mask,
                    _ => continue,
                }
            }

            // If player is 'O', swap the bitboards
            if pos_parts[1] == "O" {
                std::mem::swap(&mut player, &mut opponent);
            }

            let position = Position::new_from_bitboards(player, opponent);

            // Parse solutions (format: "MOVE:SCORE")
            let mut solutions = HashMap::new();
            for part in &parts[1..] {
                let solution_parts: Vec<&str> = part.trim().split(':').collect();
                if solution_parts.len() != 2 {
                    continue;
                }

                // Parse move (format: "A1", "B2", etc.)
                let mv = solution_parts[0].trim();
                if mv.len() != 2 {
                    continue;
                }

                let col = (mv.chars().next().unwrap() as u8 - b'A') as usize;
                let row = (mv.chars().nth(1).unwrap() as u8 - b'1') as usize;
                let index = row * 8 + col;

                // Parse score
                if let Ok(score) = solution_parts[1].trim().parse::<isize>() {
                    solutions.insert(index, score);
                }
            }

            problems.push(Problem {
                line_number: line_idx + 1,
                position,
                depth: position.count_empty(),
                solutions,
            });
        }

        problems
    }

    #[test]
    fn test_parse_ffo_problems() {
        let problems = parse_ffo_problems();

        // Check that we have the expected number of problems
        assert_eq!(problems.len(), 79);

        // Check first problem
        let first = &problems[0];
        let expected_first_solution = HashMap::from([
            (G8, 18),
            (H1, 12),
            (H7, 6),
            (A2, 6),
            (A3, 4),
            (B1, -4),
            (A4, -22),
            (G2, -24),
        ]);

        assert_eq!(first.solutions, expected_first_solution);

        // Check last problem
        let last = &problems[problems.len() - 1];
        let expected_last_solution = HashMap::from([
            (D7, 64),
            (D8, 62),
            (H8, 56),
            (C3, 30),
            (B8, 16),
            (C4, 14),
            (E2, 12),
            (D2, 12),
            (G7, -2),
        ]);
        assert_eq!(last.solutions, expected_last_solution);
    }
}
