use anyhow::{bail, Context, Result};
use clearurl::UrlCleaner;
use std::env;
use std::sync::{Arc, Mutex};
use teloxide::{
    dispatching2::UpdateFilterExt, prelude2::*, types::Update, utils::command::BotCommand,
    RequestError,
};

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

#[derive(Clone, Debug)]
struct BotRuntime {
    // TODO: Use chrono to record uptime
    total_url_met: Arc<Mutex<u32>>,
    total_cleared: Arc<Mutex<u32>>,
}

#[derive(BotCommand, Clone)]
#[command(rename = "lowercase", description = "Clearurl Bot Commands")]
enum Commands {
    #[command(description = "Show this message")]
    Help,
    #[command(description = "Show bot stats")]
    Stats,
}

async fn parse_links(
    input: &str,
    regex: &regex::Regex,
    cleaner: &UrlCleaner,
    ctx: &mut BotRuntime,
) -> Result<String> {
    let caps = regex.captures_iter(input);
    let mut buffer = String::from("Clean URL: \n");
    let mut indeed_cleared = false;
    for cap in caps {
        // Get the first capture
        let orig_url = &cap[1];
        let url = cleaner.clear(orig_url).await?;

        // we have met a new url, so we add a counter here
        let mut met = ctx.total_url_met.lock().unwrap();
        *met += 1;

        // If the final result is as same as the input
        if url.as_str() == orig_url {
            continue;
        } else {
            // we actually clear this url
            indeed_cleared = true;
        }

        buffer.push_str(url.as_str());
        buffer.push('\n');

        // we have cleared a url, so we add a counter here
        let mut cleared = ctx.total_cleared.lock().unwrap();
        *cleared += 1;
    }

    // if we didn't do anything to the url
    if !indeed_cleared {
        bail!("No rule matches");
    }

    Ok(buffer)
}

#[tokio::test]
async fn test_parse_link() {
    let regex = regex::Regex::new(
        r"(http[s]?://(?:[a-zA-Z]|[0-9]|[$-_@.&+]|[!*\(\),]|(?:%[0-9a-fA-F][0-9a-fA-F]))+)",
    )
    .unwrap();
    let cleaner = UrlCleaner::from_file("../rules.toml").unwrap();
    let mut rt = BotRuntime {
        total_url_met: Arc::new(Mutex::new(0)),
        total_cleared: Arc::new(Mutex::new(0)),
    };

    // rick roll
    let input = "https://www.bilibili.com/video/av928861104";
    let link = parse_links(input, &regex, &cleaner, &mut rt).await;

    // it should return empty string
    assert!(link.is_err());

    // lock the runtime
    {
        let cleared = rt.total_cleared.lock().unwrap();
        let met = rt.total_url_met.lock().unwrap();
        assert_eq!(*met, 1);
        assert_eq!(*cleared, 0);
    } // release the runtime

    // hit red
    let input = "https://b23.tv/YfzhsWH";
    let link = parse_links(input, &regex, &cleaner, &mut rt).await.unwrap();

    // It should return expected string
    assert_eq!(
        link.as_str(),
        "Clean URL: \nhttps://www.bilibili.com/video/BV1vZ4y1Z7Y7?p=1\n"
    );

    // lock the runtime
    {
        let cleared = rt.total_cleared.lock().unwrap();
        let met = rt.total_url_met.lock().unwrap();
        assert_eq!(*met, 2);
        assert_eq!(*cleared, 1);
    } // release the runtime
}

async fn handle_link_message(
    msg: Message,
    bot: AutoSend<Bot>,
    cleaner: Arc<UrlCleaner>,
    regex: Arc<regex::Regex>,
    mut rt: BotRuntime,
) -> Result<(), RequestError> {
    let resp_text = parse_links(msg.text().unwrap_or(""), &regex, &cleaner, &mut rt).await;
    // Error are prompt when parse fail, or no rule matches
    // This happen a lot when the bot is handling in a large group
    // So we just throw those error.
    if let Ok(resp) = resp_text {
        bot.send_message(msg.chat_id(), resp)
            .disable_web_page_preview(true) // no need for the preview, it is annoying
            .await?;
    }
    respond(())
}

async fn handle_commands(
    msg: Message,
    bot: AutoSend<Bot>,
    cmd: Commands,
    ctx: BotRuntime,
) -> Result<(), RequestError> {
    let text = match cmd {
        Commands::Stats => {
            let met = ctx.total_url_met.lock().unwrap();
            let cleared = ctx.total_cleared.lock().unwrap();
            let ratio: f32 = (*cleared as f32 / *met as f32) * 100.0;
            format!(
                "Total URL Met: {}\nTotal URL Cleared: {}\nPercentage: {} %",
                *met, *cleared, ratio
            )
        }
        Commands::Help => Commands::descriptions(),
    };

    bot.send_message(msg.chat_id(), text).await?;

    respond(())
}

fn build_runtime() -> (
    AutoSend<Bot>,
    Arc<UrlCleaner>,
    Arc<regex::Regex>,
    BotRuntime,
) {
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

    let rt = BotRuntime {
        total_url_met: Arc::new(Mutex::new(0)),
        total_cleared: Arc::new(Mutex::new(0)),
    };

    (bot, cleaner, http_regex_rule, rt)
}

fn build_handler() -> Handler<'static, DependencyMap, Result<(), RequestError>> {
    Update::filter_message()
        .branch(
            dptree::entry()
                .filter_command::<Commands>()
                .endpoint(handle_commands),
        )
        .branch(
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

    let (bot, cleaner, http_regex_rule, rt) = build_runtime();

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
        .dependencies(dptree::deps![bot_config, http_regex_rule, cleaner, rt])
        .default_handler(|_| async move {})
        .error_handler(LoggingErrorHandler::with_custom_text("Fail to handle"))
        .build()
        .setup_ctrlc_handler()
        .dispatch()
        .await;

    Ok(())
}
