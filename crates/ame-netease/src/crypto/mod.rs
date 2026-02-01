pub mod eapi;
pub mod weapi;

pub use eapi::{decrypt as eapi_decrypt, encrypt as eapi_encrypt};
pub use weapi::{encrypt as weapi_encrypt, Payload as WeapiPayload};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid key length")]
    InvalidKeyLength,
    #[error("hex decode error")]
    HexDecode(#[from] hex::FromHexError),
    #[error("utf8 error")]
    Utf8(#[from] std::string::FromUtf8Error),
}

pub type Result<T> = std::result::Result<T, Error>;
