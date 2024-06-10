use once_cell::sync::Lazy;
use teloxide::types::{ChatId, Recipient};

pub static JMARCELOMB_CHAT_ID: Lazy<ChatId> = Lazy::new(|| {
    let chat_id: i64 = std::env::var("JMARCELOMB_CHAT_ID")
        .ok()
        .unwrap()
        .parse()
        .unwrap();
    ChatId(chat_id)
});

pub static JMARCELOMB_RECIPIENT: Lazy<Recipient> = Lazy::new(|| Recipient::Id(*JMARCELOMB_CHAT_ID));
