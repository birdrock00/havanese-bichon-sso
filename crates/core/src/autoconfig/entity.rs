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


use autoconfig::config::OAuth2Config as XOAuth2Config;
use serde::{Deserialize, Serialize};

use crate::account::entity::Encryption;

#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct ServerConfig {
    /// server hostname or IP address
    pub host: String,
    /// server port number
    pub port: u16,
    /// Connection encryption method
    pub encryption: Encryption,
}

impl ServerConfig {
    pub fn new(host: String, port: u16, encryption: Encryption) -> Self {
        Self {
            host,
            port,
            encryption,
        }
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct OAuth2Config {
    /// The authorization server's issuer identifier URL
    pub issuer: String,
    /// List of scopes requested by the client
    pub scope: Vec<String>,
    /// URL of the authorization server's authorization endpoint
    pub auth_url: String,
    /// URL of the authorization server's token endpoint
    pub token_url: String,
}

impl From<&XOAuth2Config> for OAuth2Config {
    fn from(value: &XOAuth2Config) -> Self {
        Self {
            issuer: value.issuer().into(),
            scope: value.scope().into_iter().map(Into::into).collect(),
            auth_url: value.auth_url().into(),
            token_url: value.token_url().into(),
        }
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "web-api", derive(poem_openapi::Object))]
pub struct MailServerConfig {
    /// IMAP server configuration
    pub imap: ServerConfig,
    /// OAuth 2.0 client configuration parameters
    pub oauth2: Option<OAuth2Config>,
}
