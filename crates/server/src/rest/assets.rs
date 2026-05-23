#[cfg(feature = "embed-web")]
mod inner {
    use rust_embed::RustEmbed;

    #[derive(RustEmbed)]
    #[folder = "../../web/dist/"]
    pub struct FrontEndAssets;
}

#[cfg(feature = "embed-web")]
pub use inner::FrontEndAssets;
