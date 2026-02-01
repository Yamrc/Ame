pub mod client;
pub mod crypto;

pub use client::{Error as ClientError, NeteaseClient};
pub use crypto::{Error as CryptoError, WeapiPayload, eapi_decrypt, eapi_encrypt, weapi_encrypt};
