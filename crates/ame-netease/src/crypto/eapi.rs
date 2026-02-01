use aes::Aes128;
use aes::cipher::{BlockDecrypt, BlockEncrypt, KeyInit};
use md5::{Digest, Md5};

use super::{Error, Result};

const KEY: &[u8; 16] = b"e82ckenh8dichen8";

pub fn encrypt(url: &str, data: &str) -> String {
    let body = format!(
        "{}-36cd479b6b5-{}-36cd479b6b5-{:x}",
        url,
        data,
        Md5::digest(format!("nobody{}use{}md5forencrypt", url, data))
    );

    let cipher = Aes128::new_from_slice(KEY).expect("key length is valid");
    let body_bytes = body.as_bytes();

    let pad_len = 16 - (body_bytes.len() % 16);
    let total_len = body_bytes.len() + pad_len;
    let mut padded = Vec::with_capacity(total_len);
    padded.extend_from_slice(body_bytes);
    padded.extend(std::iter::repeat_n(pad_len as u8, pad_len));

    let mut result = String::with_capacity(total_len * 2);
    for chunk in padded.chunks_exact(16) {
        let mut block = aes::cipher::generic_array::GenericArray::from([0u8; 16]);
        block.copy_from_slice(chunk);
        cipher.encrypt_block(&mut block);
        for &b in block.iter() {
            result.push_str(&format!("{:02X}", b));
        }
    }
    result
}

pub fn decrypt(data: &str) -> Result<Option<String>> {
    let bytes = hex::decode(data)?;
    let cipher = Aes128::new_from_slice(KEY).map_err(|_| Error::InvalidKeyLength)?;

    let mut result = Vec::with_capacity(bytes.len());
    for chunk in bytes.chunks_exact(16) {
        let mut block = aes::cipher::generic_array::GenericArray::from([0u8; 16]);
        block.copy_from_slice(chunk);
        cipher.decrypt_block(&mut block);
        result.extend_from_slice(&block);
    }

    if let Some(&pad_len) = result.last() {
        let pad_len = pad_len as usize;
        if pad_len > 0 && pad_len <= 16 && result.len() >= pad_len {
            let valid = result[result.len() - pad_len..]
                .iter()
                .all(|&b| b == pad_len as u8);
            if valid {
                result.truncate(result.len() - pad_len);
            }
        }
    }

    Ok(String::from_utf8(result).ok())
}
