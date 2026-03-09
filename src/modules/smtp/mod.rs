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

use std::net::SocketAddr;

use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

use crate::modules::{
    common::signal::SIGNAL_MANAGER,
    settings::cli::{SmtpEncryptionMode, SETTINGS},
    smtp::{
        server::{run_smtp_server, run_smtps_server},
        tls::create_acceptor,
    },
};

pub mod server;
pub mod stream;
pub mod tls;
#[cfg(test)]
mod tests;

#[derive(Clone, Default)]
pub struct SmtpConfig {
    pub whitelist: Option<Vec<String>>,
    pub tls_acceptor: Option<TlsAcceptor>,
    pub auth_required: bool,
}

pub struct SmtpServer {
    pub smtp_addr: SocketAddr,
    smtp_handle: tokio::task::JoinHandle<()>,
}

impl SmtpServer {
    pub async fn stop(self) {
        let _ = self.smtp_handle.await;
    }
}

pub async fn start_smtp_server() -> std::io::Result<SmtpServer> {
    let smtp_port = SETTINGS.bichon_smtp_port;

    let tls_acceptor: Option<TlsAcceptor> = match SETTINGS.bichon_smtp_encryption {
        SmtpEncryptionMode::None => None,
        SmtpEncryptionMode::Starttls | SmtpEncryptionMode::Tls => Some(create_acceptor().await?),
    };

    let smtp_listener = TcpListener::bind((
        SETTINGS.bichon_bind_ip.clone().unwrap_or("0.0.0.0".into()),
        smtp_port,
    ))
    .await
    .map_err(|e| {
        if e.kind() == std::io::ErrorKind::AddrInUse {
            std::io::Error::other(format!(
                "SMTP port {smtp_port} is already in use. Is another instance running?"
            ))
        } else {
            e
        }
    })?;
    let smtp_addr = smtp_listener.local_addr()?;

    let smtp_config = SmtpConfig {
        whitelist: None,
        tls_acceptor: match SETTINGS.bichon_smtp_encryption {
            SmtpEncryptionMode::None | SmtpEncryptionMode::Tls => None,
            SmtpEncryptionMode::Starttls => tls_acceptor.clone(),
        },
        auth_required: SETTINGS.bichon_smtp_auth_required,
    };

    let smtp_shutdown = SIGNAL_MANAGER.subscribe();

    let smtp_handle = if matches!(SETTINGS.bichon_smtp_encryption, SmtpEncryptionMode::Tls) {
        let acceptor = tls_acceptor
            .clone()
            .expect("TLS acceptor required when tls=true");
        tokio::spawn(async move {
            run_smtps_server(smtp_listener, smtp_config, acceptor, smtp_shutdown).await;
        })
    } else {
        tokio::spawn(async move {
            run_smtp_server(smtp_listener, smtp_config, smtp_shutdown).await;
        })
    };

    Ok(SmtpServer {
        smtp_addr,
        smtp_handle,
    })
}
