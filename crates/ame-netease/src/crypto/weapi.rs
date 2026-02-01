use aes::Aes128;
use aes::cipher::BlockEncryptMut;
use base64::Engine;
use cbc::cipher::KeyIvInit;

const PRESET_KEY: &[u8; 16] = b"0CoJUm6Qyw8W8jud";
const IV: &[u8; 16] = b"0102030405060708";
const BASE62: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

const PUBKEY_N: &[u8] = b"00e0b509f6259df8642dbc35662901477df22677ec152b5ff68ace615bb7b725152b3ab17a876aea8a5aa76d2e417629ec4ee341f56135fccf695280104e0312ecbda92557c93870114af6c9d05c4f7f0c3685b7a46bee255932575cce10b424d813cfe4875d3e82047b97ddef52741d546b8e289dc6935b3ece0462db0a22b8e7";
const PUBKEY_E: u32 = 65537;

pub struct Payload {
    pub params: String,
    pub enc_sec_key: String,
}

pub fn encrypt(text: &str) -> Payload {
    use rand::Rng;
    let mut rng = rand::rng();
    let secret_key: Vec<u8> = (0..16).map(|_| BASE62[rng.random_range(0..62)]).collect();
    encrypt_with_key(text, &secret_key)
}

pub fn encrypt_with_key(text: &str, secret_key: &[u8]) -> Payload {
    let first_encrypted = aes_cbc_encrypt(text.as_bytes(), PRESET_KEY, IV);
    let first_b64 = base64::engine::general_purpose::STANDARD.encode(&first_encrypted);

    let second_encrypted = aes_cbc_encrypt(first_b64.as_bytes(), secret_key, IV);
    let params = base64::engine::general_purpose::STANDARD.encode(&second_encrypted);

    let enc_sec_key = rsa_encrypt(&secret_key.iter().rev().copied().collect::<Vec<_>>());

    Payload {
        params,
        enc_sec_key,
    }
}

fn aes_cbc_encrypt(data: &[u8], key: &[u8], iv: &[u8]) -> Vec<u8> {
    type Aes128CbcEnc = cbc::Encryptor<Aes128>;

    let pad_len = 16 - (data.len() % 16);
    let mut buf = data.to_vec();
    buf.extend(std::iter::repeat_n(pad_len as u8, pad_len));

    let encryptor = Aes128CbcEnc::new(key.into(), iv.into());
    let len = buf.len();
    encryptor
        .encrypt_padded_mut::<aes::cipher::block_padding::NoPadding>(&mut buf, len)
        .expect("padding is correct");

    buf
}

fn rsa_encrypt(data: &[u8]) -> String {
    let n = num_bigint::BigUint::parse_bytes(PUBKEY_N, 16).expect("pubkey is valid hex");
    let e = num_bigint::BigUint::from(PUBKEY_E);
    let m = num_bigint::BigUint::from_bytes_be(&pad_rsa(data));
    format!("{:0>256x}", m.modpow(&e, &n))
}

fn pad_rsa(data: &[u8]) -> Vec<u8> {
    let mut result = vec![0u8; 128];
    result[128 - data.len()..].copy_from_slice(data);
    result
}
