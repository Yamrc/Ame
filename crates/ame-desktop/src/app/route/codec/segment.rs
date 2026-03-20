use tracing::warn;

pub(super) fn encode_path_segment(input: &str) -> String {
    let mut encoded = String::with_capacity(input.len());
    for &byte in input.as_bytes() {
        if is_unreserved(byte) {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push(hex_upper(byte >> 4));
            encoded.push(hex_upper(byte & 0x0F));
        }
    }
    encoded
}

pub(super) fn decode_path_segment(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut idx = 0usize;
    while idx < bytes.len() {
        if bytes[idx] == b'%' {
            if idx + 2 >= bytes.len() {
                return None;
            }
            let high = hex_value(bytes[idx + 1])?;
            let low = hex_value(bytes[idx + 2])?;
            decoded.push((high << 4) | low);
            idx += 3;
        } else {
            decoded.push(bytes[idx]);
            idx += 1;
        }
    }
    match String::from_utf8(decoded) {
        Ok(value) => Some(value),
        Err(err) => {
            warn!("failed to decode route path segment as utf8: {err}");
            None
        }
    }
}

fn is_unreserved(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
}

fn hex_upper(value: u8) -> char {
    match value {
        0..=9 => char::from(b'0' + value),
        _ => char::from(b'A' + (value - 10)),
    }
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}
