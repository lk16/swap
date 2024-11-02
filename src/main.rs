use bot::edax::eval::EVAL_X2F;
use frontend::app::run_app;
use std::env;

pub mod bot;
pub mod frontend;
pub mod othello;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 && args[1] == "--dump" {
        dump_constants();
        return;
    }

    run_app().await;
}

fn dump_constants() {
    use bot::edax::eval::{EVAL_F2X, EVAL_MAX_VALUE, EVAL_N_FEATURES, EVAL_OFFSET};
    use bot::edax::weights::{EVAL_PACKED_SIZE, EVAL_SIZE, EVAL_WEIGHT};

    println!();

    // Dump EVAL_SIZE
    println!("=== EVAL_SIZE ===");
    for (i, &size) in EVAL_SIZE.iter().enumerate() {
        println!("[ {:2}] = {}", i, size);
    }
    println!();

    // Dump EVAL_PACKED_SIZE
    println!("=== EVAL_PACKED_SIZE ===");
    for (i, &size) in EVAL_PACKED_SIZE.iter().enumerate() {
        println!("[ {:2}] = {}", i, size);
    }
    println!();

    // Dump EVAL_OFFSET
    println!("=== EVAL_OFFSET ===");
    for (i, &offset) in EVAL_OFFSET.iter().enumerate() {
        println!("[ {:2}] = {}", i, offset);
    }
    println!();

    // Dump EVAL_MAX_VALUE
    println!("=== EVAL_MAX_VALUE ===");
    for (i, &max_value) in EVAL_MAX_VALUE.iter().enumerate() {
        println!("[ {:2}] = {}", i, max_value);
    }
    println!();

    // Dump EVAL_F2X
    println!("=== EVAL_F2X ===");
    for i in 0..EVAL_N_FEATURES {
        if EVAL_F2X[i].is_empty() {
            break;
        }

        println!(
            "[ {:2}] n_square={}, squares=[{}]",
            i,
            EVAL_F2X[i].len(),
            EVAL_F2X[i]
                .iter()
                .map(|&x| x.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
    }
    println!();

    // Dump EVAL_X2F
    println!("=== EVAL_X2F ===");
    for i in 0..64 {
        println!(
            "[ {:2}] n_feature={}, features=[{}]",
            i,
            EVAL_X2F[i].len(),
            EVAL_X2F[i]
                .iter()
                .map(|(idx, val)| format!("(i:{},x:{})", idx, val))
                .collect::<Vec<_>>()
                .join(",")
        );
    }
    println!();

    // Dump weights
    println!("=== EVAL_WEIGHT ===");
    for ply in 0..61 {
        println!("PLY {:2} (first 10 values):", ply);
        println!(
            "  Player 0: {:6} {:6} {:6} {:6} {:6} {:6} {:6} {:6} {:6} {:6} ...",
            EVAL_WEIGHT[0][ply][0],
            EVAL_WEIGHT[0][ply][1],
            EVAL_WEIGHT[0][ply][2],
            EVAL_WEIGHT[0][ply][3],
            EVAL_WEIGHT[0][ply][4],
            EVAL_WEIGHT[0][ply][5],
            EVAL_WEIGHT[0][ply][6],
            EVAL_WEIGHT[0][ply][7],
            EVAL_WEIGHT[0][ply][8],
            EVAL_WEIGHT[0][ply][9]
        );
        println!(
            "  Player 1: {:6} {:6} {:6} {:6} {:6} {:6} {:6} {:6} {:6} {:6} ...",
            EVAL_WEIGHT[1][ply][0],
            EVAL_WEIGHT[1][ply][1],
            EVAL_WEIGHT[1][ply][2],
            EVAL_WEIGHT[1][ply][3],
            EVAL_WEIGHT[1][ply][4],
            EVAL_WEIGHT[1][ply][5],
            EVAL_WEIGHT[1][ply][6],
            EVAL_WEIGHT[1][ply][7],
            EVAL_WEIGHT[1][ply][8],
            EVAL_WEIGHT[1][ply][9]
        );
    }
}
