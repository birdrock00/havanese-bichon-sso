//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
//
// This file is part of the Bichon Email Archiving Project
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.


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
            let status = res.status();
            let error_body = res.text().await.unwrap_or_default();
            eprintln!(
                "  {} Failed to send to [{}]. Status: {}\n  Server error: {}",
                style("✘").red(),
                folder,
                status,
                error_body
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
