mod bot;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    bot::run().await
}
