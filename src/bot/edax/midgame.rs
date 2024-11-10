use std::time::{Duration, Instant};

use crate::bot::{print_move_stats, print_search_header, print_total_stats};
use crate::othello::position::Position;

use super::bot::MIDGAME_DEPTH;
use super::eval::Eval;

pub const SCORE_MIN: i32 = -64;
pub const SCORE_MAX: i32 = 64;

pub struct MidgameSearch {
    // Contains search root position, does not change during search
    root: Position,

    // Contains search state, changes during search
    eval: Eval,

    // Number of nodes visited in the current search
    nodes: u64,
}

impl MidgameSearch {
    pub fn new(position: Position) -> Self {
        // TODO This should match search_setup() in Edax

        Self {
            root: position,
            eval: Eval::default(),
            nodes: 0,
        }
    }

    pub fn get_move(&mut self) -> usize {
        let children = self.root.children_with_index();
        let mut best_move = children.first().unwrap().0;
        let mut alpha = SCORE_MIN;

        let mut total_nodes = 0;
        let mut total_duration = Duration::ZERO;

        print_search_header("EdaxBot", false, MIDGAME_DEPTH);
        for (i, (move_, child)) in children.iter().enumerate() {
            let start = Instant::now();

            self.eval = Eval::new(child);

            // TODO replace by PVS_midgame() from Edax
            let score = -self.negamax(MIDGAME_DEPTH - 1, -SCORE_MAX, -alpha);
            let duration = start.elapsed();

            print_move_stats(
                self.nodes,
                i,
                children.len(),
                score as isize,
                alpha as isize,
                duration,
            );
            total_nodes += self.nodes;
            total_duration += duration;
            self.nodes = 0;

            if score > alpha {
                alpha = score;
                best_move = *move_;
            }
        }

        print_total_stats(total_nodes, total_duration);

        best_move
    }

    fn negamax(&mut self, depth: u32, mut alpha: i32, beta: i32) -> i32 {
        self.nodes += 1;
        if depth == 0 {
            return self.eval.heuristic();
        }

        let moves = self.eval.position().iter_move_indices();

        // If no moves available
        if moves.is_empty() {
            self.eval.pass();

            // Check if the game is finished
            if !self.eval.position().has_moves() {
                self.eval.pass();
                // Game is over, return final evaluation
                return self.eval.position().final_score() as i32;
            }

            // Recursively evaluate after passing
            let score = -self.negamax(depth - 1, -beta, -alpha);
            self.eval.pass();
            return score;
        }

        for move_ in moves {
            let flipped = self.eval.do_move(move_);
            let score = -self.negamax(depth - 1, -beta, -alpha);
            self.eval.undo_move(move_, flipped);

            alpha = alpha.max(score);

            if alpha >= beta {
                break; // Beta cutoff
            }
        }

        alpha
    }
}

#[cfg(test)]
mod tests {
    use crate::bot::edax::eval::tests::test_positions;

    use super::*;

