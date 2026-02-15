//! Transaction bundle builder with a fluent API.
//!
//! Constructs [`TransactionBundle`]s for submission to the Vexidus network.
//! Handles address parsing (accepts both Vx0 and 0x formats) and provides
//! sensible defaults for gas, validity, and nonce.
//!
//! ```ignore
//! let bundle = BundleBuilder::new("Vx0abc...")?
//!     .transfer("Vx0def...", "VXS", 5_000_000_000)?
//!     .nonce(3)
//!     .max_gas(100_000)
//!     .valid_for(3600)
//!     .sign(&wallet);
//! ```

use thiserror::Error;
use vexidus_types::*;

use crate::address_utils::{self, AddressError};
use crate::wallet::WalletKeypair;

#[derive(Error, Debug)]
pub enum BundleError {
    #[error("Address error: {0}")]
    Address(#[from] AddressError),
    #[error("No operations specified")]
    NoOperations,
}

/// Fluent builder for constructing transaction bundles.
pub struct BundleBuilder {
    sender: Address,
    operations: Vec<Operation>,
    max_gas: u64,
    max_priority_fee: u64,
    valid_until: Timestamp,
    nonce: u64,
}

impl BundleBuilder {
    /// Create a new builder for the given sender address (Vx0 or 0x).
    pub fn new(sender: &str) -> Result<Self, BundleError> {
        let addr = address_utils::parse_address(sender)?;
        Ok(Self {
            sender: addr,
            operations: Vec::new(),
            max_gas: 100_000,
            max_priority_fee: 0,
            valid_until: Timestamp::now() + 3600,
            nonce: 0,
        })
    }

    // --- Operations ---

    /// Add a token transfer operation.
    ///
    /// `token` can be "VXS" (native token) or a Vx1/0x mint address.
    /// `amount` is in raw units (1 VXS = 1_000_000_000 for 9-decimal tokens).
    pub fn transfer(mut self, to: &str, token: &str, amount: u128) -> Result<Self, BundleError> {
        let to_addr = address_utils::parse_address(to)?;
        let token_addr = parse_token(token)?;
        self.operations.push(Operation::Transfer {
            to: to_addr,
            token: token_addr,
            amount: Amount(amount),
        });
        Ok(self)
    }

    /// Add a VSA v2 AddKey operation.
    pub fn add_key(mut self, pubkey: Vec<u8>, key_type: KeyType, role: KeyRole) -> Self {
        self.operations
            .push(Operation::AddKey { pubkey, key_type, role });
        self
    }

    /// Add a VSA v2 RemoveKey operation.
    pub fn remove_key(mut self, pubkey_hash: Hash) -> Self {
        self.operations
            .push(Operation::RemoveKey { pubkey_hash });
        self
    }

    /// Add a VSA v2 RotateKey operation.
    pub fn rotate_key(
        mut self,
        old_pubkey_hash: Hash,
        new_pubkey: Vec<u8>,
        new_key_type: KeyType,
    ) -> Self {
        self.operations.push(Operation::RotateKey {
            old_pubkey_hash,
            new_pubkey,
            new_key_type,
        });
        self
    }

    /// Add a Stake operation to register as a validator.
    pub fn stake(mut self, amount: u128, validator_pubkey: Vec<u8>) -> Self {
        self.operations.push(Operation::Stake {
            amount: Amount(amount),
            validator_pubkey,
        });
        self
    }

    /// Add an Unstake operation (begins 21-day unbonding).
    pub fn unstake(mut self, amount: u128) -> Self {
        self.operations.push(Operation::Unstake {
            amount: Amount(amount),
        });
        self
    }

    /// Add a ClaimUnstake operation.
    pub fn claim_unstake(mut self) -> Self {
        self.operations.push(Operation::ClaimUnstake);
        self
    }

    /// Add a Delegate operation.
    pub fn delegate(mut self, validator: &str, amount: u128) -> Result<Self, BundleError> {
        let validator_addr = address_utils::parse_address(validator)?;
        self.operations.push(Operation::Delegate {
            validator: validator_addr,
            amount: Amount(amount),
        });
        Ok(self)
    }

    /// Add an Undelegate operation.
    pub fn undelegate(mut self, validator: &str, amount: u128) -> Result<Self, BundleError> {
        let validator_addr = address_utils::parse_address(validator)?;
        self.operations.push(Operation::Undelegate {
            validator: validator_addr,
            amount: Amount(amount),
        });
        Ok(self)
    }

    /// Add a ClaimRewards operation.
    pub fn claim_rewards(mut self) -> Self {
        self.operations.push(Operation::ClaimRewards);
        self
    }

