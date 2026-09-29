use snafu::{Location, Snafu};

use crate::error::code::ErrorCode;

pub mod code;

// Re-exported so the `raise_error!` macro can resolve `snafu::location!`
// through `$crate` at call sites in other crates (which do not depend on
// snafu directly).
pub use snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum BichonError {
    #[snafu(display("{message}"))]
    Generic {
        message: String,
        #[snafu(implicit)]
        location: Location,
        code: ErrorCode,
    },
}

impl BichonError {
    pub fn code(&self) -> ErrorCode {
        match self {
            BichonError::Generic { code, .. } => *code,
        }
    }
}

pub type BichonResult<T, E = BichonError> = std::result::Result<T, E>;
