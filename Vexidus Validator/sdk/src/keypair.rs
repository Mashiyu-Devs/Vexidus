//! Validator keypair management — generate, save, load Ed25519 signing keys.

use ed25519_dalek::{SigningKey, VerifyingKey, Signer, Signature};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum KeypairError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid key format: {0}")]
    Format(String),
    #[error("Hex decode error: {0}")]
    Hex(#[from] hex::FromHexError),
}

/// Ed25519 validator signing keypair.
///
/// The secret key file is a 64-character hex string (32 bytes).
/// The public key is derived deterministically from the secret key.
pub struct ValidatorKeypair {
    signing_key: SigningKey,
}

impl ValidatorKeypair {
    /// Generate a new random keypair.
    pub fn generate() -> Self {
        let mut rng = rand_bytes();
        let signing_key = SigningKey::from_bytes(&rng);
        // Zero out stack buffer
        rng.fill(0u8);
        Self { signing_key }
    }

    /// Load from a hex-encoded secret key file.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, KeypairError> {
        let contents = std::fs::read_to_string(path)?;
        let bytes = hex::decode(contents.trim())?;
        if bytes.len() != 32 {
            return Err(KeypairError::Format(
                format!("Expected 32 bytes, got {}", bytes.len()),
            ));
        }
        let mut secret = [0u8; 32];
        secret.copy_from_slice(&bytes);
        let signing_key = SigningKey::from_bytes(&secret);
        secret.fill(0);
        Ok(Self { signing_key })
    }

    /// Save secret key as hex to a file (chmod 600).
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), KeypairError> {
        let hex_str = hex::encode(self.signing_key.to_bytes());
        std::fs::write(&path, &hex_str)?;
        // Set file permissions to owner-only on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    }

    /// Get the 32-byte public key bytes.
    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.signing_key.verifying_key().to_bytes()
    }

    /// Get the verifying key.
    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    /// Get the public key as a `vexidus_types::PublicKey`.
    pub fn public_key(&self) -> vexidus_types::PublicKey {
        vexidus_types::PublicKey(self.public_key_bytes())
    }

    /// Get the validator address (same bytes as public key for Ed25519 validators).
    pub fn address(&self) -> vexidus_types::Address {
        vexidus_types::Address(self.public_key_bytes())
    }

    /// Get public key as 0x-prefixed hex string.
    pub fn public_key_hex(&self) -> String {
        format!("0x{}", hex::encode(self.public_key_bytes()))
    }

    /// Sign a message, returning the 64-byte Ed25519 signature.
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        let sig: Signature = self.signing_key.sign(message);
        sig.to_bytes().to_vec()
    }

    /// Sign a vote message (block_hash + vote_type + epoch).
    pub fn sign_vote(&self, block_hash: &[u8; 32], vote_type: u8, epoch: u64) -> Vec<u8> {
        let mut msg = Vec::with_capacity(41);
        msg.extend_from_slice(block_hash);
        msg.push(vote_type);
        msg.extend_from_slice(&epoch.to_le_bytes());
        self.sign(&msg)
    }
}

/// Generate 32 random bytes for key generation.
fn rand_bytes() -> [u8; 32] {
    use sha2::{Sha256, Digest};
    // Use system randomness seeded with multiple entropy sources
    let mut hasher = Sha256::new();
    hasher.update(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap()
        .as_nanos().to_le_bytes());
    hasher.update(std::process::id().to_le_bytes());
    // Read 32 bytes from /dev/urandom on Unix
    #[cfg(unix)]
    {
        use std::io::Read;
        if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
            let mut buf = [0u8; 32];
            let _ = f.read_exact(&mut buf);
            hasher.update(&buf);
        }
    }
    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_sign() {
        let kp = ValidatorKeypair::generate();
        assert_eq!(kp.public_key_bytes().len(), 32);

        let msg = b"test message";
        let sig = kp.sign(msg);
        assert_eq!(sig.len(), 64);

        // Verify with ed25519-dalek
        use ed25519_dalek::Verifier;
        let ed_sig = ed25519_dalek::Signature::from_bytes(sig.as_slice().try_into().unwrap());
        assert!(kp.verifying_key().verify(msg, &ed_sig).is_ok());
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.key");

        let kp1 = ValidatorKeypair::generate();
        kp1.save(&path).unwrap();

        let kp2 = ValidatorKeypair::load(&path).unwrap();
        assert_eq!(kp1.public_key_bytes(), kp2.public_key_bytes());
    }

    #[test]
    fn test_sign_vote() {
        let kp = ValidatorKeypair::generate();
        let block_hash = [42u8; 32];
        let sig = kp.sign_vote(&block_hash, 0, 5);
        assert_eq!(sig.len(), 64);
    }
}
