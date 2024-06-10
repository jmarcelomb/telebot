use crate::constants::JMARCELOMB_RECIPIENT;
use teloxide::prelude::*;

pub async fn send_message(msg: &str) -> Message {
    log::info!("Sending message: '{}'", msg);
    let bot = Bot::from_env();
    bot.send_message(JMARCELOMB_RECIPIENT.clone(), msg)
        .await
        .unwrap()
}
