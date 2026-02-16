//! Wallet keypair management — generate, save, load Ed25519 user wallet keys.
//!
//! Distinct from [`ValidatorKeypair`](crate::ValidatorKeypair) which is used for
//! block signing in consensus. `WalletKeypair` is for end-user transaction signing
//! and includes Vx0 address derivation.

use ed25519_dalek::{SigningKey, Signer};
use std::path::Path;
use thiserror::Error;
use vexidus_types::{PublicKey, Signature, TransactionBundle};

use crate::address_utils;

#[derive(Error, Debug)]
pub enum WalletError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid key format: {0}")]
    Format(String),
    #[error("Hex decode error: {0}")]
    Hex(#[from] hex::FromHexError),
}

/// Ed25519 wallet keypair for signing transactions.
///
/// The secret key file is a 64-character hex string (32 bytes).
/// Addresses are derived deterministically from the public key.
pub struct WalletKeypair {
    signing_key: SigningKey,
}

impl WalletKeypair {
    /// Generate a new random wallet keypair.
    pub fn generate() -> Self {
        let rng = rand_bytes();
        Self {
            signing_key: SigningKey::from_bytes(&rng),
        }
    }

    /// Create from a hex-encoded secret key string.
    pub fn from_secret_hex(hex_str: &str) -> Result<Self, WalletError> {
        let bytes = hex::decode(hex_str.trim())?;
        if bytes.len() != 32 {
            return Err(WalletError::Format(format!(
                "Expected 32 bytes, got {}",
                bytes.len()
            )));
        }
        let mut secret = [0u8; 32];
        secret.copy_from_slice(&bytes);
        let signing_key = SigningKey::from_bytes(&secret);
        secret.fill(0);
        Ok(Self { signing_key })
    }

    /// Create from raw 32-byte secret key.
    pub fn from_secret_bytes(bytes: &[u8; 32]) -> Self {
        Self {
            signing_key: SigningKey::from_bytes(bytes),
        }
    }

    /// Load from a hex-encoded secret key file.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, WalletError> {
        let contents = std::fs::read_to_string(path)?;
        Self::from_secret_hex(&contents)
    }

    /// Save secret key as hex to a file (chmod 600 on Unix).
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), WalletError> {
        let hex_str = hex::encode(self.signing_key.to_bytes());
        std::fs::write(&path, &hex_str)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    }

    /// Get the native Vx0 address (user-facing format).
    pub fn vx0_address(&self) -> String {
        address_utils::vx0_from_pubkey(&self.public_key_bytes())
    }

    /// Get the full 32-byte address as 0x hex (internal state format).
    pub fn hex_address(&self) -> String {
        let bytes = address_utils::vx0_to_bytes(&self.vx0_address())
            .expect("address derived from valid pubkey");
        format!("0x{}", hex::encode(bytes))
    }

    /// Get the 20-byte EVM-compatible address (last 20 bytes, for MetaMask).
    pub fn evm_address(&self) -> String {
        address_utils::vx0_to_evm(&self.vx0_address())
            .expect("address derived from valid pubkey")
    }

    /// Get the 32-byte public key.
    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.signing_key.verifying_key().to_bytes()
    }

    /// Get the public key as a `vexidus_types::PublicKey`.
    pub fn public_key(&self) -> PublicKey {
        PublicKey(self.public_key_bytes())
    }

    /// Sign an arbitrary message, returning the 64-byte Ed25519 signature.
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        let sig: ed25519_dalek::Signature = self.signing_key.sign(message);
        sig.to_bytes().to_vec()
    }

    /// Sign a TransactionBundle, returning the Signature to set on it.
    ///
    /// Computes the Blake3 bundle hash and signs it with Ed25519.
    pub fn sign_bundle(&self, bundle: &TransactionBundle) -> Signature {
        let hash = bundle.hash();
        let sig_bytes = self.sign(hash.as_bytes());
        Signature(sig_bytes)
    }
}

/// Generate 32 random bytes for key generation.
fn rand_bytes() -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .to_le_bytes(),
    );
    hasher.update(std::process::id().to_le_bytes());
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
    use ed25519_dalek::Verifier;
    use vexidus_types::*;

    #[test]
    fn test_generate_and_addresses() {
        let wallet = WalletKeypair::generate();
        let vx0 = wallet.vx0_address();
        assert!(vx0.starts_with("Vx0"));

        let hex_addr = wallet.hex_address();
        assert!(hex_addr.starts_with("0x"));
        assert_eq!(hex_addr.len(), 66);

        let evm_addr = wallet.evm_address();
        assert!(evm_addr.starts_with("0x"));
        assert_eq!(evm_addr.len(), 42);
    }

    #[test]
    fn test_sign_and_verify() {
        let wallet = WalletKeypair::generate();
        let msg = b"hello vexidus";
        let sig_bytes = wallet.sign(msg);
        assert_eq!(sig_bytes.len(), 64);

        let ed_sig =
            ed25519_dalek::Signature::from_bytes(sig_bytes.as_slice().try_into().unwrap());
        assert!(wallet.signing_key.verifying_key().verify(msg, &ed_sig).is_ok());
    }

    #[test]
    fn test_sign_bundle() {
        let wallet = WalletKeypair::generate();
        let bundle = TransactionBundle {
            user_account: Address([1u8; 32]),
            operations: vec![Operation::Transfer {
                to: Address([2u8; 32]),
                token: Address::ZERO,
                amount: Amount(1_000_000_000),
            }],
            max_gas: 100_000,
            max_priority_fee: 0,
            valid_until: Timestamp::now(),
            nonce: Nonce::zero(),
            signature: Signature(vec![]),
            expiry_timestamp: None,
            sender_pubkey: None,
        };

        let sig = wallet.sign_bundle(&bundle);
        assert_eq!(sig.0.len(), 64);

        // Verify using the types-level verify
        let mut signed_bundle = bundle;
        signed_bundle.signature = sig;
        assert!(signed_bundle.verify_signature(&wallet.public_key()));
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wallet.key");

        let w1 = WalletKeypair::generate();
        w1.save(&path).unwrap();

        let w2 = WalletKeypair::load(&path).unwrap();
        assert_eq!(w1.public_key_bytes(), w2.public_key_bytes());
        assert_eq!(w1.vx0_address(), w2.vx0_address());
    }

    #[test]
    fn test_from_secret_hex() {
        let wallet = WalletKeypair::generate();
        let hex_str = hex::encode(wallet.signing_key.to_bytes());
        let loaded = WalletKeypair::from_secret_hex(&hex_str).unwrap();
        assert_eq!(wallet.public_key_bytes(), loaded.public_key_bytes());
    }
}
