use clap::Parser;
use serde::{Deserialize, Serialize};

use crate::bichon_version;

pub mod admin;
pub mod auth;
pub mod eml;
pub mod mbox;
pub mod pst;
pub mod sender;
pub mod thunderbird;

#[derive(Parser, Debug)]
#[command(
    name = "bichonctl",
    author = "rustmailer",
    version = bichon_version!(),
    about = "A CLI tool to import email data into Bichon service"
)]
pub struct BichonCli {
    /// Path to the configuration file
    #[arg(
        short,
        long,
        default_value = "config.toml",
        value_name = "FILE",
        help = "Sets a custom config file"
    )]
    pub config: std::path::PathBuf,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BichonCtlConfig {
    pub base_url: String,
    pub api_token: String,
}
