use board::Board;

pub mod board;

fn main() {
    let board = Board::new();
    board.print(&mut std::io::stdout(), false).unwrap();
}