    #[test]
    fn test_negamax_values() {
        let cases: Vec<(u64, u64, u32, i32)> = vec![
            (0xFFFFFFFFFFFFFFFF, 0, 0, 63),
            (0xFFFFFFFFFFFFFFFF, 0, 1, 64),
            (0xFFFFFFFFFFFFFFFF, 0, 2, 64),
            (0xFFFFFFFFFFFFFFFF, 0, 3, 64),
            (0, 0xFFFFFFFFFFFFFFFF, 0, -63),
            (0, 0xFFFFFFFFFFFFFFFF, 1, -64),
            (0, 0xFFFFFFFFFFFFFFFF, 2, -64),
            (0, 0xFFFFFFFFFFFFFFFF, 3, -64),
            (0x20302000000000, 0x8080818f8202000, 2, 18),
            (0x180400, 0x40301818200000, 2, 19),
            (0x4081000400000, 0x302838200000, 3, -3),
            (0x3c1000000000, 0x8000878280000, 4, 16),
            (0x80401038040000, 0x382840200000, 0, -17),
            (0x40c14400000, 0x225028181000, 1, 13),
            (0x408101a2000, 0x40201708040200, 2, 29),
            (0x1020438100040, 0x301804007010, 3, 19),
            (0x4783010000000, 0xe2c1a0800, 4, 5),
            (0x81d28040200, 0x1402112a0808, 0, -13),
            (0x7010080c0a080000, 0x40c127030000000, 1, -5),
            (0x300800080402, 0x408141814fa20, 2, 45),
            (0xc0d180804000000, 0x1242e61038000000, 3, 63),
            (0x44647c20501c00, 0x805c280000, 4, -40),
            (0x2c400285c0c0400, 0x2030281420400201, 0, -32),
            (0x810000000, 0x1008000000, 0, -4),
            (0x1000000000, 0x81c000000, 1, 4),
            (0x3800000000, 0x38000000, 2, -4),
            (0x10100000, 0x201808080000, 3, 6),
            (0x810040400, 0x100e000000, 4, -6),
            (0x201800000000, 0x2241c000000, 0, 8),
            (0x40200018080000, 0x7c00000000, 1, -16),
            (0x40818000000, 0x703020200000, 2, -2),
            (0x40804200000, 0x87038080000, 3, 1),
            (0x140800, 0x101e1e020000, 4, 11),
            (0x201060060810, 0x40818280000, 0, -15),
            (0x8040b04000000, 0x221419040200, 1, 25),
            (0x10783c180800, 0x10080400200000, 2, -19),
            (0x8060c081c0000, 0x4583030000000, 3, -2),
            (0x1838600808, 0x10180404081420, 4, -1),
            (0x8083002000200, 0x224087c121100, 0, 20),
            (0x40507078140000, 0x80048804281000, 1, -10),
            (0x40c18303008, 0x70b30040c0000, 2, 31),
            (0xe943c4e0000, 0x2020e02800100000, 3, -17),
            (0x80808081cea0000, 0x4024143020001c00, 4, 4),
            (0xa14383020000000, 0x9060e1c284400, 0, 9),
            (0x80828f818040800, 0x400766381000, 1, 2),
            (0x4434040712000000, 0x3918ec1c0800, 2, -12),
            (0x82c0a0e07080808, 0x7010315018040000, 3, 5),
            (0xa0703c08080000, 0x108642f2602038, 4, -5),
            (0x400181004040000, 0x383e26a978280808, 0, 45),
            (0x100004f0183c7000, 0x2077f80c00400000, 1, 14),
            (0x7cb22440f0200400, 0x58b80c580800, 2, -11),
            (0x5c28103800000e, 0x5c20110e041f2840, 3, 26),
            (0x8000f64d88103010, 0x4044081077448404, 4, 16),
            (0x412a746a4f504808, 0x90302ea020, 0, -52),
            (0x3032ec5c3c00, 0x12360f0c12214000, 1, 24),
            (0x40001e3c2a3f0000, 0x3038608054003c52, 2, 6),
            (0x2000076a100d0101, 0x407030140ff22e48, 3, 44),
            (0x20112dfc9c1c0807, 0x4062520262420200, 4, 30),
            (0x207001041e010018, 0xc042cfa614e3f02, 0, 26),
            (0x3a041e0000106838, 0x50607cfea99480, 1, -18),
            (0x9070659614000021, 0x81828e8fe3e4a, 2, 34),
            (0x112a44cbf7773a1c, 0x804323408000400, 3, -25),
            (0x80ea6446a110040, 0xb250183895646e91, 4, -23),
            (0x20373e245c0000, 0xca48c11a237f1f, 0, 19),
            (0x15ef0f2704081504, 0x20020183ad66a78, 1, 29),
            (0x28343f0e06a663ff, 0x70f8581c00, 2, 6),
            (0x400100a603038, 0x80e9ffefb51f0f04, 3, 7),
            (0x64d8c8a48c8cc4e0, 0x9002365870733214, 4, 30),
            (0x404023489a6d4200, 0xbebd5cb644101c1e, 0, 8),
            (0x474727fa04207c08, 0x90a04805b8df8072, 1, 22),
            (0x20383f2f26685c00, 0xe4440d09897227f, 2, -1),
            (0x12561a57c6e0e0fe, 0x8814428391f1f00, 3, 7),
            (0x28727a7c367744, 0xff578d8483418000, 4, -22),
            (0xf3b1498d31013804, 0xc4c3672cefcc0d0, 0, 15),
            (0xf9d8a1816854f940, 0x2275e7e162b0682, 1, 10),
            (0x810000000, 0x1008000000, 0, -4),
            (0x8000000, 0x101810000000, 1, 4),
            (0x80c000000, 0x1010100000, 2, -4),
            (0x1000100000, 0x838080000, 3, 3),
            (0x80810000000, 0x140c040000, 4, -2),
            (0x8080000, 0x1c1810040000, 0, 4),
            (0x38100a0000, 0x8140200, 1, 11),
            (0x180c000000, 0x810780010000000, 2, 9),
            (0x814080000, 0x8040381008040000, 3, -8),
            (0x1c08102010, 0x4030488000, 4, 3),
            (0x2000281c000400, 0x402010001e0000, 0, 6),
            (0x80480c000000, 0x28303030280000, 1, 7),
            (0x403618040200, 0x20104824100000, 2, -23),
            (0x1478001400, 0x1028047c0000, 3, 2),
            (0x1010001034500000, 0x4040782808040000, 4, 23),
            (0xc020100810181000, 0x3040e81008040000, 0, 51),
            (0x80f020300804, 0x2808d8803000, 1, 13),
            (0x400040281e040404, 0x3038301020400000, 2, -14),
            (0x4020002808080808, 0x20183010f4101010, 3, 7),
            (0x10080c0e270000, 0x4020763030400000, 4, 12),
            (0x3c100c040400, 0x804020f107b0800, 0, 15),
            (0x82828b808040200, 0x404f6306080, 1, -18),
            (0xc180c0808080808, 0x224f03416000600, 2, 42),
            (0x502010383400, 0xe86ca85c28000000, 3, -13),
            (0x1012e20808000010, 0xc1434341c7c00, 4, 6),
            (0x803010301000000, 0x41c2e38784e8808, 0, 26),
            (0xe8140904000000, 0xf8140a161b120302, 1, 8),
            (0xf050325080100000, 0x4c0c3e0e0e09, 2, 6),
            (0x8482872e8011010, 0x1002040814fe0700, 3, 7),
            (0x1121010783040, 0x381c29ef0e020400, 4, -12),
            (0x40644da60783020, 0xe048b0241c040200, 0, -34),
            (0x617a3cdc0c1200, 0xe008040322e02400, 1, -37),
            (0x1d3af4b02c042020, 0x20800848d0781008, 2, 13),
            (0x3870628108002000, 0x2041c1e775e9201, 3, 13),
            (0x8007a2af2000000, 0x503704140cfc3e10, 4, 39),
            (0xe46abf1800160000, 0x100040c0fee0d0a0, 0, -19),
            (0xc0620408010060a0, 0x1838777e7f0e02, 1, 25),
            (0x21b6b6548c000, 0x100941ab63f5e, 2, 3),
            (0x4068e3e21343830, 0x81821409c49c60e, 3, 10),
            (0x804021100076e407, 0x7f3e5c287c080808, 4, -12),
            (0x9010281414747400, 0x446a54e8c88808ff, 0, 2),
            (0x353921704f070, 0xe0f0ac6c68f80800, 1, -39),
            (0x8c422312aa0602, 0x3145ced55f97c, 2, 29),
            (0x21100c0a060a4322, 0xccff3f4f8341818, 3, 20),
            (0x49003d1b1f385e80, 0x12afc2c4e0442044, 4, 18),
            (0x38f2ff6fbc2241, 0xffc70d0010010000, 0, -39),
            (0xf0e6c6b743e4004c, 0x81918081c1afd02, 1, 30),
            (0x140804c8542204, 0x1e2be7fb37abd898, 2, 12),
            (0x120c141a153b1f3f, 0x21326ba5ea84e000, 3, 40),
            (0x2330b8d8eb820100, 0x1c8f4526143c5ebe, 4, 7),
            (0x10203ff3277fff, 0x5f241f800cd88000, 0, 29),
            (0x4865318103e1e, 0x3f7078ace7efc0e0, 1, -24),
            (0x7f235978300bbc70, 0x80dca687cff00008, 2, 0),
            (0x3ebf0b270b1f0f, 0x974000f4d8f4e0e0, 3, -12),
            (0xf2f8dd574955231f, 0x8062228b68adce0, 0, 13),
            (0x87fee6a2c240373e, 0x7801195d3d3ec880, 1, 8),
        ];

        for (player, opponent, depth, expected_score) in cases {
            let position = Position::new_from_bitboards(player, opponent);

            let mut search = MidgameSearch::new(position);
            search.eval = Eval::new(&position);

            let score = search.negamax(depth, SCORE_MIN, SCORE_MAX);
            if score != expected_score {
                println!("Position:\n{}", position);
                println!("Depth: {}", depth);
                println!("Negamax: {}", score);
                println!("Expected: {}", expected_score);
                assert!(false);
            }
        }
    }

