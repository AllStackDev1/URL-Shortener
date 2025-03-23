use rand::{rng, Rng};

use super::hash::{encode_base62, random_base62_char};

/// Generates a unique short ID for URL shortening using base62 encoding of random values
pub fn generate_short_id(length: usize) -> String {
    // Generate a random 64-bit number
    let random_id: u64 = rng().random();

    // Encode it using base62
    let mut encoded = encode_base62(random_id);

    // Ensure the ID is of desired length
    // If too short, pad with additional random characters
    while encoded.len() < length {
        encoded.push(random_base62_char());
    }

    // If too long, truncate
    if encoded.len() > length {
        encoded.truncate(length);
    }

    encoded
}
