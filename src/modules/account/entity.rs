//
// Copyright (c) 2025 rustmailer.com (https://rustmailer.com)
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


use crate::{encrypt, modules::error::BichonResult};

use poem_openapi::{Enum, Object};
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Debug, Eq, PartialEq, Serialize, Deserialize, Object)]
pub struct ImapConfig {
    /// IMAP server hostname or IP address
    #[oai(validator(max_length = 253, pattern = r"^[a-zA-Z0-9\-\.]+$"))]
    pub host: String,
    /// IMAP server port number
    #[oai(validator(minimum(value = "1"), maximum(value = "65535")))]
    pub port: u16,
    /// Connection encryption method
    pub encryption: Encryption,
    /// Authentication configuration
    pub auth: AuthConfig,
    /// Optional proxy ID for establishing the connection.
    /// - If `None` or not provided, the client will connect directly to the IMAP server.
    /// - If `Some(proxy_id)`, the client will use the pre-configured proxy with the given ID.
    pub use_proxy: Option<u64>,
}

impl ImapConfig {
    pub fn try_encrypt_password(self) -> BichonResult<Self> {
        Ok(Self {
            host: self.host,
            port: self.port,
            encryption: self.encryption,
            auth: self.auth.encrypt()?,
            use_proxy: self.use_proxy,
        })
    }
}


#[derive(Enum, Default, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AuthType {
    /// Standard password authentication (PLAIN/LOGIN)
    #[default]
    Password,
    /// OAuth 2.0 authentication (SASL XOAUTH2)
    OAuth2,
}

#[derive(Object, Default, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AuthConfig {
    ///Authentication method to use
    pub auth_type: AuthType,
    /// Credential secret for Password authentication.
    ///
    /// Users should provide a plaintext password (1 to 256 characters).
    /// The server will encrypt the password using AES-256-GCM and securely store it.
    #[oai(validator(max_length = 256, min_length = 1))]
    pub password: Option<String>,
}

impl AuthConfig {
    pub fn encrypt(self) -> BichonResult<Self> {
        match self.password {
            Some(password) => Ok(Self {
                auth_type: self.auth_type,
                password: Some(encrypt!(&password)?),
            }),
            None => Ok(self),
        }
    }
}

impl AuthConfig {
    pub fn validate(&self) -> Result<(), &'static str> {
        match self.auth_type {
            AuthType::Password if self.password.is_none() => {
                Err("When auth_type is Passwd, password must not be None.")
            }
            _ => Ok(()),
        }
    }
}

#[derive(Clone, Default, Debug, Eq, PartialEq, Serialize, Deserialize, Enum)]
pub enum Encryption {
    /// SSL/TLS encrypted connection
    #[default]
    Ssl,
    /// StartTLS encryption
    StartTls,
    /// Unencrypted connection
    None,
}

impl From<bool> for Encryption {
    fn from(value: bool) -> Self {
        if value {
            Self::Ssl
        } else {
            Self::None
        }
    }
}
