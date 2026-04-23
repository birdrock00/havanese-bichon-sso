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

use poem::listener::{RustlsCertificate, RustlsConfig};
use bichon_core::{
    raise_error,
    {
        error::{code::ErrorCode, BichonResult},
        settings::dir::DATA_DIR_MANAGER,
    },
};

pub fn rustls_config() -> BichonResult<RustlsConfig> {
    let cert = std::fs::read_to_string(&DATA_DIR_MANAGER.tls_cert).map_err(|e| {
        raise_error!(
            format!(
                "Failed to read TLS certificate: '{}' (error: {})",
                DATA_DIR_MANAGER.tls_cert.display(),
                e
            ),
            ErrorCode::InternalError
        )
    })?;

    let key = std::fs::read_to_string(&DATA_DIR_MANAGER.tls_key).map_err(|e| {
        raise_error!(
            format!(
                "Failed to read TLS private key: '{}' (error: {})",
                DATA_DIR_MANAGER.tls_key.display(),
                e
            ),
            ErrorCode::InternalError
        )
    })?;
    let rustls_certificate = RustlsCertificate::new().cert(cert).key(key);
    Ok(RustlsConfig::new().fallback(rustls_certificate))
}
