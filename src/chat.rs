use crate::constants::JMARCELOMB_RECIPIENT;
use teloxide::prelude::*;
use teloxide::RequestError;

pub async fn send_message(msg: &str) -> Result<Message, RequestError> {
    log::info!("Sending message: {}", msg);
    let bot = Bot::from_env();
    let message = bot.send_message(JMARCELOMB_RECIPIENT.clone(), msg).await?;

    Ok(message)
}
