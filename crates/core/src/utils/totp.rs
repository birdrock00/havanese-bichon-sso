// TOTP (RFC 6238) two-factor authentication — community edition.
//
// - HMAC-SHA1 is computed with `ring` (already a workspace dependency).
// - Secrets are random 160-bit values, base32-encoded (RFC 4648, no padding),
//   which is what authenticator apps expect for manual entry.
// - Verification tolerates +/- `window` time steps (30s each) for clock drift.
// - Recovery codes are hashed with argon2id and are one-time use.

use crate::error::{code::ErrorCode, BichonResult};
use crate::raise_error;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use ring::hmac::{self, HMAC_SHA1_FOR_LEGACY_USE_ONLY};
use ring::rand::{SecureRandom, SystemRandom};
use subtle::ConstantTimeEq;

/// 30-second time step, per RFC 6238.
const TIME_STEP_SECS: u64 = 30;

const BASE32_ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/// Base32 alphabet without visually ambiguous characters (no 0/O, 1/I).
const RECOVERY_ALPHABET: &[u8] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";
pub const RECOVERY_CODE_COUNT: usize = 10;
const RECOVERY_CODE_LEN: usize = 8;

// ─── Base32 (RFC 4648, unpadded) ───────────────────────────────────────

pub fn base32_encode(input: &[u8]) -> String {
    let mut out = String::with_capacity((input.len() + 4) / 5 * 8);
    let mut buffer: u32 = 0;
    let mut bits = 0u32;
    for &byte in input {
        buffer = (buffer << 8) | byte as u32;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(BASE32_ALPHABET[((buffer >> bits) & 0x1f) as usize] as char);
        }
        buffer &= (1u32 << bits) - 1;
    }
    if bits > 0 {
        let index = ((buffer << (5 - bits)) & 0x1f) as usize;
        out.push(BASE32_ALPHABET[index] as char);
    }
    out
}

pub fn base32_decode(input: &str) -> BichonResult<Vec<u8>> {
    let cleaned: String = input
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '=')
        .flat_map(|c| c.to_uppercase())
        .collect();
    if cleaned.is_empty() {
        return Err(raise_error!(
            "Empty base32 input.".into(),
            ErrorCode::InvalidParameter
        ));
    }
    let mut out = Vec::with_capacity(cleaned.len() * 5 / 8);
    let mut buffer: u32 = 0;
    let mut bits = 0u32;
    for c in cleaned.chars() {
        let index = match BASE32_ALPHABET.iter().position(|&a| a == c as u8) {
            Some(i) => i as u32,
            None => {
                return Err(raise_error!(
                    format!("Invalid base32 character '{}'.", c),
                    ErrorCode::InvalidParameter
                ))
            }
        };
        buffer = (buffer << 5) | index;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push(((buffer >> bits) & 0xff) as u8);
            buffer &= (1u32 << bits) - 1;
        }
    }
    Ok(out)
}

// ─── TOTP code generation / verification ───────────────────────────────

/// Compute a TOTP code for `secret_base32` at `timestamp_secs`, truncated to
/// `digits` (6 for production, 8 used in the RFC 6238 test vectors).
pub fn totp_code_at(
    secret_base32: &str,
    timestamp_secs: u64,
    digits: u32,
) -> BichonResult<String> {
    if digits == 0 || digits > 8 {
        return Err(raise_error!(
            "TOTP digits must be between 1 and 8.".into(),
            ErrorCode::InvalidParameter
        ));
    }
    let secret = base32_decode(secret_base32)?;
    let counter = timestamp_secs / TIME_STEP_SECS;
    let counter_bytes = counter.to_be_bytes();
    let key = hmac::Key::new(HMAC_SHA1_FOR_LEGACY_USE_ONLY, &secret);
    let tag = hmac::sign(&key, &counter_bytes);
    let mac = tag.as_ref();
    let offset = (mac[mac.len() - 1] & 0x0f) as usize;
    let binary = ((mac[offset] & 0x7f) as u32) << 24
        | (mac[offset + 1] as u32) << 16
        | (mac[offset + 2] as u32) << 8
        | mac[offset + 3] as u32;
    let code = binary % 10u32.pow(digits);
    Ok(format!("{:0width$}", code, width = digits as usize))
}

/// Verify a 6-digit code against the current time, tolerating `window` steps
/// (each 30s) of clock drift on either side.
pub fn verify_code(secret_base32: &str, code: &str, window: u8) -> bool {
    let code = code.trim();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let now_i = now as i64;
    for w in -(window as i64)..=(window as i64) {
        let ts = (now_i + w * TIME_STEP_SECS as i64).max(0) as u64;
        if let Ok(expected) = totp_code_at(secret_base32, ts, code.len() as u32) {
            let a = code.as_bytes();
            let b = expected.as_bytes();
            if a.len() == b.len() && bool::from(a.ct_eq(b)) {
                return true;
            }
        }
    }
    false
}

/// Generate a fresh 160-bit secret, base32-encoded (32 characters).
pub fn generate_secret() -> String {
    let rng = SystemRandom::new();
    let mut bytes = [0u8; 20];
    let _ = rng.fill(&mut bytes);
    base32_encode(&bytes)
}

fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            ' ' => out.push_str("%20"),
            ':' => out.push_str("%3A"),
            '/' => out.push_str("%2F"),
            '?' => out.push_str("%3F"),
            '&' => out.push_str("%26"),
            '=' => out.push_str("%3D"),
            '%' => out.push_str("%25"),
            _ => out.push(c),
        }
    }
    out
}

