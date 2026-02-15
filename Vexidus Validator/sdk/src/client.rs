//! RPC client for interacting with a Vexidus node.

use anyhow::Result;
use serde_json::{json, Value};

/// Vexidus validator RPC client.
pub struct ValidatorClient {
    rpc_url: String,
    client: reqwest::Client,
}

impl ValidatorClient {
    /// Create a new client pointing at a Vexidus node RPC endpoint.
    pub fn new(rpc_url: &str) -> Self {
        Self {
            rpc_url: rpc_url.to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Raw JSON-RPC call.
    async fn rpc_call(&self, method: &str, params: Value) -> Result<Value> {
        let body = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1
        });

        let resp = self.client
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

    // --- Staking Operations ---

    /// Stake VXS to register as a validator.
    /// `from`: staker address (hex), `amount`: VXS in human units (e.g. "100000"),
    /// `validator_pubkey`: 64 hex chars of the Ed25519 public key.
    pub async fn stake(&self, from: &str, amount: &str, validator_pubkey: &str) -> Result<String> {
        let result = self.rpc_call("vex_stake", json!([from, amount, validator_pubkey])).await?;
        Ok(result.as_str().unwrap_or_default().to_string())
    }

    /// Begin unstaking VXS (starts 21-day unbonding).
    pub async fn unstake(&self, from: &str, amount: &str) -> Result<String> {
        let result = self.rpc_call("vex_unstake", json!([from, amount])).await?;
        Ok(result.as_str().unwrap_or_default().to_string())
    }

    /// Set validator commission rate.
    /// `rate` is in basis points (100 = 1%, max 5000 = 50%).
    pub async fn set_commission(&self, from: &str, rate: u16) -> Result<String> {
        let result = self.rpc_call("vex_setCommission", json!([from, rate])).await?;
        Ok(result.as_str().unwrap_or_default().to_string())
    }

    /// Self-unjail after the jail cooldown period has elapsed.
    pub async fn unjail(&self, from: &str) -> Result<String> {
        let result = self.rpc_call("vex_unjail", json!([from])).await?;
        Ok(result.as_str().unwrap_or_default().to_string())
    }

    /// Set on-chain validator profile metadata.
    pub async fn set_validator_metadata(
        &self,
        from: &str,
        name: &str,
        description: &str,
        website: &str,
        avatar_url: &str,
    ) -> Result<String> {
        let result = self.rpc_call(
            "vex_setValidatorMetadata",
            json!([from, name, description, website, avatar_url]),
        ).await?;
        Ok(result.as_str().unwrap_or_default().to_string())
    }

    // --- Read Operations ---

    /// Get delegations for an address (as delegator).
    pub async fn get_delegations(&self, address: &str) -> Result<Value> {
        self.rpc_call("vex_getDelegations", json!([address])).await
    }

    /// Get validator info by address.
    pub async fn get_validator(&self, address: &str) -> Result<Value> {
        self.rpc_call("vex_getValidator", json!([address])).await
    }

    /// List active validators.
    pub async fn list_validators(&self, limit: u32) -> Result<Value> {
        self.rpc_call("vex_listValidators", json!([limit])).await
    }

    /// Get global staking info (total staked, validator count, APY).
    pub async fn staking_info(&self) -> Result<Value> {
        self.rpc_call("vex_stakingInfo", json!([])).await
    }

    /// Get VXS balance for an address.
    pub async fn get_balance(&self, address: &str) -> Result<Value> {
        self.rpc_call("vex_getBalance", json!([address, "VXS"])).await
    }

    /// Generate a new keypair on the server (for testing only).
    pub async fn generate_keypair(&self) -> Result<Value> {
        self.rpc_call("vex_generateKeypair", json!([])).await
    }

    /// Get current block height.
    pub async fn block_number(&self) -> Result<u64> {
        let result = self.rpc_call("eth_blockNumber", json!([])).await?;
        let hex_str = result.as_str().unwrap_or("0x0");
        let height = u64::from_str_radix(hex_str.trim_start_matches("0x"), 16).unwrap_or(0);
        Ok(height)
    }

    /// Check node health (returns true if RPC is reachable).
    pub async fn is_healthy(&self) -> bool {
        self.block_number().await.is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = ValidatorClient::new("http://localhost:9933");
        assert_eq!(client.rpc_url, "http://localhost:9933");
    }
}
