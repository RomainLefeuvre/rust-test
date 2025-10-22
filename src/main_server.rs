mod graph;
mod origin;
mod server;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    server::create_server().await
}
