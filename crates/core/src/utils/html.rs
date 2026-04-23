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


use std::panic;
use tracing::error;

pub fn extract_text(html: String) -> String {
    let result = panic::catch_unwind(|| {
        html2text::config::plain()
            .allow_width_overflow()
            .string_from_read(html.as_bytes(), 100)
    });

    match result {
        Ok(Ok(text)) => text,
        Ok(Err(err)) => {
            error!("html2text error: {}", err);
            html
        }
        Err(err) => {
            if let Some(s) = err.downcast_ref::<&str>() {
                error!("html2text panic: {}", s);
            } else if let Some(s) = err.downcast_ref::<String>() {
                error!("html2text panic: {}", s);
            } else {
                error!("html2text panic: unknown error");
            }
            html
        }
    }
}
