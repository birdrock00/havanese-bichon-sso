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

use crate::modules::context::Initialize;
use crate::modules::settings::cli::SETTINGS;
use crate::{
    modules::error::{code::ErrorCode, BichonResult},
    raise_error,
};
use std::path::PathBuf;
use std::sync::LazyLock;

pub const META_FILE: &str = "meta.db";
pub const MAILBOX_FILE: &str = "mailbox.db";
const ENVELOPE_DIR: &str = "envelope";
const EML_DIR: &str = "eml";
const TMP_DIR: &str = "tmp";
const LOG_DIR: &str = "logs";
const TLS_CERT: &str = "cert.pem";
const TLS_KEY: &str = "key.pem";

pub static DATA_DIR_MANAGER: LazyLock<DataDirManager> =
    LazyLock::new(|| DataDirManager::new(PathBuf::from(&SETTINGS.bichon_root_dir)));

#[derive(Debug)]
pub struct DataDirManager {
    pub root_dir: PathBuf,
    pub meta_db: PathBuf,
    pub mailbox_db: PathBuf,
    pub temp_dir: PathBuf,
    pub tls_cert: PathBuf,
    pub tls_key: PathBuf,
    pub envelope_dir: PathBuf,
    pub eml_dir: PathBuf,
    pub log_dir: PathBuf,
}

impl Initialize for DataDirManager {
    async fn initialize() -> BichonResult<()> {
        std::fs::create_dir_all(&DATA_DIR_MANAGER.root_dir)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        std::fs::create_dir_all(&DATA_DIR_MANAGER.log_dir)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        std::fs::create_dir_all(&DATA_DIR_MANAGER.temp_dir)
            .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;
        Ok(())
    }
}

impl DataDirManager {
    pub fn new(root_dir: PathBuf) -> Self {
        let envelope_dir = if let Some(ref index_dir) = SETTINGS.bichon_index_dir {
            PathBuf::from(index_dir)
        } else {
            root_dir.join(ENVELOPE_DIR)
        };

        let eml_dir = if let Some(ref data_dir) = SETTINGS.bichon_data_dir {
            PathBuf::from(data_dir)
        } else {
            root_dir.join(EML_DIR)
        };

        Self {
            root_dir: root_dir.clone(),
            meta_db: root_dir.join(META_FILE),
            mailbox_db: root_dir.join(MAILBOX_FILE),
            tls_key: root_dir.join(TLS_KEY),
            tls_cert: root_dir.join(TLS_CERT),
            log_dir: root_dir.join(LOG_DIR),
            envelope_dir,
            temp_dir: root_dir.join(TMP_DIR),
            eml_dir,
        }
    }
}
