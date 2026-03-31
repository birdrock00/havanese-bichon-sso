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

use bichon::{
    bichon_version,
    modules::{
        blob::{manager::ENVELOPE_INDEX_MANAGER, storage::BLOB_MANAGER},
        cache::imap::task::SYNC_TASKS,
        common::rustls::BichonTls,
        context::{executors::BichonContext, Initialize},
        duckdb::init::DuckDBManager,
        error::{code::ErrorCode, BichonResult},
        logger,
        rest::start_http_server,
        settings::cli::SETTINGS,
        smtp::{start_smtp_server, SmtpServer},
        tasks::PeriodicTasks,
    },
    raise_error,
};
use mimalloc::MiMalloc;
use tracing::{error, info};

use bichon::modules::{
    common::signal::SignalManager, settings::dir::DataDirManager, users::manager::UserManager,
};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

static LOGO: &str = r#"
 _      _        _                   
| |    (_)      | |                  
| |__   _   ___ | |__    ___   _ __  
| '_ \ | | / __|| '_ \  / _ \ | '_ \ 
| |_) || || (__ | | | || (_) || | | |
|_.__/ |_| \___||_| |_| \___/ |_| |_|
                                     
"#;
#[tokio::main]
async fn main() -> BichonResult<()> {
    logger::initialize_logging();
    info!("{}", LOGO);
    info!("Starting bichon-server");
    info!("Version:  {}", bichon_version!());
    info!("Git:      [{}]", env!("GIT_HASH"));
    info!("GitHub:   https://github.com/rustmailer/bichon");

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

    start_http_server().await?;
    periodic_tasks.shutdown().await;

    if let Some(server) = smtp_service {
        info!("Shutting down SMTP server...");
        server.stop().await;
        info!("SMTP server stopped.");
    }

    SYNC_TASKS.shutdown().await;
    ENVELOPE_INDEX_MANAGER.shutdown().await;
    BLOB_MANAGER.shutdown().await;
    info!("Bichon server stopped.");
    Ok(())
}

/// Initialize the system by validating settings and starting necessary tasks.
async fn initialize() -> BichonResult<()> {
    SignalManager::initialize().await?;
    DataDirManager::initialize().await?;
    DuckDBManager::initialize().await?;
    UserManager::initialize().await?;
    BichonTls::initialize().await?;
    BichonContext::initialize().await?;
    Ok(())
}
