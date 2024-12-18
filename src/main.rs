use frontend::app::run_app;

pub mod bot;
pub mod collections;
pub mod frontend;
pub mod othello;

/// Entry point for the web app.
#[tokio::main]
async fn main() {
    run_app().await;
}
