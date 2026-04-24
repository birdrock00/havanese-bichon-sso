use bichon_core::{
    common::paginated::DataPage,
    message::search::{EmailSearchFilter, EmailSearchRequest, SortBy},
    store::envelope::Envelope,
};
use reqwest::Client;

use crate::BichonCtlConfig;

pub async fn search_messages(
    client: &Client,
    config: &BichonCtlConfig,
    page: u64,
    page_size: u64,
) -> Option<DataPage<Envelope>> {
    let url = format!("{}/api/v1/search-messages", config.base_url);

    let payload = EmailSearchRequest {
        filter: EmailSearchFilter::default(),
        page,
        page_size,
        sort_by: Some(SortBy::DATE),
        desc: Some(false),
    };

    match client
        .post(&url)
        .header("Authorization", format!("Bearer {}", config.api_token))
        .json(&payload)
        .send()
        .await
    {
        Ok(res) if res.status().is_success() => match res.json::<DataPage<Envelope>>().await {
            Ok(data) => Some(data),
            Err(e) => {
                eprintln!(" ✘ Failed to parse search response: {}", e);
                None
            }
        },
        Ok(res) => {
            let status = res.status();
            let error_body = res.text().await.unwrap_or_default();
            eprintln!(
                " ✘ Failed to search messages. Status: {}\n  Server error: {}",
                status, error_body
            );
            None
        }
        Err(e) => {
            eprintln!(" ✘ Network error performing search: {}", e);
            None
        }
    }
}
