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

use rcgen::generate_simple_self_signed;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;
use std::io::{self, BufReader, Error, ErrorKind};
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio_rustls::TlsAcceptor;

use crate::modules::settings::cli::SETTINGS;

pub async fn create_acceptor() -> io::Result<TlsAcceptor> {
    let (certs, key) = if let (Some(key_path), Some(cert_path)) = (
        &SETTINGS.bichon_smtp_tls_key_path,
        &SETTINGS.bichon_smtp_tls_cert_path,
    ) {
        load_certs_from_files(key_path, cert_path).await?
    } else {
        generate_self_signed()?
    };

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| Error::new(ErrorKind::InvalidData, e))?;

    Ok(TlsAcceptor::from(Arc::new(config)))
}

async fn load_certs_from_files(
    key_path: &str,
    cert_path: &str,
) -> io::Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
    let mut key_file = File::open(key_path).await?;
    let mut key_data = Vec::new();
    key_file.read_to_end(&mut key_data).await?;

    let mut cert_file = File::open(cert_path).await?;
    let mut cert_data = Vec::new();
    cert_file.read_to_end(&mut cert_data).await?;

    let certs: Vec<CertificateDer<'static>> =
        rustls_pemfile::certs(&mut BufReader::new(cert_data.as_slice()))
            .filter_map(Result::ok)
            .collect();

    let key = rustls_pemfile::private_key(&mut BufReader::new(key_data.as_slice()))?
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "No private key found"))?;

    Ok((certs, key))
}

fn generate_self_signed() -> io::Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
    let subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    let key = generate_simple_self_signed(subject_alt_names).map_err(Error::other)?;

    let cert_der = CertificateDer::from(key.cert.der().to_vec());
    let key_der = PrivateKeyDer::try_from(key.signing_key.serialize_der())
        .map_err(|e| Error::new(ErrorKind::InvalidData, e))?;

    Ok((vec![cert_der], key_der))
}
