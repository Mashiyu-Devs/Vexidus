//! Natural language intent parser.
//!
//! Parses human-readable strings into Goal + Constraints.
//! Supports patterns like:
//! - "swap 100 VXS for USDC"
//! - "stake 1000 VXS"
//! - "stake 500 VXS with validator Vx1abc..."
//!
//! For LLM-based parsing (Grok, Claude), see the developer guide.

use regex::Regex;
use vexidus_types::intent::{Goal, Constraints};
use vexidus_types::primitives::{Address, Amount};
use crate::intent::IntentError;

/// Result of parsing a natural language intent.
pub struct ParsedIntent {
    pub goal: Goal,
    pub constraints: Constraints,
}

/// Known token symbols → mint addresses.
/// VXS is Address::ZERO (native token). Others are bridged token Blake3 hashes.
fn resolve_token(symbol: &str) -> Option<Address> {
    match symbol.to_uppercase().as_str() {
        "VXS" | "VEXIDUS" => Some(Address::ZERO),
        // Bridged tokens use deterministic Blake3 hash of "{chain}_{contract}"
        // These are placeholder entries — real addresses computed at bridge time
        "USDC" => Some(blake3_mint("ethereum_USDC")),
        "USDT" => Some(blake3_mint("ethereum_USDT")),
        "SOL" => Some(blake3_mint("solana_SOL")),
        "ETH" | "WETH" => Some(blake3_mint("ethereum_ETH")),
        "BTC" | "WBTC" => Some(blake3_mint("ethereum_WBTC")),
        // Vexidus testnet tokens
        "VXUSD" => Some(blake3_mint("vexidus_VXUSD")),
        "VXBTC" => Some(blake3_mint("vexidus_VXBTC")),
        "VXETH" => Some(blake3_mint("vexidus_VXETH")),
        "VXAI" => Some(blake3_mint("vexidus_VXAI")),
        "VMEME" => Some(blake3_mint("vexidus_VMEME")),
        _ => None,
    }
}

fn blake3_mint(unique_id: &str) -> Address {
    let hash = blake3::hash(unique_id.as_bytes());
    Address(*hash.as_bytes())
}

/// Parse a natural language intent string into a Goal + Constraints.
///
/// Supported patterns:
/// - `swap <amount> <TOKEN_A> for <TOKEN_B>` (optional: `with <N>% slippage`)
/// - `stake <amount> <TOKEN>` (optional: `with validator <address>`)
///
/// Returns `Err` if the string doesn't match any known pattern.
pub fn parse_intent(text: &str) -> Result<ParsedIntent, IntentError> {
    let text = text.trim().to_lowercase();

    // Try swap pattern: "swap 100 VXS for USDC" or "swap 100 vxs for usdc with 2% slippage"
    if let Some(parsed) = try_parse_swap(&text) {
        return Ok(parsed);
    }

    // Try liquidity pattern: "add 100 VXS and 500 USDC liquidity"
    if let Some(parsed) = try_parse_liquidity(&text) {
        return Ok(parsed);
    }

    // Try stake pattern: "stake 1000 VXS" or "stake 1000 VXS with validator Vx1..."
    if let Some(parsed) = try_parse_stake(&text) {
        return Ok(parsed);
    }

    // Fallback: wrap as Custom goal for future LLM processing
    Ok(ParsedIntent {
        goal: Goal::Custom(text),
        constraints: Constraints::default(),
    })
}

fn try_parse_swap(text: &str) -> Option<ParsedIntent> {
    let re = Regex::new(
        r"swap\s+(\d+\.?\d*)\s+(\w+)\s+for\s+(\w+)(?:\s+with\s+(\d+)%?\s*slippage)?"
    ).ok()?;

    let caps = re.captures(text)?;
    let amount_str = caps.get(1)?.as_str();
    let from_symbol = caps.get(2)?.as_str();
    let to_symbol = caps.get(3)?.as_str();
    let slippage = caps.get(4).and_then(|m| m.as_str().parse::<u8>().ok());

    let amount: f64 = amount_str.parse().ok()?;
    let from_token = resolve_token(from_symbol)?;
    let to_token = resolve_token(to_symbol)?;

    // Convert to raw amount (9 decimals for VXS)
    let raw_amount = (amount * 1_000_000_000.0) as u128;

    let mut constraints = Constraints::default();
    if let Some(s) = slippage {
        constraints.max_slippage = Some(s);
    }

    Some(ParsedIntent {
        goal: Goal::Swap {
            from_token,
            to_token,
            amount: Amount(raw_amount),
        },
        constraints,
    })
}

