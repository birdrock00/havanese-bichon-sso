use serde::{Deserialize, Serialize};

use crate::base64_decode;

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct BichonMetadata {
    pub account_email: Option<String>,
    pub mailbox_name: Option<String>,
    pub tags: Option<Vec<String>>,
}

pub fn parse_bichon_metadata(header_value: &str) -> Option<BichonMetadata> {
    let decoded = base64_decode!(header_value.trim());
    serde_json::from_slice(&decoded).ok()
}
