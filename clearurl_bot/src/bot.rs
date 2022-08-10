use anyhow::{Context, Result};
use clearurl::UrlCleaner;
use std::env;
use std::sync::{Arc, Mutex};
use teloxide::types::{InlineQueryResult, InlineQueryResultArticle, InputMessageContentText};
use teloxide::{
    dispatching::UpdateFilterExt, prelude::*, types::Update, utils::command::BotCommands,
};

use crate::utils;

// Config store the necessary configuration for bot runtime.
#[derive(Clone)]
struct Config {
    // Bot will start the filter process only in the enabled groups
    enable_groups: Arc<Vec<i64>>,
}

impl Config {
    /// Return true if the given group is allow to be used
    pub fn is_enabled_group(&self, g: i64) -> bool {
        for group in self.enable_groups.iter() {
            if g == *group {
                return true;
            }
        }

        false
    }
}

type BotRuntime = Arc<Mutex<RuntimeInner>>;

// BotRuntime store some statistic for the clearurl process.
#[derive(Clone, Debug)]
struct RuntimeInner {
    // When did the bot start
    start_up_time: chrono::DateTime<chrono::Utc>,
    // How many url bot has met
    total_url_met: u32,
    // How many url bot has cleared
    total_cleared: u32,
}

impl std::default::Default for RuntimeInner {
    fn default() -> Self {
        Self {
            start_up_time: chrono::Utc::now(),
            total_url_met: 0,
            total_cleared: 0,
        }
    }
}

#[derive(BotCommands, Clone)]
#[command(rename = "lowercase", description = "Clearurl Bot Commands")]
enum Commands {
    #[command(description = "Show this message")]
    Help,
    #[command(description = "Show bot stats")]
    Stats,
}

async fn handle_link_message(
    msg: Message,
    bot: AutoSend<Bot>,
    cleaner: Arc<UrlCleaner>,
    rt: BotRuntime,
) -> Result<()> {
    // silently exit when we met message with no text (might be sticker, video...)
    if msg.text().is_none() {
        return Ok(());
    }

    let response = utils::clean(msg.text().unwrap(), &cleaner).await?;

    let text = response
        .data
        .iter()
        .fold("Cleared url:".to_string(), |sum, x| format!("{sum}\n* {x}"));
    bot.send_message(msg.chat.id, text)
        // enable preview because sometime user's client might fail to load preview
        .disable_web_page_preview(false)
        .await?;

    // update counter
    let mut rt = rt.lock().unwrap();
    rt.total_url_met += response.met;
    rt.total_cleared += response.cleaned;
    Ok(())
}

async fn handle_commands(
    msg: Message,
    bot: AutoSend<Bot>,
    cmd: Commands,
    ctx: BotRuntime,
) -> Result<()> {
    let text = match cmd {
        Commands::Stats => {
            let rt = ctx.lock().unwrap();
            let met = rt.total_url_met;
            let cleared = rt.total_cleared;
            let start_up = rt.start_up_time;
            drop(rt); // early drop to avoid long wait

            let ratio: f32 = if met == 0 {
                0.0
            } else {
                (cleared as f32 / met as f32) * 100.0
            };

            let now = chrono::Utc::now();
            let duration = now.signed_duration_since(start_up);

            format!(
                "Bot Uptime: {}h {}m {}s\nTotal URL Met: {}\nTotal URL Cleared: {}\nPercentage: {} %",
                duration.num_hours(), duration.num_minutes() % 60, duration.num_seconds() % 60,
                met, cleared, ratio
            )
        }
        Commands::Help => Commands::descriptions().to_string(),
    };

    bot.send_message(msg.chat.id, text).await?;

    Ok(())
}

async fn inline_handler(
    query: InlineQuery,
    bot: AutoSend<Bot>,
    cleaner: Arc<UrlCleaner>,
) -> Result<()> {
    let response = utils::replace(&query.query, &cleaner);
    if response.is_err() {
        anyhow::bail!("no link founded");
    }

    let response = InlineQueryResultArticle::new(
        "clearurl_111".to_string(),
        "URL Cleaner",
        teloxide::types::InputMessageContent::Text(InputMessageContentText::new(response.unwrap())),
    )
    .description("Automatically clean and replace the URL in your text.");
    let response = vec![InlineQueryResult::Article(response)];
    bot.answer_inline_query(&query.id, response).await?;

    Ok(())
}

pub async fn run() -> Result<()> {
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

    let bot = Bot::from_env().auto_send();

    let clearurl_file_path =
        env::var("CLEARURL_FILE").unwrap_or_else(|_| String::from("./rules.toml"));
    let cleaner = Arc::new(UrlCleaner::from_file(&clearurl_file_path).unwrap());

    let rt = BotRuntime::new(Mutex::new(RuntimeInner::default()));

    log::info!(
        "Starting bot: {}",
        bot.get_me()
            .await
            .with_context(|| "fail to get bot information, please check your token correctness.")?
            .user
            .first_name
    );

    let msg_handler = Update::filter_message()
        .branch(
            dptree::entry()
                .filter_command::<Commands>()
                .endpoint(handle_commands),
        )
        .branch(
            dptree::filter(|msg: Message, cfg: Config| {
                msg.text().is_some() && cfg.is_enabled_group(msg.chat.id.0)
            })
            .endpoint(handle_link_message),
        );
    let inline_handler = Update::filter_inline_query().endpoint(inline_handler);
    let root = dptree::entry();
    let root = root.branch(msg_handler).branch(inline_handler);

    Dispatcher::builder(bot, root)
        .dependencies(dptree::deps![bot_config, cleaner, rt])
        .default_handler(|_| async move {})
        .error_handler(LoggingErrorHandler::with_custom_text("Fail to handle"))
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}
