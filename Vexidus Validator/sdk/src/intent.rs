//! IntentVM SDK — fluent builder for constructing intent-based transactions.
//!
//! # Example
//! ```rust,no_run
//! use vexidus_sdk::IntentBuilder;
//! use vexidus_types::primitives::{Address, Amount, Timestamp};
//!
//! let (goal, constraints) = IntentBuilder::new()
//!     .swap(Address::ZERO, Address([1u8; 32]), Amount::from_vxd(100))
//!     .with_slippage(1)
//!     .with_deadline(Timestamp(1700000000))
//!     .build()
//!     .unwrap();
//! ```

use vexidus_types::intent::{Goal, Constraints, RoutePreference};
use vexidus_types::primitives::{Address, Amount, Timestamp};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IntentError {
    #[error("No goal specified — call swap(), stake(), or transfer() first")]
    NoGoal,
    #[error("No sender specified — call from_account() first")]
    NoSender,
    #[error("Invalid slippage: {0}% (max 100)")]
    InvalidSlippage(u8),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Fluent builder for constructing intents.
pub struct IntentBuilder {
    goal: Option<Goal>,
    constraints: Constraints,
    from: Option<Address>,
}

impl IntentBuilder {
    pub fn new() -> Self {
        Self {
            goal: None,
            constraints: Constraints::default(),
            from: None,
        }
    }

    /// Set the sender account.
    pub fn from_account(mut self, address: Address) -> Self {
        self.from = Some(address);
        self
    }

    /// Swap tokens.
    pub fn swap(mut self, from_token: Address, to_token: Address, amount: Amount) -> Self {
        self.goal = Some(Goal::Swap { from_token, to_token, amount });
        self
    }

    /// Stake tokens (optionally to a specific validator).
    pub fn stake(mut self, amount: Amount, validator: Option<Address>) -> Self {
        self.goal = Some(Goal::Stake {
            token: Address::ZERO, // VXS
            amount,
            validator,
        });
        self
    }

    /// Provide liquidity to a pool.
    pub fn provide_liquidity(
        mut self,
        token_a: Address,
        token_b: Address,
        amount_a: Amount,
        amount_b: Amount,
    ) -> Self {
        self.goal = Some(Goal::ProvideLiquidity { token_a, token_b, amount_a, amount_b });
        self
    }

    /// Set a custom natural language goal (for NL parser or future LLM integration).
    pub fn custom(mut self, description: String) -> Self {
        self.goal = Some(Goal::Custom(description));
        self
    }

    /// Compose multiple goals atomically.
    pub fn composite(mut self, goals: Vec<Goal>) -> Self {
        self.goal = Some(Goal::Composite(goals));
        self
    }

    /// Set maximum slippage tolerance (0-100%).
    pub fn with_slippage(mut self, pct: u8) -> Self {
        self.constraints.max_slippage = Some(pct);
        self
    }

    /// Set execution deadline.
    pub fn with_deadline(mut self, timestamp: Timestamp) -> Self {
        self.constraints.deadline = Some(timestamp);
        self
    }

    /// Set minimum output amount.
    pub fn with_min_output(mut self, amount: Amount) -> Self {
        self.constraints.min_output = Some(amount);
        self
    }

    /// Prefer a specific DEX for routing.
    pub fn prefer_dex(mut self, dex: Address) -> Self {
        self.constraints.preferred_route = RoutePreference::PreferDex(dex);
        self
    }

    /// Enable gas sponsorship.
    pub fn sponsored(mut self) -> Self {
        self.constraints.sponsored_gas = true;
        self
    }

    /// Build the intent, returning (Goal, Constraints).
    pub fn build(self) -> Result<(Goal, Constraints), IntentError> {
        let goal = self.goal.ok_or(IntentError::NoGoal)?;
        if let Some(s) = self.constraints.max_slippage {
            if s > 100 {
                return Err(IntentError::InvalidSlippage(s));
            }
        }
        Ok((goal, self.constraints))
    }

    /// Build and serialize to JSON (for RPC submission).
    pub fn to_json(&self) -> Result<String, IntentError> {
        let goal = self.goal.clone().ok_or(IntentError::NoGoal)?;
        let json = serde_json::json!({
            "goal": goal,
            "constraints": self.constraints,
        });
        serde_json::to_string(&json)
            .map_err(|e| IntentError::SerializationError(e.to_string()))
    }

    /// Get the sender address (if set).
    pub fn sender(&self) -> Option<&Address> {
        self.from.as_ref()
    }
}

impl Default for IntentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_swap_intent() {
        let (goal, constraints) = IntentBuilder::new()
            .swap(Address::ZERO, Address([1u8; 32]), Amount::from_vxd(100))
            .with_slippage(2)
            .build()
            .unwrap();

        match goal {
            Goal::Swap { amount, .. } => assert_eq!(amount, Amount::from_vxd(100)),
            _ => panic!("Expected Swap goal"),
        }
        assert_eq!(constraints.max_slippage, Some(2));
    }

    #[test]
    fn test_build_stake_intent() {
        let (goal, _) = IntentBuilder::new()
            .stake(Amount::from_vxd(1000), None)
            .build()
            .unwrap();

        match goal {
            Goal::Stake { amount, validator, .. } => {
                assert_eq!(amount, Amount::from_vxd(1000));
                assert!(validator.is_none());
            }
            _ => panic!("Expected Stake goal"),
        }
    }

    #[test]
    fn test_no_goal_errors() {
        let result = IntentBuilder::new().build();
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_slippage() {
        let result = IntentBuilder::new()
            .swap(Address::ZERO, Address([1u8; 32]), Amount::from_vxd(10))
            .with_slippage(101)
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_to_json() {
        let builder = IntentBuilder::new()
            .swap(Address::ZERO, Address([1u8; 32]), Amount(50))
            .with_slippage(1);
        let json = builder.to_json().unwrap();
        assert!(json.contains("Swap"));
        assert!(json.contains("max_slippage"));
    }

    #[test]
    fn test_composite_goal() {
        let goals = vec![
            Goal::Swap {
                from_token: Address::ZERO,
                to_token: Address([1u8; 32]),
                amount: Amount::from_vxd(100),
            },
            Goal::Stake {
                token: Address::ZERO,
                amount: Amount::from_vxd(50),
                validator: None,
            },
        ];
        let (goal, _) = IntentBuilder::new()
            .composite(goals)
            .build()
            .unwrap();

        match goal {
            Goal::Composite(g) => assert_eq!(g.len(), 2),
            _ => panic!("Expected Composite"),
        }
    }
}
