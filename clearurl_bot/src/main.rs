use clearurl::data;
use clearurl::filter;
use lazy_static::lazy_static;
use log::{error, info};
use teloxide::prelude2::*;

lazy_static! {
    static ref HTTP_REGEX_MATCH_RULE: regex::Regex = regex::Regex::new(
        r"(http[s]?://(?:[a-zA-Z]|[0-9]|[$-_@.&+]|[!*\(\),]|(?:%[0-9a-fA-F][0-9a-fA-F]))+)",
    )
    .unwrap();
    // TODO: use portable path for rules file
    static ref RULES:data::Domains = data::Domains::load_from_file("../rules.toml").unwrap();
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    teloxide::enable_logging!();
    info!("Starting clearurl_bot...");

    let bot = Bot::from_env().auto_send();

    teloxide::repls2::repl(bot, |message: Message, bot: AutoSend<Bot>| async move {
        let text = message.text().unwrap_or("");
        let capture = filter_domain(text);
        let mut buffer = String::new();
        for cap in capture {
            let mut url = match url::Url::parse(&cap[1]) {
                Ok(u) => u,
                Err(e) => {
                    error!("Fail to parse url: {} from capture. Error: {}", &cap[1], e);
                    return respond(());
                }
            };
            let url = match filter::clear(&RULES, &mut url).await {
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
            let resp = bot.send_message(message.chat_id(), buffer).await;
            if let Err(e) = resp {
                error!("{}", e);
            }
        }
        respond(())
    })
    .await;
}

fn filter_domain(text: &str) -> regex::CaptureMatches {
    HTTP_REGEX_MATCH_RULE.captures_iter(text)
}
