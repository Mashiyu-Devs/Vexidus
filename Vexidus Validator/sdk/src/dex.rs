//! DEX client for interacting with VexiDEX on-chain pools.
//!
//! High-level wrapper around pool RPC endpoints. Provides swap quotes,
//! pool queries, and convenience methods that build+sign+submit bundles.
//!
//! ```ignore
//! let dex = DexClient::new("http://localhost:9933");
//! let quote = dex.quote_swap("VXS", "0x..usdc..", "1000000000").await?;
//! let tx = dex.swap(&wallet, "VXS", "0x..usdc..", 1_000_000_000, 50).await?;
//! ```

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::bundle::BundleBuilder;
use crate::wallet::WalletKeypair;

/// Pool information returned by RPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolInfo {
    pub address: String,
    pub token_a: String,
    pub token_b: String,
    pub reserve_a: String,
    pub reserve_b: String,
    pub lp_total_supply: String,
    pub lp_locked: bool,
    pub creator: String,
    pub created_at: u64,
}

/// Swap quote returned by `quote_swap`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapQuote {
    pub amount_out: String,
    pub price_impact_percent: String,
    pub fee: String,
    pub pool_address: String,
    pub reserve_in: String,
    pub reserve_out: String,
}

/// Async client for VexiDEX pool operations.
pub struct DexClient {
    rpc_url: String,
    client: reqwest::Client,
}

impl DexClient {
    /// Create a new DEX client.
    pub fn new(rpc_url: &str) -> Self {
        Self {
            rpc_url: rpc_url.to_string(),
            client: reqwest::Client::new(),
        }
    }

    async fn rpc_call(&self, method: &str, params: Value) -> Result<Value> {
        let body = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1
        });
        let resp = self
            .client
            .post(&self.rpc_url)
            .json(&body)
            .send()
            .await?
            .json::<Value>()
            .await?;
        if let Some(error) = resp.get("error") {
            anyhow::bail!("RPC error: {}", error);
        }
        Ok(resp["result"].clone())
    }

    /// Get pool info by token pair.
    pub async fn get_pool(&self, token_a: &str, token_b: &str) -> Result<PoolInfo> {
        let result = self.rpc_call("vex_getPool", json!([token_a, token_b])).await?;
        Ok(serde_json::from_value(result)?)
    }

    /// List all pools (up to limit).
    pub async fn list_pools(&self, limit: u32) -> Result<Vec<PoolInfo>> {
        let result = self.rpc_call("vex_listPools", json!([limit])).await?;
        Ok(serde_json::from_value(result)?)
    }

    /// Get a swap quote (read-only, no transaction submitted).
    pub async fn quote_swap(&self, from_token: &str, to_token: &str, amount_in: &str) -> Result<SwapQuote> {
        let result = self.rpc_call("vex_quoteSwap", json!([from_token, to_token, amount_in])).await?;
        Ok(serde_json::from_value(result)?)
    }

    /// Get the spot price of token_a in terms of token_b.
    pub async fn get_price(&self, token_a: &str, token_b: &str) -> Result<f64> {
        let pool = self.get_pool(token_a, token_b).await?;
        let ra: f64 = pool.reserve_a.parse().unwrap_or(0.0);
        let rb: f64 = pool.reserve_b.parse().unwrap_or(0.0);
        if ra == 0.0 { return Ok(0.0); }
        Ok(rb / ra)
    }

    /// Execute a swap: quote → build → sign → submit.
    ///
    /// `slippage_bps` is in basis points (e.g., 50 = 0.5%).
    pub async fn swap(
        &self,
        wallet: &WalletKeypair,
        from_token: &str,
        to_token: &str,
        amount_in: u128,
        slippage_bps: u16,
    ) -> Result<Value> {
        // Get quote to calculate min_amount_out
        let quote = self.quote_swap(from_token, to_token, &amount_in.to_string()).await?;
        let estimated_out: u128 = quote.amount_out.parse().unwrap_or(0);
        let min_out = estimated_out * (10_000 - slippage_bps as u128) / 10_000;

        let bundle = BundleBuilder::new(&wallet.hex_address())?
            .swap(from_token, to_token, amount_in, min_out)?
            .sign(wallet);

        self.submit_bundle(&bundle).await
    }

    /// Create a new liquidity pool.
    pub async fn create_pool(
        &self,
        wallet: &WalletKeypair,
        token_a: &str,
        token_b: &str,
        amount_a: u128,
        amount_b: u128,
        lp_lock_duration: u64,
    ) -> Result<Value> {
        let bundle = BundleBuilder::new(&wallet.hex_address())?
            .create_pool(token_a, token_b, amount_a, amount_b, lp_lock_duration)?
            .sign(wallet);

        self.submit_bundle(&bundle).await
    }

    /// Add liquidity to an existing pool.
    pub async fn add_liquidity(
        &self,
        wallet: &WalletKeypair,
        token_a: &str,
        token_b: &str,
        amount_a: u128,
        amount_b: u128,
        slippage_bps: u16,
    ) -> Result<Value> {
        // min_lp_tokens = 0 for simplicity (could be calculated from pool state)
        let min_lp = 0u128;
        let _ = slippage_bps; // Reserved for future precision

        let bundle = BundleBuilder::new(&wallet.hex_address())?
            .add_liquidity(token_a, token_b, amount_a, amount_b, min_lp)?
            .sign(wallet);

        self.submit_bundle(&bundle).await
    }

    /// Remove liquidity from a pool.
    pub async fn remove_liquidity(
        &self,
        wallet: &WalletKeypair,
        token_a: &str,
        token_b: &str,
        lp_amount: u128,
        slippage_bps: u16,
    ) -> Result<Value> {
        let _ = slippage_bps;
        let bundle = BundleBuilder::new(&wallet.hex_address())?
            .remove_liquidity(token_a, token_b, lp_amount, 0, 0)?
            .sign(wallet);

        self.submit_bundle(&bundle).await
    }

    async fn submit_bundle(&self, bundle: &vexidus_types::TransactionBundle) -> Result<Value> {
        let bundle_hex = hex::encode(borsh::to_vec(bundle)?);
        self.rpc_call("vex_submitBundle", json!([bundle_hex])).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dex_client_creation() {
        let client = DexClient::new("http://localhost:9933");
        assert_eq!(client.rpc_url, "http://localhost:9933");
    }
}
