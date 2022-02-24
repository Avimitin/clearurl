mod bot;

#[tokio::main]
async fn main() {
    bot::run().await
}
