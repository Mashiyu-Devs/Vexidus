//! Address conversion utilities for Vexidus's dual-format address system.
//!
//! Vexidus uses two address formats:
//! - **Vx0** (native): Base58-encoded with SHA256 checksum, 20-byte payload in 32-byte container
//! - **0x** (hex): 20-byte (EVM compat) or 32-byte (internal state)
//!
//! This module provides conversions between all formats.

use thiserror::Error;
use vexidus_types::{Address, VexidusAddress};

#[derive(Error, Debug)]
pub enum AddressError {
    #[error("Invalid address format: {0}")]
    InvalidFormat(String),
    #[error("Hex decode error: {0}")]
    HexDecode(#[from] hex::FromHexError),
    #[error("Vexidus address error: {0}")]
    Checksum(String),
}

/// Derive a Vx0 address from an Ed25519 public key.
///
/// ```ignore
/// let vx0 = vx0_from_pubkey(&pubkey_bytes); // "Vx0abc..."
/// ```
pub fn vx0_from_pubkey(pubkey: &[u8]) -> String {
    VexidusAddress::from_pubkey(pubkey).to_string()
}

/// Decode a Vx0 address to a 32-byte internal Address (20-byte payload right-aligned).
pub fn vx0_to_bytes(vx0: &str) -> Result<[u8; 32], AddressError> {
    let vx_addr: VexidusAddress = vx0
        .parse()
        .map_err(|e: String| AddressError::InvalidFormat(e))?;
    let payload = vx_addr.decode().map_err(AddressError::Checksum)?;
    let mut addr = [0u8; 32];
    if payload.len() <= 32 {
        addr[32 - payload.len()..].copy_from_slice(&payload);
    }
    Ok(addr)
}

/// Convert a Vx0 address to 0x hex string (full 32-byte representation).
///
/// ```ignore
/// let hex = vx0_to_hex("Vx0abc...")?; // "0x0000...abcd"
/// ```
pub fn vx0_to_hex(vx0: &str) -> Result<String, AddressError> {
    let bytes = vx0_to_bytes(vx0)?;
    Ok(format!("0x{}", hex::encode(bytes)))
}

/// Convert a Vx0 address to 20-byte EVM-compatible 0x address (last 20 bytes).
///
/// This is what MetaMask and other EVM wallets display.
pub fn vx0_to_evm(vx0: &str) -> Result<String, AddressError> {
    let bytes = vx0_to_bytes(vx0)?;
    Ok(format!("0x{}", hex::encode(&bytes[12..])))
}

/// Validate a Vx0/Vx1 address string (prefix + base58 + checksum).
pub fn is_valid_vx0(addr: &str) -> bool {
    if !VexidusAddress::is_valid(addr) {
        return false;
    }
    match addr.parse::<VexidusAddress>() {
        Ok(vx) => vx.decode().is_ok(),
        Err(_) => false,
    }
}

/// Validate a 0x hex address (20 or 32 bytes).
pub fn is_valid_hex_address(addr: &str) -> bool {
    let stripped = addr.strip_prefix("0x").unwrap_or(addr);
    // 40 hex chars = 20 bytes (EVM), 64 hex chars = 32 bytes (native)
    (stripped.len() == 40 || stripped.len() == 64) && hex::decode(stripped).is_ok()
}

/// Parse any address format (Vx0, Vx1, or 0x) into a 32-byte Address.
///
/// - Vx0/Vx1: Decodes base58, extracts 20-byte payload, right-aligns to 32 bytes
/// - 0x (20-byte): Right-aligns to 32 bytes
/// - 0x (32-byte): Uses directly
pub fn parse_address(input: &str) -> Result<Address, AddressError> {
    if input.starts_with("Vx0") || input.starts_with("Vx1") {
        let bytes = vx0_to_bytes(input)?;
        Ok(Address(bytes))
    } else if input.starts_with("0x") || input.starts_with("0X") {
        let hex_str = &input[2..];
        let bytes = hex::decode(hex_str)?;
        let mut addr = [0u8; 32];
        match bytes.len() {
            20 => addr[12..].copy_from_slice(&bytes),
            32 => addr.copy_from_slice(&bytes),
            n => {
                return Err(AddressError::InvalidFormat(format!(
                    "Expected 20 or 32 bytes, got {}",
                    n
                )));
            }
        }
        Ok(Address(addr))
    } else {
        Err(AddressError::InvalidFormat(format!(
            "Address must start with Vx0, Vx1, or 0x: {}",
            input
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vx0_roundtrip() {
        let pubkey = [42u8; 32];
        let vx0 = vx0_from_pubkey(&pubkey);
        assert!(vx0.starts_with("Vx0"));

        // Decode back to bytes
        let bytes = vx0_to_bytes(&vx0).unwrap();
        assert_ne!(bytes, [0u8; 32]);

        // Parse should produce same Address
        let addr = parse_address(&vx0).unwrap();
        assert_eq!(addr.0, bytes);
    }

    #[test]
    fn test_vx0_to_hex_and_evm() {
        let pubkey = [1u8; 32];
        let vx0 = vx0_from_pubkey(&pubkey);

        let hex_addr = vx0_to_hex(&vx0).unwrap();
        assert!(hex_addr.starts_with("0x"));
        assert_eq!(hex_addr.len(), 66); // "0x" + 64 hex chars

        let evm_addr = vx0_to_evm(&vx0).unwrap();
        assert!(evm_addr.starts_with("0x"));
        assert_eq!(evm_addr.len(), 42); // "0x" + 40 hex chars

        // EVM addr should be the last 20 bytes of the full hex
        assert_eq!(&hex_addr[26..], &evm_addr[2..]);
    }

    #[test]
    fn test_parse_hex_address() {
        // 20-byte EVM address
        let evm = "0x71C7656EC7ab88b098defB751B7401B5f6d8976F";
        let addr = parse_address(evm).unwrap();
        assert_eq!(&addr.0[..12], &[0u8; 12]); // First 12 bytes zero
        assert_ne!(&addr.0[12..], &[0u8; 20]); // Last 20 bytes non-zero

        // 32-byte full address
        let full = format!("0x{}", hex::encode([7u8; 32]));
        let addr = parse_address(&full).unwrap();
        assert_eq!(addr.0, [7u8; 32]);
    }

    #[test]
    fn test_validation() {
        let pubkey = [99u8; 32];
        let vx0 = vx0_from_pubkey(&pubkey);
        assert!(is_valid_vx0(&vx0));
        assert!(!is_valid_vx0("Vx0INVALID"));
        assert!(!is_valid_vx0("notanaddress"));

        assert!(is_valid_hex_address("0x71C7656EC7ab88b098defB751B7401B5f6d8976F"));
        assert!(is_valid_hex_address(&format!("0x{}", hex::encode([0u8; 32]))));
        assert!(!is_valid_hex_address("0xZZZ"));
        assert!(!is_valid_hex_address("0x1234")); // Too short
    }

    #[test]
    fn test_invalid_addresses() {
        assert!(parse_address("notanaddress").is_err());
        assert!(parse_address("0x1234").is_err()); // Wrong length
        assert!(parse_address("Vx0INVALID").is_err()); // Bad checksum
    }
}
