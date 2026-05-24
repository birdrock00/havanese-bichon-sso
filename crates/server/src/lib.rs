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


pub mod common;
pub mod error;
pub mod rest;

use std::sync::LazyLock;

use bichon_core::{
    bichon_version,
    cache::imap::task::SYNC_TASKS,
    common::{rustls::BichonTls, signal::SignalManager},
    context::{executors::BichonContext, Initialize},
    database::manager::DB_MANAGER,
    error::{code::ErrorCode, BichonResult},
    logger,
    migrate::check_data_status,
    raise_error,
    settings::{cli::SETTINGS, dir::DataDirManager},
    store::{
        blob::BLOB_MANAGER,
        tantivy::{attachment::ATTACHMENT_MANAGER, envelope::ENVELOPE_MANAGER},
    },
    tasks::PeriodicTasks,
    users::manager::UserManager,
};
use bichon_smtp::server::{start_smtp_server, SmtpServer};
use tracing::{error, info};

pub async fn run() -> BichonResult<()> {
    logger::initialize_logging();
    info!(
        r#"
     _      _        _
    | |    (_)      | |
    | |__   _   ___ | |__    ___   _ __
    | '_ \ | | / __|| '_ \  / _ \ | '_ \
    | |_) || || (__ | | | || (_) || | | |
    |_.__/ |_| \___||_| |_| \___/ |_| |_|

    "#
    );
    info!("Starting bichon-server");
    info!("Version:  {}", bichon_version!());
    info!("Git:      [{}]", env!("GIT_HASH"));
    info!("GitHub:   https://github.com/rustmailer/bichon");

    match check_data_status() {
        Ok(false) => {
            error!("Incompatible data format detected.");
            error!("Your data was created by an older version of Bichon and must be migrated before use.");
            error!("Please stop the Bichon v0.3.7 service before migration.");
            error!("Please run: bichon-admin");
            error!("Documentation: https://github.com/rustmailer/bichon/wiki/Bichon-Data-Migration:-v0.3.7-%E2%86%92-v1.0");
            return Err(raise_error!(
                "Legacy data layout detected".into(),
                ErrorCode::InternalError
            ));
        }
        Err(e) => {
            error!("Failed to check data layout: {:#?}", e);
            return Err(raise_error!(format!("{:#?}", e), ErrorCode::InternalError));
        }
        Ok(true) => {}
    }

    if let Err(error) = initialize().await {
        eprintln!("{:?}", error);
        return Err(error);
    }

    let periodic_tasks = PeriodicTasks::setup();
    let mut smtp_service: Option<SmtpServer> = None;
    if SETTINGS.bichon_enable_smtp {
        info!("SMTP service is enabled, starting...");
        match start_smtp_server().await {
            Ok(server) => {
                info!("SMTP server listening on: {}", server.smtp_addr);
                smtp_service = Some(server);
            }
            Err(e) => {
                error!("Failed to start SMTP server: {}", e);
                return Err(raise_error!(format!("{:#?}", e), ErrorCode::InternalError));
            }
        }
    } else {
        info!("SMTP service is disabled by configuration.");
    }

    rest::start_http_server().await?;
    periodic_tasks.shutdown().await;

    if let Some(server) = smtp_service {
        info!("Shutting down SMTP server...");
        server.stop().await;
        info!("SMTP server stopped.");
    }

    SYNC_TASKS.shutdown().await;
    ENVELOPE_MANAGER.shutdown().await;
    ATTACHMENT_MANAGER.shutdown().await;
    BLOB_MANAGER.shutdown().await;
    DB_MANAGER.flush();
    info!("Bichon server stopped.");
    Ok(())
}

async fn initialize() -> BichonResult<()> {
    SignalManager::initialize().await?;
    DataDirManager::initialize().await?;
    UserManager::initialize().await?;
    BichonTls::initialize().await?;
    BichonContext::initialize().await?;
    LazyLock::force(&BLOB_MANAGER);
    LazyLock::force(&ENVELOPE_MANAGER);
    LazyLock::force(&ATTACHMENT_MANAGER);
    Ok(())
}
