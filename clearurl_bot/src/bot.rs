use anyhow::{Context, Result, bail};
use clearurl::UrlCleaner;
use std::env;
use std::sync::Arc;
use teloxide::{dispatching2::UpdateFilterExt, prelude2::*, types::Update, RequestError};

#[derive(Clone)]
struct Config {
    enable_groups: Arc<Vec<i64>>,
}

impl Config {
    pub fn is_enabled_group(&self, g: i64) -> bool {
        for group in self.enable_groups.iter() {
            if g == *group {
                return true;
            }
        }

        false
    }
}

async fn parse_links(input: &str, regex: &regex::Regex, cleaner: &UrlCleaner) -> Result<String> {
    let caps = regex.captures_iter(input);
    let mut buffer = String::new();
    for cap in caps {
        // Get the first capture
        let orig_url = &cap[1];
        let url = cleaner.clear(orig_url).await?;

        // If the final result is as same as the input
        if url.as_str() == orig_url {
            continue;
        }

        buffer.push_str(url.as_str());
        buffer.push('\n');
    }

    if buffer.is_empty() {
        bail!("No rule matches");
    }

    Ok(buffer)
}

#[tokio::test]
async fn test_parse_link() {
    // rick roll
    let input = "https://www.bilibili.com/video/av928861104";
    let regex = regex::Regex::new(r"(http[s]?://(?:[a-zA-Z]|[0-9]|[$-_@.&+]|[!*\(\),]|(?:%[0-9a-fA-F][0-9a-fA-F]))+)").unwrap();
    let cleaner = UrlCleaner::from_file("../rules.toml").unwrap();
    let link = parse_links(input, &regex, &cleaner).await;

    // it should return empty string
    assert!(link.is_err());

    // hit red
    let input = "https://b23.tv/YfzhsWH";
    let link = parse_links(input, &regex, &cleaner).await.unwrap();

    // It should return expected string
    assert_eq!(link.as_str(), "https://www.bilibili.com/video/BV1vZ4y1Z7Y7?p=1\n");
}

async fn handle_link_message(
    msg: Message,
    bot: AutoSend<Bot>,
    cleaner: Arc<UrlCleaner>,
    regex: Arc<regex::Regex>,
) -> Result<(), RequestError> {
    let resp_text = parse_links(msg.text().unwrap_or(""), &regex, &cleaner).await;
    // Error are prompt when parse fail, or no rule matches
    // This happen a lot when the bot is handling in a large group
    // So we just throw those error.
    if let Ok(resp) = resp_text {
        bot.send_message(msg.chat_id(), resp).await?;
    }
    respond(())
}

fn build_runtime() -> (AutoSend<Bot>, Arc<UrlCleaner>, Arc<regex::Regex>) {
    let clearurl_file_path =
        env::var("CLEARURL_FILE").unwrap_or_else(|_| String::from("./rules.toml"));
    let bot = Bot::from_env().auto_send();
    let cleaner = Arc::new(UrlCleaner::from_file(&clearurl_file_path).unwrap());
    let http_regex_rule = Arc::new(
        regex::Regex::new(
            r"(http[s]?://(?:[a-zA-Z]|[0-9]|[$-_@.&+]|[!*\(\),]|(?:%[0-9a-fA-F][0-9a-fA-F]))+)",
        )
        .expect("Illegal http regex rule"),
    );

    (bot, cleaner, http_regex_rule)
}

fn build_handler() -> Handler<'static, DependencyMap, Result<(), RequestError>> {
    Update::filter_message().branch(
        dptree::filter(|msg: Message, cfg: Config| {
            msg.text().is_some() && cfg.is_enabled_group(msg.chat_id())
        })
        .endpoint(handle_link_message),
    )
}

pub async fn run() -> Result<()> {
    teloxide::enable_logging!();
    dotenv::dotenv().ok();

    let groups = env::var("CLBOT_ENABLE_GROUPS").with_context(|| "You must setup enable groups")?;
    let groups: Vec<i64> = groups
        .split(',')
        .map(|x| x.parse::<i64>()
            .with_context(|| format!(
                    "Fail to parse group `{}` to int64, please check your $CLBOT_ENABLE_GROUPS variable.", x))
            .unwrap())
        .collect();

    log::info!("Enabled groups: {:?}", groups);

    let bot_config = Config {
        enable_groups: Arc::new(groups),
    };

    let (bot, cleaner, http_regex_rule) = build_runtime();

    log::info!("Loaded URL rules: {}", cleaner.amount());
    log::info!(
        "Starting bot: {}",
        bot.get_me()
            .await
            .with_context(|| "fail to get bot information, please check your token correctness.")?
            .user
            .first_name
    );

    Dispatcher::builder(bot, build_handler())
        .dependencies(dptree::deps![bot_config, http_regex_rule, cleaner])
        .default_handler(|_| async move {})
        .error_handler(LoggingErrorHandler::with_custom_text("Fail to handle"))
        .build()
        .setup_ctrlc_handler()
        .dispatch()
        .await;

    Ok(())
}
