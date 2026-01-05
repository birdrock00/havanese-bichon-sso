use console::style;
use reqwest::Client;

use crate::modules::{cli::BichonCtlConfig, import::BatchEmlRequest};

pub async fn send_batch_request(
    client: &Client,
    config: &BichonCtlConfig,
    account_id: u64,
    folder: &str,
    emls: Vec<String>,
) {
    let url = format!("{}/api/v1/import", config.base_url);
    let payload = BatchEmlRequest {
        account_id,
        mail_folder: folder.to_string(),
        emls,
    };

    let count = payload.emls.len();

    match client
        .post(&url)
        .header("Authorization", format!("Bearer {}", config.api_token))
        .json(&payload)
        .send()
        .await
    {
        Ok(res) if res.status().is_success() => {
            println!(
                "  {} Sent {} emails to [{}]",
                style("✔").green(),
                count,
                folder
            );
        }
        Ok(res) => {
            eprintln!(
                "  {} Failed to send to [{}]. Status: {}",
                style("✘").red(),
                folder,
                res.status()
            );
        }
        Err(e) => {
            eprintln!(
                "  {} Network error on [{}]: {}",
                style("✘").red(),
                folder,
                e
            );
        }
    }
}
