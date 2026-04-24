use bichon_core::account::stats::AccountStats;
use reqwest::Client;

use crate::BichonCtlConfig;

pub async fn fetch_account_stats(
    client: &Client,
    config: &BichonCtlConfig,
    account_id: u64,
) -> Option<AccountStats> {
    let url = format!("{}/api/v1/accounts/{}/stats", config.base_url, account_id);

    match client
        .get(&url)
        .header("Authorization", format!("Bearer {}", config.api_token))
        .send()
        .await
    {
        Ok(res) if res.status().is_success() => {
            match res.json::<AccountStats>().await {
                Ok(stats) => Some(stats),
                Err(e) => {
                    eprintln!(" ✘ Failed to parse stats response: {}", e);
                    None
                }
            }
        }
        Ok(res) => {
            let status = res.status();
            let error_body = res.text().await.unwrap_or_default();
            eprintln!(
                " ✘ Failed to fetch stats for account [{}]. Status: {}\n  Server error: {}",
                account_id,
                status,
                error_body
            );
            None
        }
        Err(e) => {
            eprintln!(
                " ✘ Network error fetching stats for [{}]: {}",
                account_id,
                e
            );
            None
        }
    }
}