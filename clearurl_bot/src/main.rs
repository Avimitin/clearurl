use dotenv;
use lazy_static::lazy_static;
use teloxide::prelude2::*;
use clearurl::filter;
use clearurl::data;

lazy_static! {
    static ref HTTP_REGEX_MATCH_RULE: regex::Regex = regex::Regex::new(
        r"(http[s]?://(?:[a-zA-Z]|[0-9]|[$-_@.&+]|[!*\(\),]|(?:%[0-9a-fA-F][0-9a-fA-F]))+)",
    )
    .unwrap();
    static ref RULES:data::Domains = data::Domains::load_from_file("../rules.toml").unwrap();
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    teloxide::enable_logging!();
    log::info!("Starting dices_bot...");

    let bot = Bot::from_env().auto_send();

    teloxide::repls2::repl(bot, |message: Message, bot: AutoSend<Bot>| async move {
        let text = message.text().unwrap_or("");
        let capture = filter_domain(text);
        let mut buffer = String::new();
        for cap in capture {
            let mut url = url::Url::parse(&cap[1]).unwrap();
            let url = filter::filter(&RULES, &mut url).unwrap();
            buffer.push_str(url.as_str());
        }

        if !buffer.is_empty() {
            bot.send_message(message.chat_id(), buffer).await.unwrap();
        }
        respond(())
    })
    .await;
}

fn filter_domain(text: &str) -> regex::CaptureMatches {
    HTTP_REGEX_MATCH_RULE.captures_iter(text)
}
