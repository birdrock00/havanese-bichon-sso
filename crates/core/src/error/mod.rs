use snafu::{Location, Snafu};

use crate::error::code::ErrorCode;

pub mod code;

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

pub type BichonResult<T, E = BichonError> = std::result::Result<T, E>;
