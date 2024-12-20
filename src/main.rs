use frontend::app::run_app;

pub mod bot;
pub mod frontend;
pub mod othello;

#[tokio::main]
async fn main() {
    run_app().await;
}