    /// Set validator commission rate (basis points, max 5000 = 50%).
    pub fn set_commission(mut self, rate: u16) -> Self {
        self.operations.push(Operation::SetCommission { rate });
        self
    }

    /// Self-unjail after jail period has elapsed.
    pub fn unjail(mut self) -> Self {
        self.operations.push(Operation::Unjail);
        self
    }

    /// Set on-chain validator profile metadata.
    pub fn set_validator_metadata(
        mut self,
        name: String,
        description: String,
        website: String,
        avatar_url: String,
    ) -> Self {
        self.operations.push(Operation::SetValidatorMetadata {
            name, description, website, avatar_url,
        });
        self
    }

    // --- DEX Operations ---

    /// Create a new liquidity pool.
    pub fn create_pool(
        mut self,
        token_a: &str,
        token_b: &str,
        amount_a: u128,
        amount_b: u128,
        lp_lock_duration: u64,
    ) -> Result<Self, BundleError> {
        let addr_a = parse_token(token_a)?;
        let addr_b = parse_token(token_b)?;
        self.operations.push(Operation::CreatePool {
            token_a: addr_a,
            token_b: addr_b,
            amount_a,
            amount_b,
            lp_lock_duration,
        });
        self.max_gas = self.max_gas.max(300_000);
        Ok(self)
    }

    /// Add liquidity to an existing pool.
    pub fn add_liquidity(
        mut self,
        token_a: &str,
        token_b: &str,
        amount_a: u128,
        amount_b: u128,
        min_lp_tokens: u128,
    ) -> Result<Self, BundleError> {
        let addr_a = parse_token(token_a)?;
        let addr_b = parse_token(token_b)?;
        self.operations.push(Operation::AddLiquidity {
            token_a: addr_a,
            token_b: addr_b,
            amount_a,
            amount_b,
            min_lp_tokens,
        });
        self.max_gas = self.max_gas.max(150_000);
        Ok(self)
    }

    /// Remove liquidity from a pool.
    pub fn remove_liquidity(
        mut self,
        token_a: &str,
        token_b: &str,
        lp_amount: u128,
        min_amount_a: u128,
        min_amount_b: u128,
    ) -> Result<Self, BundleError> {
        let addr_a = parse_token(token_a)?;
        let addr_b = parse_token(token_b)?;
        self.operations.push(Operation::RemoveLiquidity {
            token_a: addr_a,
            token_b: addr_b,
            lp_amount,
            min_amount_a,
            min_amount_b,
        });
        self.max_gas = self.max_gas.max(150_000);
        Ok(self)
    }

    /// Swap tokens through an on-chain pool.
    pub fn swap(
        mut self,
        from_token: &str,
        to_token: &str,
        amount_in: u128,
        min_amount_out: u128,
    ) -> Result<Self, BundleError> {
        let from_addr = parse_token(from_token)?;
        let to_addr = parse_token(to_token)?;
        self.operations.push(Operation::Swap {
            from_token: from_addr,
            to_token: to_addr,
            amount_in,
            min_amount_out,
        });
        Ok(self)
    }

    // --- Configuration ---

    /// Set the nonce (replay protection). Must match the account's current nonce.
    pub fn nonce(mut self, n: u64) -> Self {
        self.nonce = n;
        self
    }

    /// Set the maximum gas the sender is willing to pay.
    pub fn max_gas(mut self, g: u64) -> Self {
        self.max_gas = g;
        self
    }

    /// Set the maximum priority fee per gas.
    pub fn max_priority_fee(mut self, f: u64) -> Self {
        self.max_priority_fee = f;
        self
    }

    /// Set the validity window in seconds from now.
    pub fn valid_for(mut self, seconds: u64) -> Self {
        self.valid_until = Timestamp::now() + seconds;
        self
    }

    // --- Build ---

    /// Build an unsigned bundle (empty signature).
    pub fn build(self) -> TransactionBundle {
        TransactionBundle {
            user_account: self.sender,
            operations: self.operations,
            max_gas: self.max_gas,
            max_priority_fee: self.max_priority_fee,
            valid_until: self.valid_until,
            nonce: Nonce(self.nonce),
            signature: Signature(vec![]),
            expiry_timestamp: None,
        }
    }

