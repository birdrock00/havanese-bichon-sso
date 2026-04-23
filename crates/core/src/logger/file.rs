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

use crate::logger::LocalTimer;
use crate::settings::cli::SETTINGS;
use crate::settings::dir::DATA_DIR_MANAGER;
use std::sync::OnceLock;
use tracing::level_filters::LevelFilter;
use tracing::Level;
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;

pub static LOG_WORKER_GUARD: OnceLock<Vec<WorkerGuard>> = OnceLock::new();

pub fn setup_file_logger(level: Level) -> Result<(), tracing::dispatcher::SetGlobalDefaultError> {
    let with_ansi = SETTINGS.bichon_ansi_logs;

    let (server_nonb, server_guard) = server_log_writer();
    LOG_WORKER_GUARD.set(vec![server_guard]).unwrap();

    let server_layer = fmt::layer()
        .with_timer(LocalTimer)
        .with_ansi(with_ansi)
        .with_level(true)
        .with_writer(server_nonb)
        .with_target(true);

    let subscriber = tracing_subscriber::registry()
        .with(LevelFilter::from_level(level))
        .with(server_layer);

    // Set the combined subscriber as the global default
    tracing::subscriber::set_global_default(subscriber)
}

fn server_log_writer() -> (NonBlocking, WorkerGuard) {
    let rolling = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("server")
        .max_log_files(SETTINGS.bichon_max_server_log_files)
        .build(DATA_DIR_MANAGER.log_dir.clone())
        .expect("failed to initialize rolling file appender");
    let (nb, wg) = tracing_appender::non_blocking(rolling);
    (nb, wg)
}
