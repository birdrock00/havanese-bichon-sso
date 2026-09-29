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

//! Thin binary half of `bichon-admin`: everything lives in the library so
//! the Pro build can ship its own `bichon-admin` on top of the same code.

use bichon_admin::{AdminCli, AdminCommand};
use clap::Parser;

fn main() {
    let cli = AdminCli::parse();
    match cli.command {
        Some(AdminCommand::Restore(args)) => {
            let code = tokio::runtime::Runtime::new()
                .expect("tokio runtime")
                .block_on(bichon_admin::restore::run_cli(args, false));
            std::process::exit(code);
        }
        None => bichon_admin::run_interactive(false),
    }
}