    /// Build and sign the bundle with a wallet keypair.
    pub fn sign(self, wallet: &WalletKeypair) -> TransactionBundle {
        let mut bundle = self.build();
        bundle.signature = wallet.sign_bundle(&bundle);
        bundle
    }
}

/// Parse a token identifier: "VXS" → Address::ZERO, otherwise parse as address.
fn parse_token(token: &str) -> Result<Address, AddressError> {
    if token.eq_ignore_ascii_case("VXS") {
        Ok(Address::ZERO)
    } else {
        address_utils::parse_address(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_transfer() {
        let sender = format!("0x{}", hex::encode([1u8; 32]));
        let to = format!("0x{}", hex::encode([2u8; 32]));

        let bundle = BundleBuilder::new(&sender)
            .unwrap()
            .transfer(&to, "VXS", 1_000_000_000)
            .unwrap()
            .nonce(5)
            .max_gas(50_000)
            .build();

        assert_eq!(bundle.operations.len(), 1);
        assert_eq!(bundle.nonce.value(), 5);
        assert_eq!(bundle.max_gas, 50_000);
        assert!(bundle.signature.0.is_empty()); // Unsigned
    }

    #[test]
    fn test_sign_transfer() {
        let wallet = WalletKeypair::generate();
        let sender_hex = wallet.hex_address();
        let to = format!("0x{}", hex::encode([2u8; 32]));

        let bundle = BundleBuilder::new(&sender_hex)
            .unwrap()
            .transfer(&to, "VXS", 5_000_000_000)
            .unwrap()
            .sign(&wallet);

        assert_eq!(bundle.signature.0.len(), 64);
        assert!(bundle.verify_signature(&wallet.public_key()));
    }

    #[test]
    fn test_multi_operation_bundle() {
        let sender = format!("0x{}", hex::encode([1u8; 32]));
        let to = format!("0x{}", hex::encode([2u8; 32]));
        let validator = format!("0x{}", hex::encode([3u8; 32]));

        let bundle = BundleBuilder::new(&sender)
            .unwrap()
            .transfer(&to, "VXS", 1_000_000_000)
            .unwrap()
            .delegate(&validator, 50_000_000_000_000)
            .unwrap()
            .build();

        assert_eq!(bundle.operations.len(), 2);
    }

    #[test]
    fn test_vxs_token_shorthand() {
        let sender = format!("0x{}", hex::encode([1u8; 32]));
        let to = format!("0x{}", hex::encode([2u8; 32]));

        let bundle = BundleBuilder::new(&sender)
            .unwrap()
            .transfer(&to, "vxs", 100)
            .unwrap()
            .build();

        if let Operation::Transfer { token, .. } = &bundle.operations[0] {
            assert_eq!(*token, Address::ZERO);
        } else {
            panic!("Expected Transfer operation");
        }
    }

    #[test]
    fn test_staking_operations() {
        let sender = format!("0x{}", hex::encode([1u8; 32]));

        let bundle = BundleBuilder::new(&sender)
            .unwrap()
            .stake(100_000_000_000_000, vec![42u8; 32])
            .unstake(50_000_000_000_000)
            .claim_rewards()
            .build();

        assert_eq!(bundle.operations.len(), 3);
    }

    #[test]
    fn test_create_pool() {
        let sender = format!("0x{}", hex::encode([1u8; 32]));
        let token_b = format!("0x{}", hex::encode([5u8; 32]));

        let bundle = BundleBuilder::new(&sender)
            .unwrap()
            .create_pool("VXS", &token_b, 1_000_000_000, 500_000_000, 0)
            .unwrap()
            .build();

        assert_eq!(bundle.operations.len(), 1);
        assert_eq!(bundle.max_gas, 300_000);
        match &bundle.operations[0] {
            Operation::CreatePool { token_a, amount_a, .. } => {
                assert_eq!(*token_a, Address::ZERO);
                assert_eq!(*amount_a, 1_000_000_000);
            }
            _ => panic!("Expected CreatePool"),
        }
    }

    #[test]
    fn test_swap_operation() {
        let sender = format!("0x{}", hex::encode([1u8; 32]));
        let token_b = format!("0x{}", hex::encode([5u8; 32]));

        let bundle = BundleBuilder::new(&sender)
            .unwrap()
            .swap("VXS", &token_b, 100_000_000_000, 95_000_000_000)
            .unwrap()
            .build();

        assert_eq!(bundle.operations.len(), 1);
        match &bundle.operations[0] {
            Operation::Swap { from_token, amount_in, min_amount_out, .. } => {
                assert_eq!(*from_token, Address::ZERO);
                assert_eq!(*amount_in, 100_000_000_000);
                assert_eq!(*min_amount_out, 95_000_000_000);
            }
            _ => panic!("Expected Swap"),
        }
    }
}
