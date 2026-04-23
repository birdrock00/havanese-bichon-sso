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


use tokio::signal;

pub(crate) async fn shutdown_signal() {
    let ctrl_c_signal = async {
        signal::ctrl_c()
            .await
            .expect("Error installing Ctrl+C signal handler");
    };

    #[cfg(unix)]
    let terminate_signal = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Error installing terminate signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate_signal = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c_signal => {},
        _ = terminate_signal => {},
    };
}
