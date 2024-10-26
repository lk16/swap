use position::Position;

pub mod board;
pub mod position;

fn main() {
    let board = Position::new();
    println!("{}", board);
}
