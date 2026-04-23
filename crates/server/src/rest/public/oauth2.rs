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


use bichon_core::oauth2::{flow::OAuth2Flow, pending::OAuth2PendingEntity};
use poem::{
    handler,
    web::{Query, Redirect},
    IntoResponse, Result,
};
use serde::{Deserialize, Serialize};
use tracing::error;

#[derive(Serialize, Deserialize, Debug)]
pub struct OAuth2CallbackParams {
    state: Option<String>,
    code: Option<String>,
}
#[handler]
pub async fn oauth2_callback(
    Query(params): Query<OAuth2CallbackParams>,
) -> Result<impl IntoResponse> {
    let (state, code) = match (&params.state, &params.code) {
        (Some(state), Some(code)) => (state, code),
        (None, _) => {
            let message =
                "The state parameter is missing. Please initiate the OAuth2 process again.";
            return Ok(Redirect::temporary(format!(
                "/oauth2-result?error=missing_state&message={}",
                urlencoding::encode(message)
            ))
            .into_response());
        }
        (_, None) => {
            let message = "The authorization code is missing. Please try the OAuth2 login again.";
            return Ok(Redirect::temporary(format!(
                "/oauth2-result?error=missing_code&message={}",
                urlencoding::encode(message)
            ))
            .into_response());
        }
    };

    let pending = match OAuth2PendingEntity::get(state).await {
        Ok(Some(pending)) => pending,
        _ => {
            let message =
                "The provided state is invalid or expired. Please start the OAuth2 process again.";
            return Ok(Redirect::temporary(format!(
                "/oauth2-result?error=invalid_state&message={}",
                urlencoding::encode(message)
            ))
            .into_response());
        }
    };

    let flow = OAuth2Flow::new(pending.oauth2_id);
    if let Err(e) = flow
        .fetch_save_access_token(pending.account_id, &pending.code_verifier, code)
        .await
    {
        error!("Failed to save access token: {:#?}", e);
        let message = format!(
            "Failed to retrieve or save the access token. Error details: {:#?}",
            e
        );
        return Ok(Redirect::temporary(format!(
            "/oauth2-result?error=token_fetch_failed&message={}",
            urlencoding::encode(&message)
        ))
        .into_response());
    }

    if let Err(e) = OAuth2PendingEntity::delete(state).await {
        error!("Failed to delete pending OAuth2 entity: {}", e);
    }

    Ok(Redirect::temporary("/oauth2-result?success=true").into_response())
}
