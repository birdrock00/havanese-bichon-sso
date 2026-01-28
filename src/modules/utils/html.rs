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
