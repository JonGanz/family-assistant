use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};

use crate::error::{AuthError, Result};

// OWASP Password Storage Cheat Sheet, argon2id minimum profile.
const MEMORY_KIB: u32 = 19 * 1024;
const ITERATIONS: u32 = 2;
const LANES: u32 = 1;

fn hasher() -> Result<Argon2<'static>> {
    let params = Params::new(MEMORY_KIB, ITERATIONS, LANES, None)
        .map_err(|e| AuthError::Internal(e.to_string()))?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
}

fn salt() -> Result<SaltString> {
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut bytes);
    SaltString::encode_b64(&bytes).map_err(|e| AuthError::Internal(e.to_string()))
}

/// Hashes a secret (password or PIN) to a PHC string. CPU-heavy: call from a blocking context.
pub fn hash_password(password: &str) -> Result<String> {
    hasher()?
        .hash_password(password.as_bytes(), &salt()?)
        .map(|h| h.to_string())
        .map_err(|e| AuthError::Internal(e.to_string()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordCheck {
    Mismatch,
    /// `needs_rehash`: the stored hash uses weaker parameters than today's.
    Match {
        needs_rehash: bool,
    },
}

/// Constant-time verification against a PHC string.
pub fn verify_password(password: &str, hash: &str) -> Result<PasswordCheck> {
    let parsed = PasswordHash::new(hash).map_err(|e| AuthError::Internal(e.to_string()))?;
    match Argon2::default().verify_password(password.as_bytes(), &parsed) {
        Ok(()) => Ok(PasswordCheck::Match {
            needs_rehash: needs_rehash(&parsed),
        }),
        Err(argon2::password_hash::Error::Password) => Ok(PasswordCheck::Mismatch),
        Err(e) => Err(AuthError::Internal(e.to_string())),
    }
}

fn needs_rehash(h: &PasswordHash<'_>) -> bool {
    if h.algorithm.as_str() != "argon2id" {
        return true;
    }
    match Params::try_from(h) {
        Ok(p) => p.m_cost() < MEMORY_KIB || p.t_cost() < ITERATIONS || p.p_cost() != LANES,
        Err(_) => true,
    }
}

/// Async wrappers: argon2 runs on the blocking pool.
pub(crate) async fn hash_blocking(secret: String) -> Result<String> {
    tokio::task::spawn_blocking(move || hash_password(&secret))
        .await
        .map_err(|e| AuthError::Internal(e.to_string()))?
}

pub(crate) async fn verify_blocking(secret: String, hash: String) -> Result<PasswordCheck> {
    tokio::task::spawn_blocking(move || verify_password(&secret, &hash))
        .await
        .map_err(|e| AuthError::Internal(e.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use fa_domain::{Role, validate_password};

    #[test]
    fn hash_and_verify() {
        let h = hash_password("correct horse battery").unwrap();
        assert!(h.starts_with("$argon2id$"));
        assert_eq!(
            verify_password("correct horse battery", &h).unwrap(),
            PasswordCheck::Match {
                needs_rehash: false
            }
        );
        assert_eq!(
            verify_password("wrong", &h).unwrap(),
            PasswordCheck::Mismatch
        );
        assert_ne!(h, hash_password("correct horse battery").unwrap(), "salted");
    }

    #[test]
    fn weak_params_need_rehash() {
        let weak = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(8, 1, 1, None).unwrap(),
        );
        let h = weak
            .hash_password(b"pw-for-testing", &salt().unwrap())
            .unwrap()
            .to_string();
        assert_eq!(
            verify_password("pw-for-testing", &h).unwrap(),
            PasswordCheck::Match { needs_rehash: true }
        );
    }

    #[test]
    fn policy_comes_from_domain() {
        assert!(validate_password(Role::Adult, "short").is_err());
        assert!(validate_password(Role::Adult, "twelve chars!").is_ok());
        assert!(validate_password(Role::Child, "abcd").is_ok());
        assert!(validate_password(Role::Child, "abc").is_err());
    }
}
