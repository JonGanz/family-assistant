//! Envelope encryption: per-secret random DEK (AES-256-GCM), DEK wrapped by the master key.

use std::fmt;
use std::path::Path;

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use rand::RngCore;

use crate::{Result, StoreError};

const NONCE_LEN: usize = 12;

pub struct MasterKey {
    key: [u8; 32],
    version: i32,
}

impl fmt::Debug for MasterKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MasterKey")
            .field("key", &"<redacted>")
            .field("version", &self.version)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct SealedSecret {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
    /// `dek_nonce (12 bytes) || AES-GCM(master, dek)`.
    pub wrapped_dek: Vec<u8>,
    pub key_version: i32,
}

impl fmt::Debug for SealedSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SealedSecret")
            .field(
                "ciphertext",
                &format_args!("<{} bytes>", self.ciphertext.len()),
            )
            .field("key_version", &self.key_version)
            .finish_non_exhaustive()
    }
}

impl MasterKey {
    /// Reads 32 raw bytes, or 64 hex characters (surrounding whitespace ignored).
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let bytes = std::fs::read(path.as_ref()).map_err(|e| {
            StoreError::Key(format!("cannot read {}: {e}", path.as_ref().display()))
        })?;
        Self::from_bytes(&bytes, 1)
    }

    pub fn from_bytes(bytes: &[u8], version: i32) -> Result<Self> {
        let key: [u8; 32] = if bytes.len() == 32 {
            bytes
                .try_into()
                .map_err(|_| StoreError::Key("bad length".into()))?
        } else {
            let text = std::str::from_utf8(bytes)
                .map_err(|_| {
                    StoreError::Key("key must be 32 raw bytes or 64 hex characters".into())
                })?
                .trim();
            let raw = hex::decode(text).map_err(|_| {
                StoreError::Key("key must be 32 raw bytes or 64 hex characters".into())
            })?;
            raw.try_into().map_err(|_| {
                StoreError::Key("key must be 32 raw bytes or 64 hex characters".into())
            })?
        };
        Ok(Self { key, version })
    }

    pub fn with_version(mut self, version: i32) -> Self {
        self.version = version;
        self
    }

    pub fn version(&self) -> i32 {
        self.version
    }

    fn cipher(&self) -> Aes256Gcm {
        Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.key))
    }

    pub fn seal(&self, plaintext: &[u8]) -> Result<SealedSecret> {
        let mut dek = [0u8; 32];
        rand::rng().fill_bytes(&mut dek);
        let mut nonce = [0u8; NONCE_LEN];
        rand::rng().fill_bytes(&mut nonce);
        let ciphertext = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&dek))
            .encrypt(Nonce::from_slice(&nonce), plaintext)
            .map_err(|_| StoreError::Crypto("encrypt failed"))?;
        let wrapped_dek = self.wrap(&dek)?;
        Ok(SealedSecret {
            ciphertext,
            nonce: nonce.to_vec(),
            wrapped_dek,
            key_version: self.version,
        })
    }

    pub fn open(&self, sealed: &SealedSecret) -> Result<Vec<u8>> {
        if sealed.key_version != self.version {
            return Err(StoreError::Crypto("key version mismatch"));
        }
        if sealed.nonce.len() != NONCE_LEN {
            return Err(StoreError::Crypto("bad nonce"));
        }
        let dek = self.unwrap_dek(&sealed.wrapped_dek, sealed.key_version)?;
        Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&dek))
            .decrypt(
                Nonce::from_slice(&sealed.nonce),
                sealed.ciphertext.as_slice(),
            )
            .map_err(|_| StoreError::Crypto("decrypt failed"))
    }

    /// Re-wrap the DEK under `new` without touching the ciphertext (key rotation).
    pub fn rewrap(&self, sealed: &SealedSecret, new: &MasterKey) -> Result<SealedSecret> {
        let dek = self.unwrap_dek(&sealed.wrapped_dek, sealed.key_version)?;
        Ok(SealedSecret {
            ciphertext: sealed.ciphertext.clone(),
            nonce: sealed.nonce.clone(),
            wrapped_dek: new.wrap(&dek)?,
            key_version: new.version,
        })
    }

    fn wrap(&self, dek: &[u8; 32]) -> Result<Vec<u8>> {
        let mut nonce = [0u8; NONCE_LEN];
        rand::rng().fill_bytes(&mut nonce);
        let ct = self
            .cipher()
            .encrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: dek,
                    aad: &self.version.to_be_bytes(),
                },
            )
            .map_err(|_| StoreError::Crypto("wrap failed"))?;
        let mut out = nonce.to_vec();
        out.extend(ct);
        Ok(out)
    }

    fn unwrap_dek(&self, wrapped: &[u8], version: i32) -> Result<[u8; 32]> {
        if wrapped.len() <= NONCE_LEN {
            return Err(StoreError::Crypto("bad wrapped dek"));
        }
        let (nonce, ct) = wrapped.split_at(NONCE_LEN);
        let dek = self
            .cipher()
            .decrypt(
                Nonce::from_slice(nonce),
                Payload {
                    msg: ct,
                    aad: &version.to_be_bytes(),
                },
            )
            .map_err(|_| StoreError::Crypto("unwrap failed"))?;
        dek.try_into()
            .map_err(|_| StoreError::Crypto("bad dek length"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> MasterKey {
        MasterKey::from_bytes(&[7u8; 32], 1).unwrap()
    }

    #[test]
    fn round_trip() {
        let k = key();
        let s = k.seal(b"hunter2").unwrap();
        assert_eq!(k.open(&s).unwrap(), b"hunter2");
        assert!(!format!("{s:?}").contains("hunter2"));
        assert!(!format!("{k:?}").contains('7'));
    }

    #[test]
    fn tamper_detected() {
        let k = key();
        let s = k.seal(b"hunter2").unwrap();
        let mut a = s.clone();
        a.ciphertext[0] ^= 1;
        assert!(k.open(&a).is_err());
        let mut b = s.clone();
        let last = b.wrapped_dek.len() - 1;
        b.wrapped_dek[last] ^= 1;
        assert!(k.open(&b).is_err());
        let other = MasterKey::from_bytes(&[8u8; 32], 1).unwrap();
        assert!(other.open(&s).is_err());
    }

    #[test]
    fn hex_and_raw_files() {
        let dir = tempfile::tempdir().unwrap();
        let raw = dir.path().join("raw");
        std::fs::write(&raw, [9u8; 32]).unwrap();
        let hexf = dir.path().join("hex");
        std::fs::write(&hexf, format!("{}\n", "09".repeat(32))).unwrap();
        let s = MasterKey::from_file(&raw).unwrap().seal(b"x").unwrap();
        assert_eq!(MasterKey::from_file(&hexf).unwrap().open(&s).unwrap(), b"x");
        std::fs::write(&raw, b"short").unwrap();
        assert!(MasterKey::from_file(&raw).is_err());
    }

    #[test]
    fn rotation_rewrap() {
        let old = key();
        let new = MasterKey::from_bytes(&[1u8; 32], 2).unwrap();
        let s = old.seal(b"v").unwrap();
        let r = old.rewrap(&s, &new).unwrap();
        assert_eq!(r.key_version, 2);
        assert_eq!(new.open(&r).unwrap(), b"v");
        assert!(old.open(&r).is_err());
    }
}
