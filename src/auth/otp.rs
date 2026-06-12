use rand::Rng;
use sha2::{Digest, Sha256};

/// Generates a random 6-digit one-time code, e.g. "042913".
pub fn generate_code() -> String {
    let code: u32 = rand::thread_rng().gen_range(0..1_000_000);
    format!("{code:06}")
}

/// Hashes a one-time code for storage (so plaintext codes are never
/// persisted in `login_challenges`). A simple SHA-256 hex digest is
/// sufficient here since the codes are short-lived, single-use, and not a
/// long-term secret like a password.
pub fn hash_code(code: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(code.as_bytes());
    hex::encode(hasher.finalize())
}
