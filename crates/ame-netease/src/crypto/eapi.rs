use aes::Aes128;
use aes::cipher::{BlockDecrypt, BlockEncrypt, KeyInit};
use md5::{Digest, Md5};

const KEY: &[u8; 16] = b"e82ckenh8dichen8";

pub fn encrypt(url: &str, data: &str) -> String {
    let body = format!(
        "{}-36cd479b6b5-{}-36cd479b6b5-{:x}",
        url,
        data,
        Md5::digest(format!("nobody{}use{}md5forencrypt", url, data))
    );

    let cipher = Aes128::new_from_slice(KEY).unwrap();

    let body_bytes = body.as_bytes();
    let pad_len = 16 - (body_bytes.len() % 16);
    let mut padded: Vec<u8> = body_bytes.to_vec();
    padded.extend(std::iter::repeat(pad_len as u8).take(pad_len));

    let mut blocks: Vec<_> = padded
        .chunks(16)
        .map(|chunk| {
            let mut block = [0u8; 16];
            block.copy_from_slice(chunk);
            aes::cipher::generic_array::GenericArray::from(block)
        })
        .collect();

    cipher.encrypt_blocks(&mut blocks);

    blocks
        .iter()
        .flat_map(|b| b.iter())
        .map(|b| format!("{:02X}", b))
        .collect()
}

pub fn decrypt(data: &str) -> Option<String> {
    let bytes: Vec<u8> = (0..data.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&data[i..i + 2], 16).ok())
        .collect::<Option<Vec<_>>>()?;

    let cipher = Aes128::new_from_slice(KEY).unwrap();
    let mut blocks: Vec<_> = bytes
        .chunks(16)
        .map(|chunk| {
            let mut block = [0u8; 16];
            block.copy_from_slice(chunk);
            aes::cipher::generic_array::GenericArray::from(block)
        })
        .collect();

    cipher.decrypt_blocks(&mut blocks);

    let mut result: Vec<u8> = blocks.iter().flat_map(|b| b.iter().copied()).collect();

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

    String::from_utf8(result).ok()
}
