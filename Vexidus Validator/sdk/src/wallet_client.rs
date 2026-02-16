//! Async RPC client for wallet operations.
//!
//! Provides balance queries, nonce management, bundle submission,
//! and convenience methods for common wallet actions.
//!
//! ```ignore
//! let client = WalletClient::new("http://localhost:9933");
//! let balance = client.get_balance("Vx0abc...", "VXS").await?;
//! let tx = client.transfer(&wallet, "Vx0def...", "VXS", 5_000_000_000).await?;
//! ```

use anyhow::Result;
use serde_json::{json, Value};
use vexidus_types::TransactionBundle;

use crate::bundle::BundleBuilder;
use crate::wallet::WalletKeypair;

/// Async RPC client for wallet operations on a Vexidus node.
pub struct WalletClient {
    rpc_url: String,
    client: reqwest::Client,
}

impl WalletClient {
    /// Create a new wallet client pointing at a Vexidus node RPC endpoint.
    pub fn new(rpc_url: &str) -> Self {
        Self {
            rpc_url: rpc_url.to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Raw JSON-RPC 2.0 call.
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

    // --- Balance & Account ---

    /// Get token balance for an address.
    ///
    /// Returns the raw balance string from the node (human-readable VXS units).
    /// `token` can be "VXS" or a mint address.
    pub async fn get_balance(&self, address: &str, token: &str) -> Result<String> {
        let result = self.rpc_call("vex_getBalance", json!([address, token])).await?;
        Ok(result.as_str().unwrap_or("0").to_string())
    }

    /// Get the current nonce for an address (for replay protection).
    pub async fn get_nonce(&self, address: &str) -> Result<u64> {
        let result = self
            .rpc_call("eth_getTransactionCount", json!([address, "latest"]))
            .await?;
        let hex_str = result.as_str().unwrap_or("0x0");
        let nonce = u64::from_str_radix(hex_str.trim_start_matches("0x"), 16).unwrap_or(0);
        Ok(nonce)
    }

    // --- Transactions ---

    /// Submit a pre-signed TransactionBundle to the network.
    ///
    /// The bundle is Borsh-serialized and hex-encoded before submission
    /// via `vex_submitBundle`.
    pub async fn submit_bundle(&self, bundle: &TransactionBundle) -> Result<String> {
        let bytes = borsh::to_vec(bundle)?;
        let hex_str = format!("0x{}", hex::encode(&bytes));
        let result = self.rpc_call("vex_submitBundle", json!([hex_str])).await?;
        Ok(result.as_str().unwrap_or_default().to_string())
    }

    /// Convenience: build, sign, and submit a VXS transfer in one call.
    ///
    /// Automatically fetches the current nonce, builds a bundle, signs it,
    /// and submits it to the network.
    ///
    /// `amount` is in raw units (1 VXS = 1_000_000_000).
    pub async fn transfer(
        &self,
        wallet: &WalletKeypair,
        to: &str,
        token: &str,
        amount: u128,
    ) -> Result<String> {
        let sender = wallet.hex_address();
        let nonce = self.get_nonce(&sender).await?;

        let bundle = BundleBuilder::new(&sender)?
            .transfer(to, token, amount)?
            .nonce(nonce)
            .valid_for(3600)
            .sign(wallet);

        self.submit_bundle(&bundle).await
    }

    // --- Token Info ---

    /// Get token metadata by mint address or symbol.
    pub async fn get_token_info(&self, address_or_symbol: &str) -> Result<Value> {
        self.rpc_call("vex_getTokenInfo", json!([address_or_symbol]))
            .await
    }

    /// List tokens registered on the network.
    pub async fn list_tokens(&self, limit: u32) -> Result<Value> {
        self.rpc_call("vex_listTokens", json!([limit])).await
    }

    // --- Chain Info ---

    /// Get the chain ID (testnet: "0x18b470", mainnet: "0x18b471").
    pub async fn chain_id(&self) -> Result<String> {
        let result = self.rpc_call("eth_chainId", json!([])).await?;
        Ok(result.as_str().unwrap_or("0x0").to_string())
    }

    /// Get the current block height.
    pub async fn block_number(&self) -> Result<u64> {
        let result = self.rpc_call("eth_blockNumber", json!([])).await?;
        let hex_str = result.as_str().unwrap_or("0x0");
        let height = u64::from_str_radix(hex_str.trim_start_matches("0x"), 16).unwrap_or(0);
        Ok(height)
    }

    /// Check if the node RPC is reachable.
    pub async fn is_healthy(&self) -> bool {
        self.block_number().await.is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = WalletClient::new("http://localhost:9933");
        assert_eq!(client.rpc_url, "http://localhost:9933");
    }
}
