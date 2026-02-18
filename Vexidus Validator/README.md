# Vexidus Validator

Run a validator node on the Vexidus network and earn VXS rewards for securing the chain.

## Quick Start

1. Read the full [Setup Guide](SETUP_GUIDE.md)
2. Get testnet VXS from the [faucet](https://vexswap.xyz)
3. Stake a minimum of **1,000 VXS** to join the active validator set
4. Set your commission rate and validator profile

## What's in This Folder

| Path | Description |
|------|-------------|
| `SETUP_GUIDE.md` | Complete validator setup guide (requirements, install, config, staking, troubleshooting) |
| `genesis/` | Genesis files required by all validators (must be byte-identical across nodes) |
| `sdk/` | Validator SDK (Rust) — keypair management, RPC client, config generation |

## Rewards

Validators earn from two sources:
- **Block rewards** — ~4.94 VXS per block (Year 1-2 rate, 0.8% annual inflation)
- **Transaction fees** — 100% of fees go to the block proposer

Testnet validators who maintain high performance earn **mainnet VXS airdrops** at launch, proportional to on-chain reputation score.

## Network Info

| Parameter | Value |
|-----------|-------|
| Minimum stake | 1,000 VXS |
| Block time | 12 seconds |
| Unbonding period | 21 days |
| P2P transport | QUIC (UDP) |
| Consensus | HyperSync (Ed25519 voting) |
| Gas price | ~$0.0002 per transfer |

### Bootstrap Node

```
/ip4/51.255.80.34/udp/9945/quic-v1/p2p/12D3KooWHhBs5eZFBePWtZhgQfcB7Ds55uRjQqmS6ara7aF8hV1U
```

## SDK Usage (Rust)

The `sdk/` folder contains the Validator SDK source for programmatic node management:

```rust
use vexidus_sdk::{ValidatorKeypair, ValidatorClient, ValidatorConfig};

// Generate a validator key
let keypair = ValidatorKeypair::generate();
keypair.save("/opt/vexidus/validator.key")?;

// Connect to your node
let client = ValidatorClient::new("http://localhost:9933");

// Stake to register
client.stake("YOUR_ADDRESS", "1000", &keypair.public_key_hex()).await?;

// Set commission (500 = 5%)
client.set_commission("YOUR_ADDRESS", 500).await?;

// Set your profile
client.set_validator_metadata(
    "YOUR_ADDRESS",
    "My Validator",
    "Reliable validator node",
    "https://mysite.com",
    "https://mysite.com/avatar.png",
).await?;

// Check status
let info = client.get_validator("YOUR_ADDRESS").await?;
```

## Links

- [Documentation](https://docs.vexidus.io)
- [Block Explorer](https://vexscan.io)
- [Testnet Faucet](https://vexswap.xyz)
- [Developer Studio](https://vexforge.xyz)
