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


use clap::Parser;
use serde::{Deserialize, Serialize};

use crate::bichon_version;

pub mod admin;
pub mod auth;
pub mod eml;
pub mod mbox;
pub mod pst;
pub mod sender;
pub mod thunderbird;

#[derive(Parser, Debug)]
#[command(
    name = "bichonctl",
    author = "rustmailer",
    version = bichon_version!(),
    about = "A CLI tool to import email data into Bichon service"
)]
pub struct BichonCli {
    /// Path to the configuration file
    #[arg(
        short,
        long,
        default_value = "config.toml",
        value_name = "FILE",
        help = "Sets a custom config file"
    )]
    pub config: std::path::PathBuf,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BichonCtlConfig {
    pub base_url: String,
    pub api_token: String,
}
