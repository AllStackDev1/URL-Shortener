use rand::{rng, Rng};

/// Converts a number to base62 representation (0-9, A-Z, a-z)
pub fn encode_base62(mut num: u64) -> String {
    const CHARSET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    const BASE: u64 = 62;

    if num == 0 {
        return "0".to_string();
    }

    let mut result = Vec::new();

    while num > 0 {
        result.push(CHARSET[(num % BASE) as usize]);
        num /= BASE;
    }

    // Reverse and convert to string
    result.reverse();
    String::from_utf8(result).unwrap()
}

/// Generates a random base62 character
pub fn random_base62_char() -> char {
    const CHARSET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let idx = rng().random_range(0..CHARSET.len());
    CHARSET[idx] as char
}