/// Build the `otpauth://` URI that authenticator apps use to enroll.
pub fn otpauth_uri(issuer: &str, account: &str, secret: &str) -> String {
    let issuer_enc = percent_encode(issuer);
    let account_enc = percent_encode(account);
    format!(
        "otpauth://totp/{issuer_enc}:{account_enc}?secret={secret}&issuer={issuer_enc}&algorithm=SHA1&digits=6&period=30"
    )
}

// ─── One-time recovery codes ───────────────────────────────────────────

pub fn generate_recovery_codes() -> Vec<String> {
    let rng = SystemRandom::new();
    let mut codes = Vec::with_capacity(RECOVERY_CODE_COUNT);
    for _ in 0..RECOVERY_CODE_COUNT {
        let mut bytes = [0u8; RECOVERY_CODE_LEN];
        let _ = rng.fill(&mut bytes);
        let code: String = bytes
            .iter()
            .map(|b| RECOVERY_ALPHABET[*b as usize % RECOVERY_ALPHABET.len()] as char)
            .collect();
        codes.push(code);
    }
    codes
}

/// Normalize a recovery code for comparison: uppercase, keep alphanumerics.
pub fn normalize_recovery_code(code: &str) -> String {
    code.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_uppercase())
        .collect()
}

pub fn hash_recovery_code(code: &str) -> BichonResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(code.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| {
            raise_error!(
                "Failed to hash recovery code.".into(),
                ErrorCode::InternalError
            )
        })
}

pub fn verify_hashed_code(stored: &str, code: &str) -> BichonResult<bool> {
    let parsed = PasswordHash::new(stored).map_err(|_| {
        raise_error!(
            "Stored recovery code hash is corrupt.".into(),
            ErrorCode::InternalError
        )
    })?;
    Ok(Argon2::default()
        .verify_password(code.as_bytes(), &parsed)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFC 6238 Appendix B vectors: SHA-1, secret bytes = ASCII
    // "12345678901234567890" (base32: GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ).
    const RFC_SECRET_B32: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

    fn rfc_code(ts: u64, digits: u32) -> String {
        totp_code_at(RFC_SECRET_B32, ts, digits).unwrap()
    }

    #[test]
    fn rfc6238_sha1_vectors() {
        assert_eq!(rfc_code(59, 8), "94287082");
        assert_eq!(rfc_code(1_111_111_109, 8), "07081804");
        assert_eq!(rfc_code(1_111_111_111, 8), "14050471");
        assert_eq!(rfc_code(1_234_567_890, 8), "89005924");
        assert_eq!(rfc_code(2_000_000_000, 8), "69279037");
        assert_eq!(rfc_code(20_000_000_000, 8), "65353130");
    }

    #[test]
    fn six_digit_codes_match_rfc_6238() {
        // 8-digit vector truncated to 6 digits must equal last-6 of the vector.
        assert_eq!(rfc_code(59, 6), "287082");
        assert_eq!(rfc_code(1_234_567_890, 6), "005924");
    }

    #[test]
    fn base32_vectors_unpadded() {
        assert_eq!(base32_encode(b""), "");
        assert_eq!(base32_encode(b"f"), "MY");
        assert_eq!(base32_encode(b"fo"), "MZXQ");
        assert_eq!(base32_encode(b"foo"), "MZXW6");
        assert_eq!(base32_encode(b"foob"), "MZXW6YQ");
        assert_eq!(base32_encode(b"fooba"), "MZXW6YTB");
        assert_eq!(base32_encode(b"foobar"), "MZXW6YTBOI");
    }

    #[test]
    fn base32_roundtrip() {
        let data = b"Hello, Bichon TOTP! 0123456789";
        let enc = base32_encode(data);
        assert_eq!(base32_decode(&enc).unwrap(), data);
    }

    #[test]
    fn base32_accepts_lowercase_and_padding() {
        assert_eq!(base32_decode("mzxw6===").unwrap(), b"foo");
        assert_eq!(base32_decode("mzxw6").unwrap(), b"foo");
    }

    #[test]
    fn base32_rejects_garbage() {
        assert!(base32_decode("!!!!!").is_err());
        assert!(base32_decode("").is_err());
    }

    #[test]
    fn otpauth_uri_shape() {
        let uri = otpauth_uri("Bichon", "user@example.com", "JBSWY3DPEHPK3PXP");
        assert_eq!(
            uri,
            "otpauth://totp/Bichon:user@example.com?secret=JBSWY3DPEHPK3PXP&issuer=Bichon&algorithm=SHA1&digits=6&period=30"
        );
        assert!(uri.starts_with("otpauth://totp/"));
    }

    #[test]
    fn generated_secret_is_base32_and_32_chars() {
        let secret = generate_secret();
        assert_eq!(secret.len(), 32);
        assert_eq!(base32_decode(&secret).unwrap().len(), 20);
    }

    #[test]
    fn recovery_code_hash_verify_and_normalize() {
        let codes = generate_recovery_codes();
        assert_eq!(codes.len(), RECOVERY_CODE_COUNT);
        let hashed = hash_recovery_code(&codes[0]).unwrap();
        assert!(verify_hashed_code(&hashed, &codes[0]).unwrap());
        assert!(!verify_hashed_code(&hashed, "XXXXX123").unwrap());

        let normalized = normalize_recovery_code("abcd ef12-gh34");
        assert_eq!(normalized, "ABCDEF12GH34");
    }
}