fn try_parse_liquidity(text: &str) -> Option<ParsedIntent> {
    // "add 100 VXS and 500 USDC liquidity" or "provide 100 vxs and 500 usdc liquidity"
    let re = Regex::new(
        r"(?:add|provide)\s+(\d+\.?\d*)\s+(\w+)\s+and\s+(\d+\.?\d*)\s+(\w+)\s+liquidity"
    ).ok()?;

    let caps = re.captures(text)?;
    let amount_a_str = caps.get(1)?.as_str();
    let symbol_a = caps.get(2)?.as_str();
    let amount_b_str = caps.get(3)?.as_str();
    let symbol_b = caps.get(4)?.as_str();

    let amount_a: f64 = amount_a_str.parse().ok()?;
    let amount_b: f64 = amount_b_str.parse().ok()?;
    let token_a = resolve_token(symbol_a)?;
    let token_b = resolve_token(symbol_b)?;

    let raw_a = (amount_a * 1_000_000_000.0) as u128;
    let raw_b = (amount_b * 1_000_000_000.0) as u128;

    Some(ParsedIntent {
        goal: Goal::ProvideLiquidity {
            token_a,
            token_b,
            amount_a: Amount(raw_a),
            amount_b: Amount(raw_b),
        },
        constraints: Constraints::default(),
    })
}

fn try_parse_stake(text: &str) -> Option<ParsedIntent> {
    let re = Regex::new(
        r"stake\s+(\d+\.?\d*)\s+(\w+)(?:\s+with\s+validator\s+(\S+))?"
    ).ok()?;

    let caps = re.captures(text)?;
    let amount_str = caps.get(1)?.as_str();
    let token_symbol = caps.get(2)?.as_str();
    let validator_str = caps.get(3).map(|m| m.as_str());

    let amount: f64 = amount_str.parse().ok()?;
    let _token = resolve_token(token_symbol)?; // Validate token exists

    let raw_amount = (amount * 1_000_000_000.0) as u128;

    let validator = validator_str.and_then(|v| {
        crate::address_utils::parse_address(v).ok()
    });

    Some(ParsedIntent {
        goal: Goal::Stake {
            token: Address::ZERO, // Staking is always VXS
            amount: Amount(raw_amount),
            validator,
        },
        constraints: Constraints::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_swap() {
        let result = parse_intent("swap 100 VXS for USDC").unwrap();
        match result.goal {
            Goal::Swap { from_token, to_token, amount } => {
                assert_eq!(from_token, Address::ZERO); // VXS
                assert_ne!(to_token, Address::ZERO); // USDC
                assert_eq!(amount, Amount(100_000_000_000)); // 100 * 10^9
            }
            _ => panic!("Expected Swap"),
        }
    }

    #[test]
    fn test_parse_swap_with_slippage() {
        let result = parse_intent("swap 50 ETH for VXS with 3% slippage").unwrap();
        match result.goal {
            Goal::Swap { amount, .. } => {
                assert_eq!(amount, Amount(50_000_000_000));
            }
            _ => panic!("Expected Swap"),
        }
        assert_eq!(result.constraints.max_slippage, Some(3));
    }

    #[test]
    fn test_parse_stake() {
        let result = parse_intent("stake 1000 VXS").unwrap();
        match result.goal {
            Goal::Stake { amount, validator, .. } => {
                assert_eq!(amount, Amount(1_000_000_000_000));
                assert!(validator.is_none());
            }
            _ => panic!("Expected Stake"),
        }
    }

    #[test]
    fn test_parse_unknown_falls_to_custom() {
        let result = parse_intent("do something complex").unwrap();
        match result.goal {
            Goal::Custom(text) => assert_eq!(text, "do something complex"),
            _ => panic!("Expected Custom"),
        }
    }

    #[test]
    fn test_parse_case_insensitive() {
        let result = parse_intent("SWAP 10 vxs FOR usdc").unwrap();
        match result.goal {
            Goal::Swap { .. } => {} // Success
            _ => panic!("Expected Swap"),
        }
    }

    #[test]
    fn test_parse_add_liquidity() {
        let result = parse_intent("add 100 VXS and 500 USDC liquidity").unwrap();
        match result.goal {
            Goal::ProvideLiquidity { token_a, token_b, amount_a, amount_b } => {
                assert_eq!(token_a, Address::ZERO); // VXS
                assert_ne!(token_b, Address::ZERO); // USDC
                assert_eq!(amount_a, Amount(100_000_000_000));
                assert_eq!(amount_b, Amount(500_000_000_000));
            }
            _ => panic!("Expected ProvideLiquidity"),
        }
    }

    #[test]
    fn test_parse_provide_liquidity() {
        let result = parse_intent("provide 50 ETH and 1000 VXS liquidity").unwrap();
        match result.goal {
            Goal::ProvideLiquidity { amount_a, amount_b, .. } => {
                assert_eq!(amount_a, Amount(50_000_000_000));
                assert_eq!(amount_b, Amount(1_000_000_000_000));
            }
            _ => panic!("Expected ProvideLiquidity"),
        }
    }

    #[test]
    fn test_unknown_token_falls_to_custom() {
        let result = parse_intent("swap 10 DOGWIFHAT for VXS").unwrap();
        match result.goal {
            Goal::Custom(_) => {} // Unknown token → Custom fallback
            _ => panic!("Expected Custom for unknown token"),
        }
    }
}
