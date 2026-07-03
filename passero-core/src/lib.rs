pub mod crypto;
pub mod github;
pub mod store;
pub mod sync;

#[derive(thiserror::Error, Debug)]
pub enum CoreError {
    #[error("crypto error: {0}")]
    Crypto(String),
    #[error("store error: {0}")]
    Store(String),
    #[error("git error: {0}")]
    Git(#[from] git2::Error),
    #[error("http error: {0}")]
    Http(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, CoreError>;

pub fn version_tag() -> &'static str {
    "passero-core"
}
