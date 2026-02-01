pub mod client;
pub mod crypto;

pub use client::{Error, NeteaseClient};
pub use crypto::{WeapiPayload, eapi_decrypt, eapi_encrypt, weapi_encrypt};
