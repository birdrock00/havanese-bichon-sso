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

use console::style;
use dialoguer::{theme::ColorfulTheme, Select};

use crate::{migrate::handle_migration, reset::handle_reset_password};

pub mod meta;
pub mod migrate;
pub mod reset;


fn main() {
    run_interactive();
}

#[tokio::main]
async fn run_interactive() {
    let theme = ColorfulTheme::default();
    println!(
        "\n{}\n",
        style("BICHON ADMINISTRATIVE TOOL").bold().bright().cyan()
    );

    let main_options = vec![
        "Reset Admin Password",
        "Migrate Legacy v0.3.7 Storage to v1.x",
        "Exit",
    ];

    let selection = Select::with_theme(&theme)
        .with_prompt("Select an operation")
        .default(0)
        .items(&main_options)
        .interact()
        .unwrap();

    match selection {
        0 => handle_reset_password(&theme),
        1 => handle_migration(&theme),
        _ => {
            println!("{}", style("Exiting...").dim());
        }
    }
}
