//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
//
// This file is part of the Bichon Email Archiving Project
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use crate::error::code::ErrorCode;
use crate::error::BichonResult;
use crate::raise_error;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use ring::hmac;
use serde::Deserialize;
use serde_json::Value;

/// Constant-time byte-slice equality. Used for comparing security-sensitive
/// values (nonces, tokens) where a variable-time compare would leak the
/// value one byte at a time through timing side-channels.
fn eq_ct(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Claims we care about for user identification. Extra claims are ignored.
#[derive(Debug, Clone, Deserialize)]
pub struct IdTokenClaims {
    pub iss: String,
    #[serde(default)]
    pub aud: Value,
    pub sub: String,
    pub exp: i64,
    #[serde(default)]
    pub iat: Option<i64>,
    #[serde(default)]
    pub nonce: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub email_verified: Option<bool>,
    #[serde(default)]
    pub preferred_username: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub given_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Header {
    alg: String,
    #[serde(default)]
    #[allow(dead_code)]
    kid: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    typ: Option<String>,
}

pub struct VerifyParams<'a> {
    pub expected_issuer: &'a str,
    pub expected_audience: &'a str,
    pub expected_nonce: &'a str,
    /// Client secret bytes, required for HS256 verification. Ignored for other algs.
    pub client_secret: &'a [u8],
    /// Clock skew tolerance in seconds.
    pub clock_skew_secs: i64,
    /// Current unix time in seconds.
    pub now_secs: i64,
}

fn split_jwt(token: &str) -> BichonResult<(&str, &str, &str)> {
    let mut parts = token.split('.');
    let header = parts.next();
    let payload = parts.next();
    let signature = parts.next();
    match (header, payload, signature, parts.next()) {
        (Some(h), Some(p), Some(s), None) => Ok((h, p, s)),
        _ => Err(raise_error!(
            "ID token is not a well-formed JWT (expected 3 segments)".into(),
            ErrorCode::InvalidParameter
        )),
    }
}

fn b64url_decode(s: &str) -> BichonResult<Vec<u8>> {
    URL_SAFE_NO_PAD.decode(s).map_err(|e| {
        raise_error!(
            format!("Failed to base64url-decode ID token segment: {}", e),
            ErrorCode::InvalidParameter
        )
    })
}

fn audience_matches(claim: &Value, expected: &str) -> bool {
    match claim {
        Value::String(s) => s == expected,
        Value::Array(arr) => arr.iter().any(|v| v.as_str() == Some(expected)),
        _ => false,
    }
}

/// Verify and parse the ID token.
///
/// The signature is verified for `HS256` using the OAuth client secret shared
/// with the IdP. Any other algorithm is rejected — asymmetric verification via
/// JWKS is intentionally not implemented yet, and silently trusting an
/// unverified signature would enable token forgery if the IdP or the transport
/// were ever compromised.
///
/// After the signature check all standard OIDC claims are validated: `iss`,
/// `aud`, `exp` (with configurable skew), and `nonce` (constant-time compare
/// to defeat timing attacks).
pub fn verify_and_parse(token: &str, params: &VerifyParams<'_>) -> BichonResult<IdTokenClaims> {
    let (h_b64, p_b64, s_b64) = split_jwt(token)?;

    let header_bytes = b64url_decode(h_b64)?;
    let header: Header = serde_json::from_slice(&header_bytes).map_err(|e| {
        raise_error!(
            format!("Failed to parse ID token header: {}", e),
            ErrorCode::InvalidParameter
        )
    })?;

    match header.alg.as_str() {
        "HS256" => {
            let signature = b64url_decode(s_b64)?;
            let signing_input = format!("{}.{}", h_b64, p_b64);
            let key = hmac::Key::new(hmac::HMAC_SHA256, params.client_secret);
            hmac::verify(&key, signing_input.as_bytes(), &signature).map_err(|_| {
                raise_error!(
                    "ID token HS256 signature verification failed".into(),
                    ErrorCode::PermissionDenied
                )
            })?;
        }
        other => {
            return Err(raise_error!(
                format!(
                    "Unsupported ID token signing algorithm '{}'. Configure your \
                     OIDC provider to use HS256, or extend Bichon with JWKS-based \
                     verification for asymmetric algorithms.",
                    other
                ),
                ErrorCode::PermissionDenied
            ));
        }
    }

    let payload_bytes = b64url_decode(p_b64)?;
    let claims: IdTokenClaims = serde_json::from_slice(&payload_bytes).map_err(|e| {
        raise_error!(
            format!("Failed to parse ID token claims: {}", e),
            ErrorCode::InvalidParameter
        )
    })?;

    if claims.iss.trim_end_matches('/') != params.expected_issuer.trim_end_matches('/') {
        return Err(raise_error!(
            format!(
                "ID token issuer mismatch: expected '{}', got '{}'",
                params.expected_issuer, claims.iss
            ),
            ErrorCode::PermissionDenied
        ));
    }

    if !audience_matches(&claims.aud, params.expected_audience) {
        return Err(raise_error!(
            "ID token audience does not include this client".into(),
            ErrorCode::PermissionDenied
        ));
    }

    if params.now_secs > claims.exp + params.clock_skew_secs {
        return Err(raise_error!(
            "ID token has expired".into(),
            ErrorCode::PermissionDenied
        ));
    }

    let nonce_ok = claims
        .nonce
        .as_deref()
        .map(|n| eq_ct(n.as_bytes(), params.expected_nonce.as_bytes()))
        .unwrap_or(false);
    if !nonce_ok {
        return Err(raise_error!(
            "ID token nonce mismatch — possible replay attack".into(),
            ErrorCode::PermissionDenied
        ));
    }

    Ok(claims)
}
