//! # Vexidus SDK
//!
//! Toolkit for interacting with the Vexidus blockchain:
//!
//! - **Validator SDK**: Keypair management, staking operations, node configuration
//! - **Wallet SDK**: User wallet management, transaction building, balance queries
//! - **Intent SDK**: Intent-based transaction building, natural language parsing
//! - **Address utilities**: Conversion between Vx0, 0x hex, and EVM address formats

// Validator SDK
pub mod keypair;
pub mod client;
pub mod config;

// Wallet SDK
pub mod address_utils;
pub mod wallet;
pub mod bundle;
pub mod wallet_client;

// Intent SDK
pub mod intent;
pub mod intent_parser;

// DEX SDK
pub mod dex;

// Validator exports
pub use keypair::ValidatorKeypair;
pub use client::ValidatorClient;
pub use config::ValidatorConfig;

// Wallet exports
pub use wallet::{WalletKeypair, WalletError};
pub use bundle::{BundleBuilder, BundleError};
pub use wallet_client::WalletClient;
pub use address_utils::AddressError;

// Intent exports
pub use intent::{IntentBuilder, IntentError};
pub use intent_parser::{parse_intent, ParsedIntent};

// DEX exports
pub use dex::{DexClient, PoolInfo, SwapQuote};
