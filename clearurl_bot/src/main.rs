mod bot;
mod utils;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    bot::run().await
}
