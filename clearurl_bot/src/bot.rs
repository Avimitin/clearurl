use teloxide::{dispatching2::UpdateFilterExt, prelude2::*, types::Update};
use log::{error, info};

#[allow(dead_code)]
async fn run() {
    teloxide::enable_logging!();
    log::info!("Starting bot...");

    let bot_config = Config {
        token: String::from("test token"),
        enable_groups: vec![123456, 7891011],
    };

    let bot = Bot::new(&bot_config.token).auto_send();
    let http_regex_rule = Box::new(regex::Regex::new(
        r"(http[s]?://(?:[a-zA-Z]|[0-9]|[$-_@.&+]|[!*\(\),]|(?:%[0-9a-fA-F][0-9a-fA-F]))+)",
    )
    .expect("Illegal http regex rule"));

    let update_handler = Update::filter_message().branch(
        dptree::filter(|msg: &Message, cfg: &Config| {
            msg.text().is_some() && cfg.is_enabled_group(msg.chat_id())
        })
        .endpoint(
            |msg: Message, bot: AutoSend<Bot>, cleaner: clearurl::UrlCleaner, regex: regex::Regex | async move {
                let caps = regex.captures_iter(msg.text().unwrap_or(""));
                let mut buffer = String::new();
                for cap in caps {
                    let url = &cap[1];
                    let url = match cleaner.clear(url).await {
                        Ok(u) => u,
                        Err(e) => {
                            error!("{}", e);
                            return respond(());
                        }
                    };
                    buffer.push_str(url.as_str());
                    buffer.push('\n');
                }

                if !buffer.is_empty() {
                    let resp = bot.send_message(msg.chat_id(), buffer).await;
                    if let Err(e) = resp {
                        error!("{}", e);
                    }
                }
                respond(())
            },
        ),
    );

    Dispatcher::builder(bot, update_handler)
        .dependencies(dptree::deps![bot_config, http_regex_rule])
        .default_handler(|upd| async move {
            log::warn!("Unhandle update from chat {}", upd.chat().unwrap().id)
        })
        .error_handler(LoggingErrorHandler::with_custom_text("Fail to handle"))
        .build()
        .setup_ctrlc_handler()
        .dispatch()
        .await;
}

struct Config {
    token: String,
    enable_groups: Vec<i64>,
}

impl Config {
    pub fn is_enabled_group(&self, g: i64) -> bool {
        for group in &self.enable_groups {
            if g == *group {
                return true;
            }
        }

        false
    }
}
