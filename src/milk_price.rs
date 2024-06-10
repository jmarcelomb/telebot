use reqwest;
use scraper::{Html, Selector};
use thiserror::Error;
use tokio::time::{sleep, Duration};

use crate::chat;

#[derive(Error, Debug)]
pub enum PriceError {
    #[error("Request failed with status: {0}")]
    RequestFailed(reqwest::StatusCode),
    #[error("Failed to parse HTML")]
    HtmlParseError,
    #[error("Failed to parse price: {0}")]
    PriceParseError(#[from] std::num::ParseFloatError),
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
}
pub async fn get_price(url: &str) -> Result<Option<f32>, PriceError> {
    let response = reqwest::get(url).await?;

    if !response.status().is_success() {
        return Err(PriceError::RequestFailed(response.status()));
    }

    let body = response.text().await?;
    let document = Html::parse_document(&body);
    let selector =
        Selector::parse("span.ct-price-formatted").map_err(|_| PriceError::HtmlParseError)?;

    for element in document.select(&selector) {
        let text = element.text().collect::<Vec<_>>().join("");
        let text = text.trim();
        let price_as_float = text
            .trim_start_matches('€')
            .replace(",", ".")
            .parse::<f32>()?;
        return Ok(Some(price_as_float));
    }

    Ok(None)
}

pub async fn price_periodically_checker_thread(url: &str, interval: u64) {
    log::info!(
        "price_periodically_checker_thread started running with interval {}",
        interval
    );

    let mut last_price = if let Some(price) = get_price(url).await.unwrap() {
        chat::send_message(&format!(
            "Current Mimosa Protein Milk price is {} € 🥛🐄",
            price
        ))
        .await;
        price
    } else {
        0.0
    };

    loop {
        log::info!("Sleeping for {} seconds..", interval);
        sleep(Duration::from_secs(interval)).await;
        log::info!("Checking milk price again..");
        let current_price_res = get_price(url).await;
        let current_price: f32;

        match current_price_res {
            Ok(price_option) => {
                if price_option.is_none() {
                    continue;
                }
                current_price = price_option.unwrap();
            }
            Err(_) => continue,
        }

        if current_price != last_price {
            let value_increased = current_price > last_price;
            let emoji = if value_increased { "😔" } else { "😊" };
            let message = format!(
                "Mimosa Protein Milk price went from {} to {}! 🥛🐄{}",
                last_price, current_price, emoji
            );
            chat::send_message(&message).await;
            last_price = current_price;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito;

    #[tokio::test]
    async fn test_get_price_success() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock(
                "GET",
                "/produto/leite-proteina-sem-lactose-mimosa-7652960.html",
            )
            .with_status(200)
            .with_body("<span class=\"ct-price-formatted\">€1,29</span>")
            .create();

        let mut url = server.url();
        url.push_str("/produto/leite-proteina-sem-lactose-mimosa-7652960.html");

        let result = get_price(&url).await.unwrap();
        assert_eq!(result, Some(1.29));
        mock.assert()
    }

    #[tokio::test]
    async fn test_get_price_parse_error() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock(
                "GET",
                "/produto/leite-proteina-sem-lactose-mimosa-7652960.html",
            )
            .with_status(200)
            .with_body("<span class=\"ct-price-formatted\">€invalid</span>")
            .create();

        let mut url = server.url();
        url.push_str("/produto/leite-proteina-sem-lactose-mimosa-7652960.html");
        let result = get_price(&url).await;
        assert!(matches!(result, Err(PriceError::PriceParseError(_))));
        mock.assert()
    }

    #[tokio::test]
    async fn test_get_price_request_failed() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock(
                "GET",
                "/produto/leite-proteina-sem-lactose-mimosa-7652960.html",
            )
            .with_status(404)
            .create();

        let mut url = server.url();
        url.push_str("/produto/leite-proteina-sem-lactose-mimosa-7652960.html");
        let result = get_price(&url).await;
        assert!(matches!(result, Err(PriceError::RequestFailed(_))));
        mock.assert()
    }
}
