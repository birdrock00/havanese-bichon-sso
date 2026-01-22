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

use crate::modules::settings::cli::Settings;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};

pub mod cli;
pub mod dir;
pub mod io;
pub mod proxy;
pub mod system;
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize, Object)]
pub struct SystemConfigurations {
    pub bichon_log_level: String,
    pub bichon_http_port: i32,
    pub bichon_bind_ip: Option<String>,
    pub bichon_public_url: String,

    pub bichon_cors_origins: Option<Vec<String>>,
    pub bichon_cors_max_age: i32,

    pub bichon_ansi_logs: bool,
    pub bichon_log_to_file: bool,
    pub bichon_json_logs: bool,
    pub bichon_max_server_log_files: usize,

    pub bichon_encrypt_password_set: bool,
    pub bichon_webui_token_expiration_hours: u32,

    pub bichon_root_dir: String,
    pub bichon_metadata_cache_size: Option<usize>,
    pub bichon_envelope_cache_size: Option<usize>,

    pub bichon_enable_rest_https: bool,
    pub bichon_http_compression_enabled: bool,
    pub bichon_sync_concurrency: Option<u16>,
}

impl From<&Settings> for SystemConfigurations {
    fn from(s: &Settings) -> Self {
        Self {
            bichon_log_level: s.bichon_log_level.clone(),
            bichon_http_port: s.bichon_http_port,
            bichon_bind_ip: s.bichon_bind_ip.clone(),
            bichon_public_url: s.bichon_public_url.clone(),
            bichon_cors_origins: s
                .bichon_cors_origins
                .as_ref()
                .map(|set| set.iter().cloned().collect()),
            bichon_cors_max_age: s.bichon_cors_max_age,
            bichon_ansi_logs: s.bichon_ansi_logs,
            bichon_log_to_file: s.bichon_log_to_file,
            bichon_json_logs: s.bichon_json_logs,
            bichon_max_server_log_files: s.bichon_max_server_log_files,
            bichon_encrypt_password_set: s.bichon_encrypt_password.is_some()
                || s.bichon_encrypt_password_file.is_some(),
            bichon_webui_token_expiration_hours: s.bichon_webui_token_expiration_hours,
            bichon_root_dir: s.bichon_root_dir.clone(),
            bichon_metadata_cache_size: s.bichon_metadata_cache_size,
            bichon_envelope_cache_size: s.bichon_envelope_cache_size,
            bichon_enable_rest_https: s.bichon_enable_rest_https,
            bichon_http_compression_enabled: s.bichon_http_compression_enabled,
            bichon_sync_concurrency: s.bichon_sync_concurrency,
        }
    }
}
