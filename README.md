# Vexidus

**The Ultimate L1 Blockchain** — Quantum-hardened, intent-driven, Solana-competitive fees.

## Current Status

- **Network:** 5 validators across 4 continents, 500K+ blocks produced
- **Transactions:** 1.5M+ processed, zero crashes, zero manual restarts
- **Performance:** 1,000 TPS sustained (proven), sub-second block execution
- **Upgrades:** 22 on-chain upgrades via VexVisor with zero manual intervention
- **Codebase:** 15 Rust crates, 241 passing tests, 95+ RPC endpoints

## What's Here

| Folder | Description |
|--------|-------------|
| [Vexidus Validator](Vexidus%20Validator/) | Run a validator node — setup guide, genesis files, SDK |
| [Vexidus Smart Contracts](Vexidus%20Smart%20Contracts/) | VSC token standards (VSC-7, VSC-21, VSC-55, VSC-8, VSC-88) |
| [IntentVM](IntentVM/) | IntentVM overview (patent-pending) |
| [Vexidus Smart Wallets](Vexidus%20Smart%20Wallets/) | VSA v2 wallet SDK |
| [Builder Toolkit](Builder%20Toolkit/) | Tools for building on Vexidus |

## Key Features

- **IntentVM** (U.S. Patent App. 63/914,009) — 1 intent, 1 signature, N operations. Describe what you want in natural language.
- **HyperSync Consensus + Vexcel** — Ed25519 block signing, weighted leader rotation, adaptive attestation DAG for global latency fairness, on-chain reputation scoring.
- **Dragonfly Stream** — Mempoolless direct-to-leader transaction delivery with PQ seals. Zero MEV by design.
- **VexBridge** (U.S. Patent App. 63/987,929) — Vaultless canonical token bridge. No wrapped tokens, no vaults to hack.
- **Atomence** (U.S. Patent App. 63/998,160) — Cross-chain settlement layer. Market-maker matched exits back to origin chains.
- **Quantum-Hardened** — Ed25519 + Dilithium3 dual-signature architecture. PQ seals active on transaction forwarding.
- **Solana-Competitive Fees** — ~$0.0002 per transfer. 80% to validators, 20% Foundation Treasury (no burn).
- **No Slashing** — Validators are jailed for downtime, never slashed. Your VXS is never at risk.
- **Risk Management** — Account/token freeze, state correction, circuit breaker, emergency mint — all via governance.
- **State Preservation** — Full state export/import/verify for state-preserving genesis restarts.

## Network

| Parameter | Value |
|-----------|-------|
| Chain ID | 1618032 |
| Block time | Adaptive (500ms-12s, load-responsive) |
| Min validator stake | 1,000 VXS |
| Gas price | ~$0.0002/transfer |
| Total supply | 1.618B VXS (fixed, unburnable) |
| Block rewards | 15% Foundation, 85% Validator (density-weighted) |
| Transaction fees | 20% Foundation, 80% Proposer |

## Links

- [Documentation](https://docs.vexidus.io)
- [Block Explorer](https://vexscan.io)
- [Testnet Faucet](https://vexswap.xyz)
- [Developer Studio](https://vexforge.xyz)
- [Native Wallet](https://wallet.vexspark.com)
- [DEX](https://vexidex.com)
- [Atomence](https://atomence.com)

## Development

| Entity | Role |
|--------|------|
| **Vexidus Labs Ltd** (United Kingdom) | Engineering, infrastructure, operations |
| **Vexidus Corporation** (Delaware C-Corp) | IP, patents, investor capital, strategy |
| **Vexidus Stiftung** (Switzerland, planned) | Foundation governance, community treasury |

## Patent Notice

IntentVM (63/914,009), VexBridge (63/987,929), and Atomence (63/998,160) are protected by pending U.S. patent applications. See [vexidus.io](https://vexidus.io) for details.

## License

Apache 2.0 — See [LICENSE](LICENSE) for details.
