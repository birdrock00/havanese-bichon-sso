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

use std::sync::LazyLock;

use bichon_core::{
    bichon_version,
    cache::imap::task::SYNC_TASKS,
    common::rustls::BichonTls,
    context::{executors::BichonContext, Initialize},
    error::{code::ErrorCode, BichonResult},
    logger,
    migrate::check_data_status,
    raise_error,
    settings::cli::SETTINGS,
    store::{
        blob::BLOB_MANAGER,
        tantivy::{attachment::ATTACHMENT_MANAGER, envelope::ENVELOPE_MANAGER},
    },
    tasks::PeriodicTasks,
};
use bichon_smtp::server::{start_smtp_server, SmtpServer};
use mimalloc::MiMalloc;
use tracing::{error, info};

use bichon_core::{
    common::signal::SignalManager, settings::dir::DataDirManager, users::manager::UserManager,
};

use crate::rest::start_http_server;

pub mod common;
pub mod error;
pub mod rest;

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

    match check_data_status() {
        Ok(false) => {
            error!("Incompatible data format detected.");
            error!("Your data was created by an older version of Bichon and must be migrated before use.");
            error!("Please run: bichon-admin");
            error!("Documentation: https://github.com/rustmailer/bichon/wiki/migration");
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

    start_http_server().await?;
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
    info!("Bichon server stopped.");
    Ok(())
}

/// Initialize the system by validating settings and starting necessary tasks.
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

#[cfg(test)]
mod tests;

#[cfg(test)]
mod api_tests {
    use super::rest::api::create_openapi_service;
    use poem::test::TestClient;

    #[tokio::test]
    async fn openapi_spec_json_is_served() {
        let api_service = create_openapi_service();
        let spec_endpoint = api_service.spec_endpoint();
        let cli = TestClient::new(spec_endpoint);

        let resp = cli.get("/").send().await;
        resp.assert_status_is_ok();

        let body = resp.json().await;
        let obj = body.value().object();
        assert!(obj.get_opt("openapi").is_some(), "missing openapi version");
        assert!(obj.get_opt("info").is_some(), "missing info section");
        assert!(obj.get_opt("paths").is_some(), "missing paths section");
    }

    #[tokio::test]
    async fn openapi_spec_yaml_is_served() {
        let api_service = create_openapi_service();
        let spec_endpoint = api_service.spec_endpoint_yaml();
        let cli = TestClient::new(spec_endpoint);

        let resp = cli.get("/").send().await;
        resp.assert_status_is_ok();
    }

    #[tokio::test]
    async fn swagger_ui_is_served() {
        let api_service = create_openapi_service();
        let swagger = api_service.swagger_ui();
        let cli = TestClient::new(swagger);

        let resp = cli.get("/").send().await;
        resp.assert_status_is_ok();
    }

    #[tokio::test]
    async fn openapi_spec_lists_all_tag_groups() {
        let api_service = create_openapi_service();
        let spec_endpoint = api_service.spec_endpoint();
        let cli = TestClient::new(spec_endpoint);

        let resp = cli.get("/").send().await;
        let body = resp.json().await;
        let value = body.value();

        let tag_names: Vec<&str> = value
            .object()
            .get("tags")
            .array()
            .iter()
            .map(|v| v.object().get("name").string())
            .collect();

        assert!(tag_names.contains(&"AccessToken"), "missing AccessToken tag");
        assert!(tag_names.contains(&"Attachment"), "missing Attachment tag");
        assert!(tag_names.contains(&"AutoConfig"), "missing AutoConfig tag");
        assert!(tag_names.contains(&"Account"), "missing Account tag");
        assert!(tag_names.contains(&"System"), "missing System tag");
        assert!(tag_names.contains(&"Mailbox"), "missing Mailbox tag");
        assert!(tag_names.contains(&"OAuth2"), "missing OAuth2 tag");
        assert!(tag_names.contains(&"Message"), "missing Message tag");
        assert!(tag_names.contains(&"Import"), "missing Import tag");
        assert!(tag_names.contains(&"Users"), "missing Users tag");
    }
}
