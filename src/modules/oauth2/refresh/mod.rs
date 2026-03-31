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

use crate::modules::common::periodic::{PeriodicTask, TaskHandle};
use crate::modules::context::BichonTask;
use crate::modules::oauth2::token::EXTERNAL_OAUTH_APP_ID;
use crate::modules::oauth2::{flow::OAuth2Flow, token::OAuth2AccessToken};
use crate::utc_now;
use std::time::Duration;
use tracing::{debug, error, info};

const TASK_INTERVAL: Duration = Duration::from_secs(60); // Interval set to 1 minute
const FIFTEEN_MINUTES: Duration = Duration::from_secs(45 * 60);
///This task cleans up expired OAuth2 pending authorizations that haven't been completed by users in a timely manner.
pub struct OAuth2RefreshTask;

impl BichonTask for OAuth2RefreshTask {
    fn start() -> TaskHandle {
        let periodic_task = PeriodicTask::new("oauth2-token-refresh-task");

        let task = move |_: Option<u64>| {
            Box::pin(async move {
                debug!("Starting OAuth2 token refresh task");

                // Try to retrieve all OAuth2 access tokens
                match OAuth2AccessToken::list_all().await {
                    Ok(all_tokens) => {
                        let need_refresh: Vec<OAuth2AccessToken> = all_tokens
                            .into_iter()
                            .filter(|token| {
                                ((utc_now!() - token.updated_at)
                                    > FIFTEEN_MINUTES.as_millis() as i64)
                                    && token.oauth2_id != EXTERNAL_OAUTH_APP_ID
                            }) // Filter tokens older than 15 minutes
                            .collect();

                        if need_refresh.is_empty() {
                            debug!("No expired tokens need to be refreshed.");
                        } else {
                            debug!(
                                "Found {} tokens that need to be refreshed",
                                need_refresh.len()
                            );
                            for token in need_refresh {
                                tokio::spawn(async move {
                                    let flow = OAuth2Flow::new(token.oauth2_id.clone());
                                    if let Err(error) = flow.refresh_access_token(&token).await {
                                        error!(
                                            "Failed to refresh access token for {}: {}",
                                            token.account_id, error
                                        );
                                    } else {
                                        info!(
                                            "Successfully refreshed access token for {}",
                                            token.account_id
                                        );
                                    }
                                });
                            }
                        }
                    }
                    Err(e) => {
                        // Log the error when retrieving tokens
                        error!("Failed to fetch OAuth2 tokens: {:?}", e);
                    }
                }

                debug!("OAuth2 token refresh task completed");
                Ok(())
            })
        };

        periodic_task.start(task, None, TASK_INTERVAL, false, true)
    }
}
