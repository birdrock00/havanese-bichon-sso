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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_text_from_plain_html() {
        let html = "<html><body><p>Hello World</p></body></html>".to_string();
        let text = extract_text(html);
        assert!(text.contains("Hello World"));
    }

    #[test]
    fn extract_text_strips_tags() {
        let html = "<div><h1>Title</h1><p>Paragraph with <b>bold</b> text.</p></div>".to_string();
        let text = extract_text(html);
        assert!(text.contains("Title"));
        assert!(text.contains("Paragraph"));
        assert!(text.contains("bold"));
        assert!(!text.contains("<h1>"));
        assert!(!text.contains("<b>"));
    }

    #[test]
    fn extract_text_empty_string() {
        let html = "".to_string();
        let text = extract_text(html);
        assert!(text.is_empty());
    }

    #[test]
    fn extract_text_plain_text_passthrough() {
        let html = "Just some plain text without any HTML tags.".to_string();
        let text = extract_text(html);
        assert!(text.contains("plain text"));
    }

    #[test]
    fn extract_text_with_links() {
        let html = "<a href=\"https://example.com\">Click here</a>".to_string();
        let text = extract_text(html);
        assert!(text.contains("Click here"));
    }
}
