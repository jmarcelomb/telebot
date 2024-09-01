pub mod chat;
pub mod constants;
pub mod db;
mod milk_price;
pub mod services;

use services::Services;
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock as AsyncRwLock;
use tokio::time::Duration;

use regex::Regex;

use std::error::Error;
use teloxide::{
    payloads::SendMessageSetters,
    prelude::*,
    types::{
        InlineKeyboardButton, InlineKeyboardMarkup, InlineQueryResultArticle, InputMessageContent,
        InputMessageContentText, Me,
    },
    utils::command::BotCommands,
};

use teloxide::dispatching::UpdateHandler;

type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

const MILK_URL: &str =
    "https://www.continente.pt/produto/leite-proteina-sem-lactose-mimosa-7652960.html";
// const FOUR_HOURS_IN_SECONDS: u64 = 60 * 60 * 4;
const FOUR_HOURS_IN_SECONDS: u64 = 15;

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "display current application version.")]
    Version,
    #[command(description = "List available services, use ls command.")]
    List,
    #[command(description = "Query current mimosa milk price in Continente.")]
    MilkPrice,
}

fn get_services() -> &'static Arc<AsyncRwLock<Services>> {
    static SERVICES: OnceLock<Arc<AsyncRwLock<Services>>> = OnceLock::new();
    SERVICES.get_or_init(|| Arc::new(AsyncRwLock::new(Services::new())))
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    pretty_env_logger::init();

    db::init().await;

    log::info!("Starting purchase bot...");

    let bot = Bot::from_env();

    {
        let services = get_services();
        {
            let mut services_write = services.write().await;
            let milk_work_mng =
                services::WorkerManagement::new(false, Duration::from_secs(FOUR_HOURS_IN_SECONDS));
            services_write
                .create_service("mimosa_milk".to_string(), milk_work_mng, async {
                    milk_price::price_periodically_checker_thread("mimosa_milk", MILK_URL).await
                })
                .await;
        }
    }

    Dispatcher::builder(bot, schema())
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    let command_handler = dptree::entry()
        .branch(Update::filter_message().endpoint(message_handler))
        .branch(Update::filter_callback_query().endpoint(callback_handler))
        .branch(Update::filter_inline_query().endpoint(inline_query_handler));
    command_handler
}

async fn help(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .await?;
    Ok(())
}

async fn version(bot: Bot, msg: Message) -> HandlerResult {
    let version = env!("CARGO_PKG_VERSION");
    bot.send_message(msg.chat.id, format!("Current version is: {} 🏷️", version))
        .await?;
    Ok(())
}

async fn list(bot: Bot, msg: Message) -> HandlerResult {
    let keyboard = make_keyboard().await;
    bot.send_message(msg.chat.id, "Services:")
        .reply_markup(keyboard)
        .await?;
    Ok(())
}

async fn make_keyboard() -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];
    let mut services_list = vec![];
    let services_guard = get_services().read().await;
    for service_guard in services_guard.services.iter() {
        let service = service_guard.lock().await;
        services_list.push(format!(
            "[{}] {}: {}",
            &service.id,
            &service.name,
            if service.enable { "on" } else { "off" }
        ));
    }
    services_list.push("Exit".to_string());

    for service_chunk in services_list.chunks(3) {
        let row = service_chunk
            .iter()
            .map(|service| InlineKeyboardButton::callback(service.to_owned(), service.to_owned()))
            .collect();

        keyboard.push(row);
    }

    InlineKeyboardMarkup::new(keyboard)
}
async fn milk_price_command(bot: Bot, msg: Message) -> HandlerResult {
    let milk_price = milk_price::get_price(MILK_URL).await.unwrap();

    match milk_price {
        Some(price) => {
            bot.send_message(msg.chat.id, format!("Current milk price is: {} €", price))
                .await?;
            Ok(())
        }
        None => Ok(()),
    }
}

async fn inline_query_handler(
    bot: Bot,
    q: InlineQuery,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let choose_debian_version = InlineQueryResultArticle::new(
        "0",
        "Chose debian version",
        InputMessageContent::Text(InputMessageContentText::new("Debian versions:")),
    )
    .reply_markup(make_keyboard().await);

    bot.answer_inline_query(q.id, vec![choose_debian_version.into()])
        .await?;

    Ok(())
}

/// When it receives a callback from a button it edits the message with all
/// those buttons writing a text with the selected Debian version.
///
/// **IMPORTANT**: do not send privacy-sensitive data this way!!!
/// Anyone can read data stored in the callback button.
async fn callback_handler(bot: Bot, q: CallbackQuery) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(service_string) = q.data {
        let re = Regex::new(r"\[([0-9+])\] (.+): (.+)").unwrap();

        if let Some(captures) = re.captures(&service_string) {
            let id = captures.get(1).map_or("", |m| m.as_str());
            let service_name = captures.get(2).map_or("", |m| m.as_str());
            let state_str = captures.get(3).map_or("", |m| m.as_str());
            let state_bool = if state_str == "on" { true } else { false };

            let text = format!(
                "Service '{service_name}' ({id}) was {}",
                if state_bool == true {
                    "disable"
                } else {
                    "enable"
                }
            );
            {
                let service_guard;
                {
                    let services = get_services().write().await;
                    service_guard = services.get_service(&service_name).await;
                }
                if let Some(service_guard) = service_guard {
                    let mut service = service_guard.lock().await;
                    service.set_enable_state(!state_bool).await;
                }
            }
            // Tell telegram that we've seen this query, to remove 🕑 icons from the
            // clients. You could also use `answer_callback_query`'s optional
            // parameters to tweak what happens on the client side.
            bot.answer_callback_query(q.id).await?;

            // Edit text of the message to which the buttons were attached
            if let Some(Message { id, chat, .. }) = q.message {
                bot.edit_message_text(chat.id, id, text).await?;
            } else if let Some(id) = q.inline_message_id {
                bot.edit_message_text_inline(id, text).await?;
            }
            log::info!("You chose: {}", service_string);
        }
    }

    Ok(())
}

async fn message_handler(
    bot: Bot,
    msg: Message,
    me: Me,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(text) = msg.text() {
        match BotCommands::parse(text, me.username()) {
            Ok(Command::Help) => help(bot, msg).await?,
            Ok(Command::Version) => version(bot, msg).await?,
            Ok(Command::List) => list(bot, msg).await?,
            Ok(Command::MilkPrice) => milk_price_command(bot, msg).await?,
            Err(_) => {
                bot.send_message(msg.chat.id, "Command not found!").await?;
            }
        }
    }
    Ok(())
}
