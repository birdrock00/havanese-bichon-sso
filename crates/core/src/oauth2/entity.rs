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

use crate::{
    common::paginated::DataPage,
    database::{
        delete_impl, find_impl, insert_impl, manager::DB_MANAGER, paginate_impl, update_impl,
        MemDbModel,
    },
    encrypt,
    error::{code::ErrorCode, BichonResult},
    id, raise_error, utc_now,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Whether the OAuth2 client authenticates at the token endpoint with a secret.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Enum))]
pub enum OAuth2ClientType {
    /// A client with a provider-issued secret.
    #[default]
    Confidential,
    /// A public/native client without a secret. Authorization relies on PKCE.
    Public,
}

/// Represents the OAuth2 configuration for a client, including initialization and runtime values.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct OAuth2 {
    /// A unique identifier for the OAuth2 configuration.
    pub id: u64,
    /// A description of what this configuration is used for.
    pub description: Option<String>,
    /// The client ID used for authenticating the application with the OAuth2 provider.
    pub client_id: String,
    /// The client type. Records without one are confidential.
    #[serde(default)]
    pub client_type: OAuth2ClientType,
    /// The client secret used in conjunction with the client ID. `None` for public clients.
    ///
    /// Users should provide a plaintext secret.
    /// The server will encrypt it using AES-256-GCM and securely store it.
    /// The plaintext secret is never stored, so users must ensure it is valid for OAuth2 authentication.
    pub client_secret: Option<String>,
    /// The URL to redirect users to for OAuth2 authorization.
    pub auth_url: String,
    /// The URL to exchange authorization codes for access tokens.
    pub token_url: String,
    /// The URI where the OAuth2 provider will redirect to after authorization.
    pub redirect_uri: String,
    /// The scopes of access that are being requested (e.g., email, profile).
    pub scopes: Option<Vec<String>>,
    /// Any additional parameters to include in the OAuth2 requests (e.g., access_type, prompt).
    pub extra_params: Option<BTreeMap<String, String>>,
    /// Indicates whether this configuration is enabled or disabled.
    pub enabled: bool,
    /// route OAuth through proxy (when direct access is blocked)
    pub use_proxy: Option<u64>,
    /// The timestamp when the configuration was created, in milliseconds since the Unix epoch.
    pub created_at: i64,
    /// The timestamp when the configuration was last updated, in milliseconds since the Unix epoch.
    pub updated_at: i64,
}

impl MemDbModel for OAuth2 {
    fn collection() -> &'static str {
        "oauth2_configs"
    }
    fn key(&self) -> String {
        self.id.to_string()
    }
}

impl OAuth2 {
    pub fn new(request: OAuth2CreateRequest) -> BichonResult<Self> {
        let client_secret =
            resolve_client_secret(request.client_type, request.client_secret.as_deref(), None)?;
        Ok(OAuth2 {
            id: id!(64),
            description: request.description,
            client_id: request.client_id,
            client_type: request.client_type,
            client_secret,
            auth_url: request.auth_url,
            token_url: request.token_url,
            redirect_uri: request.redirect_uri,
            scopes: request.scopes,
            extra_params: request.extra_params,
            enabled: request.enabled,
            created_at: utc_now!(),
            updated_at: utc_now!(),
            use_proxy: request.use_proxy,
        })
    }

    pub fn scrub_sensitive_fields(&mut self) {
        let mask = "********";
        let notice =
            " [REDACTED: You do not have permission to view sensitive configuration details]";

        let original_desc = self
            .description
            .clone()
            .unwrap_or_else(|| "OAuth2 Config".to_string());
        self.description = Some(format!("{}{}", original_desc, notice));

        self.client_id = mask.to_string();
        self.client_secret = self.client_secret.as_ref().map(|_| mask.to_string());
        self.auth_url = self.auth_url.clone();
        self.token_url = self.token_url.clone();
        self.redirect_uri = self.redirect_uri.clone();

        self.scopes = None;
        self.extra_params = None;
    }