    fn minimax(position: Position, depth: u32, is_max: bool) -> i32 {
        if depth == 0 {
            let heuristic = Eval::new(&position).heuristic();
            return if is_max { heuristic } else { -heuristic };
        }

        let moves = position.iter_move_indices();

        // If no moves available
        if moves.is_empty() {
            let mut passed = position.clone();
            passed.pass();

            // Check if the game is finished
            if !passed.has_moves() {
                // Game is over, return final evaluation
                let score = position.final_score() as i32;
                return if is_max { score } else { -score };
            }

            // Recursively evaluate after passing
            return minimax(passed, depth - 1, !is_max);
        }

        let mut best;

        if is_max {
            best = SCORE_MIN;
            for move_ in moves {
                let mut child = position.clone();
                child.do_move(move_);
                let score = minimax(child, depth - 1, !is_max);
                best = best.max(score);
            }
        } else {
            best = SCORE_MAX;
            for move_ in moves {
                let mut child = position.clone();
                child.do_move(move_);
                let score = minimax(child, depth - 1, !is_max);
                best = best.min(score);
            }
        }
        best
    }

    #[test]
    fn test_negamax() {
        for position in test_positions() {
            for depth in 0..=3 {
                // Initialize search state, set position in `eval`
                let mut search = MidgameSearch::new(position);
                search.eval = Eval::new(&position);

                let negamax = search.negamax(depth, SCORE_MIN, SCORE_MAX);

                let minimax = minimax(position, depth, true);

                if negamax != minimax {
                    println!("Position:\n{}", position);
                    println!("Negamax: {}", negamax);
                    println!("Minimax: {}", minimax);
                    assert!(false);
                }
            }
        }
    }
}
