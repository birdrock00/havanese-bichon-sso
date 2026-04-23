//use poem_openapi::Object;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct Contact {
    pub email: String,
    pub name: Option<String>,
}