    pub fn save(&self) -> BichonResult<()> {
        insert_impl(DB_MANAGER.db(), self.to_owned())?;
        Ok(())
    }

    pub fn paginate_list(
        page: Option<u64>,
        page_size: Option<u64>,
        desc: Option<bool>,
    ) -> BichonResult<DataPage<OAuth2>> {
        let paginated = paginate_impl::<OAuth2>(DB_MANAGER.db(), page, page_size, desc)?;
        Ok(DataPage::from(paginated))
    }

    pub fn get(id: u64) -> BichonResult<Option<OAuth2>> {
        let results = find_impl::<OAuth2>(DB_MANAGER.db(), &id.to_string())?;
        Ok(results.into_iter().next())
    }

    pub fn delete(id: u64) -> BichonResult<()> {
        delete_impl::<OAuth2>(DB_MANAGER.db(), &id.to_string())
    }

    /// Applies a partial update and returns the configuration as stored, so
    /// callers can report the effective values (e.g. the client type) rather
    /// than re-deriving them from the request.
    pub fn update(id: u64, request: OAuth2UpdateRequest) -> BichonResult<OAuth2> {
        update_impl(DB_MANAGER.db(), &id.to_string(), |current| {
            apply_update(&current, request)
        })
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct OAuth2CreateRequest {
    /// A description of what this configuration is used for.
    pub description: Option<String>,

    /// The client ID used for authenticating the application with the OAuth2 provider.
    pub client_id: String,

    /// The client type. Defaults to confidential when omitted.
    #[serde(default)]
    #[cfg_attr(feature = "web-api", oai(default))]
    pub client_type: OAuth2ClientType,

    /// The client secret used in conjunction with the client ID. Required for confidential clients.
    pub client_secret: Option<String>,

    /// The URL to redirect users to for OAuth2 authorization.
    pub auth_url: String,

    /// The URL to exchange authorization codes for access tokens.
    pub token_url: String,

    /// The URI where the OAuth2 provider will redirect to after authorization.
    pub redirect_uri: String,

    /// The scopes of access that are being requested (e.g., email, profile).
    pub scopes: Option<Vec<String>>,

    /// Any additional parameters to include in the OAuth2 requests (e.g., access_type, prompt).
    pub extra_params: Option<BTreeMap<String, String>>,

    /// Indicates whether this configuration is enabled or disabled.
    pub enabled: bool,

    /// route OAuth through proxy (when direct access is blocked)
    pub use_proxy: Option<u64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct OAuth2UpdateRequest {
    /// A description of what this configuration is used for.
    pub description: Option<String>,

    /// The client ID used for authenticating the application with the OAuth2 provider.
    pub client_id: Option<String>,

    /// The client type. Switching to public clears the stored secret.
    pub client_type: Option<OAuth2ClientType>,

    /// The client secret used in conjunction with the client ID.
    pub client_secret: Option<String>,

    /// The URL to redirect users to for OAuth2 authorization.
    pub auth_url: Option<String>,

    /// The URL to exchange authorization codes for access tokens.
    pub token_url: Option<String>,

    /// The URI where the OAuth2 provider will redirect to after authorization.
    pub redirect_uri: Option<String>,

    /// The scopes of access that are being requested (e.g., email, profile).
    pub scopes: Option<Vec<String>>,

    /// Any additional parameters to include in the OAuth2 requests (e.g., access_type, prompt).
    pub extra_params: Option<BTreeMap<String, String>>,

    /// Indicates whether this configuration is enabled or disabled.
    pub enabled: Option<bool>,

    /// route OAuth through proxy (when direct access is blocked)
    pub use_proxy: Option<u64>,
}

fn apply_update(old: &OAuth2, request: OAuth2UpdateRequest) -> BichonResult<OAuth2> {
    let mut new = old.clone();
    new.client_type = request.client_type.unwrap_or(old.client_type);
    new.client_secret = resolve_client_secret(
        new.client_type,
        request.client_secret.as_deref(),
        old.client_secret.as_deref(),
    )?;
    if request.description.is_some() {
        new.description = request.description;
    }
    if let Some(client_id) = request.client_id {
        new.client_id = client_id;
    }
    if let Some(auth_url) = request.auth_url {
        new.auth_url = auth_url;
    }
    if let Some(token_url) = request.token_url {
        new.token_url = token_url;
    }
    if let Some(redirect_uri) = request.redirect_uri {
        new.redirect_uri = redirect_uri;
    }
    if request.scopes.is_some() {
        new.scopes = request.scopes;
    }
    if request.extra_params.is_some() {
        new.extra_params = request.extra_params;
    }
    if let Some(enabled) = request.enabled {
        new.enabled = enabled;
    }
    if let Some(use_proxy) = request.use_proxy {
        new.use_proxy = Some(use_proxy);
    }
    new.updated_at = utc_now!();
    Ok(new)
}

/// Returns the encrypted secret to store: a supplied secret replaces `current`,
/// confidential clients must end up with one and public clients never have one.
fn resolve_client_secret(
    client_type: OAuth2ClientType,
    supplied: Option<&str>,
    current: Option<&str>,
) -> BichonResult<Option<String>> {
    let supplied = supplied.filter(|secret| !secret.trim().is_empty());
    match (client_type, supplied) {
        (OAuth2ClientType::Confidential, Some(secret)) => Ok(Some(encrypt!(secret)?)),
        (OAuth2ClientType::Confidential, None) => {
            current.map(ToOwned::to_owned).map(Some).ok_or_else(|| {
                raise_error!(
                    "A client secret is required for confidential OAuth2 clients.".into(),
                    ErrorCode::InvalidParameter
                )
            })
        }
        (OAuth2ClientType::Public, Some(_)) => Err(raise_error!(
            "Public OAuth2 clients must not include a client secret.".into(),
            ErrorCode::InvalidParameter
        )),
        (OAuth2ClientType::Public, None) => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    static TEST_ENV: Once = Once::new();
    fn init_test_env() {
        TEST_ENV.call_once(|| {
            let root = crate::test_support::unique_test_root("bichon-core-test");
            std::env::set_var("BICHON_ROOT_DIR", &root);
            std::env::set_var("BICHON_ENCRYPT_PASSWORD", "test-password");
            let _ = &*crate::settings::cli::SETTINGS;
        });
    }

    #[test]
    fn stored_records_without_client_type_are_confidential() {
        let oauth2: OAuth2 = serde_json::from_value(serde_json::json!({
            "id": 1, "description": null, "client_id": "id", "client_secret": "encrypted",
            "auth_url": "a", "token_url": "t", "redirect_uri": "r", "scopes": null,
            "extra_params": null, "enabled": true, "use_proxy": null,
            "created_at": 1, "updated_at": 1
        }))
        .unwrap();
        assert_eq!(oauth2.client_type, OAuth2ClientType::Confidential);
        assert_eq!(oauth2.client_secret.as_deref(), Some("encrypted"));
    }

    #[test]
    fn client_secret_follows_client_type() {
        use OAuth2ClientType::{Confidential, Public};
        init_test_env();
        let stored = "stored".to_string();

        assert!(resolve_client_secret(Confidential, Some("new"), None)
            .unwrap()
            .is_some_and(|secret| secret != "new"));
        assert_eq!(
            resolve_client_secret(Confidential, Some(" "), Some(stored.as_str())).unwrap(),
            Some(stored.clone())
        );
        assert!(resolve_client_secret(Confidential, None, None).is_err());
        assert_eq!(
            resolve_client_secret(Public, None, Some(stored.as_str())).unwrap(),
            None
        );
        assert!(resolve_client_secret(Public, Some("new"), None).is_err());
    }
}
