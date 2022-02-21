use teloxide::{dispatching2::{MessageFilterExt, UpdateFilterExt}, prelude2::*, types::Update};

#[allow(dead_code)]
async fn run() {
    teloxide::enable_logging!();
    log::info!("Starting bot...");

    let bot_config = Config {
        token: String::from("test token"),
        enable_groups: vec![123456, 7891011],
    };

    let bot = Bot::new(&bot_config.token).auto_send();

    let update_handler = Update::filter_message()
        .branch(
            Message::filter_text().endpoint(|| {

            })
            );
}

struct Config {
    token: String,
    enable_groups: Vec<i64>,
}
