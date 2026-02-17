# Vexidus Validator Setup Guide

Run a validator node on the Vexidus network and earn VXS rewards for securing the chain.

---

## Table of Contents

1. [Overview](#overview)
2. [System Requirements](#system-requirements)
3. [Install Vexidus](#install-vexidus)
4. [Generate Your Validator Key](#generate-your-validator-key)
5. [Configure Your Node](#configure-your-node)
6. [Start Your Node](#start-your-node)
7. [Stake and Register as a Validator](#stake-and-register-as-a-validator)
8. [Connect to the Network](#connect-to-the-network)
9. [Monitoring](#monitoring)
10. [Security Best Practices](#security-best-practices)
11. [Maintenance](#maintenance)
12. [Troubleshooting](#troubleshooting)
13. [Testnet Rewards](#testnet-rewards)

---

## Overview

Vexidus validators produce blocks every 12 seconds and earn rewards from two sources:

- **Block rewards** — ~17.96 VXS per block (year 1-2 rate), auto-adjusting over a 10-year emission schedule
- **Transaction fees** — 100% of fees go to the block proposer (no burn)

| Parameter | Value |
|-----------|-------|
| Minimum stake | 1,000 VXS |
| Unbonding period | 21 days |
| Max active validators | 100 |
| Block time | 12 seconds |
| Epoch duration | 300 seconds |
| Slashing | None (jail + throttle only — your VXS is never at risk) |
| Jail threshold | 5 missed blocks → jailed (1-hour cooldown, then unjail) |

Validators use Ed25519 keys for block signing and vote participation in the HyperSync consensus protocol. P2P transport uses QUIC (TLS 1.3 + multiplexing over UDP).

---

## System Requirements

### Minimum

| Resource | Spec |
|----------|------|
| CPU | 4 cores / 4 threads |
| RAM | 8 GB |
| Storage | 100 GB SSD |
| Network | 100 Mbps, static IP or NAT with port forwarding |
| OS | Ubuntu 22.04+ / Debian 12+ (x86_64) |

### Recommended

| Resource | Spec |
|----------|------|
| CPU | 8+ cores / 16 threads |
| RAM | 16 GB+ |
| Storage | 500 GB NVMe SSD |
| Network | 1 Gbps, static IP, low latency |
| OS | Ubuntu 24.04 LTS (x86_64) |

### Required Ports

| Port | Protocol | Purpose |
|------|----------|---------|
| 9944 | UDP | P2P (QUIC) — must be open to all peers |
| 9933 | TCP | RPC — restrict to localhost or trusted IPs |

### Required Tools

These are installed automatically by the [quick install script](#quick-install-one-command), or you can install them manually:

| Tool | Purpose |
|------|---------|
| `build-essential` | C compiler and build tools |
| `pkg-config` | Library path resolution |
| `libssl-dev` | TLS for P2P and RPC |
| `libclang-dev` | RocksDB compilation |
| `cmake` | Build system for native dependencies |
| `git` | Clone the repository |
| `curl` | Downloads and RPC calls |
| `jq` | JSON formatting for RPC responses |
| `rustc` 1.75+ | Rust compiler (installed via rustup) |

---

## Install Vexidus

### Install

#### 1. Install system dependencies

```bash
sudo apt update && sudo apt install -y \
  build-essential \
  pkg-config \
  libssl-dev \
  libclang-dev \
  cmake \
  git \
  curl \
  jq
```

#### 2. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
```

Verify: `rustc --version` (requires 1.75+)

#### 3. Clone and build

```bash
git clone https://github.com/Mashiyu-Devs/Vexidus.git
cd Vexidus
cargo build --release
```

Build takes 5-10 minutes depending on your hardware. The binary is at `target/release/vexidus-node`.

#### 4. Install the binary

```bash
sudo cp target/release/vexidus-node /usr/local/bin/
vexidus-node --help
```

#### 5. Create system user and directories

```bash
sudo useradd --system --no-create-home --shell /usr/sbin/nologin vexidus
sudo mkdir -p /opt/vexidus/data
sudo chown -R vexidus:vexidus /opt/vexidus
```

#### 6. Copy genesis files

Every validator **must** have identical genesis files to produce the same state root:

```bash
sudo cp genesis_vxs.json /opt/vexidus/
sudo cp genesis_tokens.json /opt/vexidus/
sudo chown vexidus:vexidus /opt/vexidus/genesis_*.json
```

---

## Generate Your Validator Key

Your validator key is an Ed25519 keypair used to sign blocks and votes. **If you lose your secret key, your staked VXS is unrecoverable.**

### Generate the key

```bash
openssl rand -hex 32 > validator.key
chmod 600 validator.key
```

This creates a 64-character hex file containing your 32-byte Ed25519 secret key.

### Move to secure location

```bash
sudo mv validator.key /opt/vexidus/validator.key
sudo chown vexidus:vexidus /opt/vexidus/validator.key
sudo chmod 600 /opt/vexidus/validator.key
```

### View your public key

Your node logs the derived public key on startup when `--validator-key` is provided. You can preview it with a quick test run:

```bash
vexidus-node --validator-key /opt/vexidus/validator.key --data-dir /tmp/vex-keytest 2>&1 | head -20
rm -rf /tmp/vex-keytest
```

Look for: `Validator public key: 0x...` — this is your validator address.

### Back up your key

Copy your `validator.key` to a secure offline location:
- Encrypted USB drive
- Password manager with file attachments
- Hardware security module (HSM) for production deployments

**Never** share your secret key, paste it in chat, or commit it to version control.

---

## Configure Your Node

### Create a validator config

Create `/opt/vexidus/validator.toml`:

```toml
# Vexidus Validator Configuration

# Path to your Ed25519 secret key (64 hex chars)
keypair_path = "/opt/vexidus/validator.key"

# RPC endpoint (keep on localhost for security)
rpc_url = "http://127.0.0.1:9933"

# Network ports
p2p_port = 9944
rpc_port = 9933

# Chain data directory
data_dir = "/opt/vexidus/data"

# Your server's public IP for peer discovery
# Find yours with: curl -4 ifconfig.me
external_addr = "/ip4/YOUR_PUBLIC_IP/udp/9944/quic-v1"

# Bootstrap peers (comma-separated multiaddrs)
bootnodes = "/ip4/51.255.80.34/udp/9945/quic-v1/p2p/12D3KooWHhBs5eZFBePWtZhgQfcB7Ds55uRjQqmS6ara7aF8hV1U"

# Enable detailed logging (useful during initial setup)
verbose = false
```

Replace `YOUR_PUBLIC_IP` with your server's actual public IP:

```bash
curl -4 ifconfig.me
```

### Set permissions

```bash
sudo chown vexidus:vexidus /opt/vexidus/validator.toml
```

### Create the systemd service

Create `/etc/systemd/system/vexidus-validator.service`:

```ini
[Unit]
Description=Vexidus Validator Node
After=network.target
Wants=network-online.target

[Service]
Type=simple
User=vexidus
Group=vexidus
WorkingDirectory=/opt/vexidus
ExecStart=/usr/local/bin/vexidus-node \
  --data-dir /opt/vexidus/data \
  --rpc-port 9933 \
  --p2p-port 9944 \
  --validator-key /opt/vexidus/validator.key \
  --external-addr /ip4/YOUR_PUBLIC_IP/udp/9944/quic-v1 \
  --bootnodes /ip4/51.255.80.34/udp/9945/quic-v1/p2p/12D3KooWHhBs5eZFBePWtZhgQfcB7Ds55uRjQqmS6ara7aF8hV1U \
  --gas-price 10
Restart=always
RestartSec=5
LimitNOFILE=65536

# Hardening
ProtectSystem=full
ProtectHome=read-only
NoNewPrivileges=true

[Install]
WantedBy=multi-user.target
```

---

## Start Your Node

```bash
sudo systemctl daemon-reload
sudo systemctl enable vexidus-validator
sudo systemctl start vexidus-validator
```

### Verify it's running

```bash
sudo systemctl status vexidus-validator
```

### Watch the logs

```bash
sudo journalctl -u vexidus-validator -f
```

You should see:

```
Vexidus Blockchain Node v0.1.0
Loading validator key from "/opt/vexidus/validator.key"
Validator public key: 0xabc123...
P2P Network started successfully
Listening on /ip4/0.0.0.0/udp/9944/quic-v1
RPC server listening on 0.0.0.0:9933
Block #12345 produced ...
```

### Wait for sync

Before staking, ensure your node is fully synced. Check your current block height:

```bash
curl -s http://localhost:9933 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' | jq .result
```

---

## Stake and Register as a Validator

Once synced, stake VXS to join the active validator set.

### Get Testnet VXS

Before staking, you need VXS tokens. Visit the testnet faucet at [vexswap.xyz](https://vexswap.xyz):

1. Generate a wallet address (your node logs the derived address on startup)
2. Go to https://vexswap.xyz and enter your address to receive testnet VXS
3. Rate limit: 100 VXS per address per hour — request multiple times to accumulate the 1,000 VXS minimum stake

### Check your balance

```bash
curl -s http://localhost:9933 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"vex_getBalance","params":["YOUR_ADDRESS","VXS"],"id":1}' | jq .
```

### Stake VXS (minimum 1,000)

```bash
curl -s http://localhost:9933 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "vex_stake",
    "params": ["YOUR_STAKER_ADDRESS", "1000", "YOUR_VALIDATOR_PUBKEY_HEX"],
    "id": 1
  }' | jq .
```

| Parameter | Description |
|-----------|-------------|
| `YOUR_STAKER_ADDRESS` | The account holding your VXS (hex-encoded 32 bytes) |
| `1000` | Amount of VXS to stake (minimum 1,000) |
| `YOUR_VALIDATOR_PUBKEY_HEX` | 64-char hex public key derived from your `validator.key` |

Or using the SDK:

```rust
use vexidus_sdk::ValidatorClient;

let client = ValidatorClient::new("http://localhost:9933");
client.stake("YOUR_ADDRESS", "1000", "YOUR_PUBKEY_HEX").await?;
```

### Verify your registration

```bash
# Check your validator entry
curl -s http://localhost:9933 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"vex_getValidator","params":["YOUR_ADDRESS"],"id":1}' | jq .

# List all active validators
curl -s http://localhost:9933 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"vex_listValidators","params":[100],"id":1}' | jq .

# Network staking summary
curl -s http://localhost:9933 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"vex_stakingInfo","params":[],"id":1}' | jq .
```

### Set your validator profile

Set on-chain metadata so delegators can identify your validator:

```bash
curl -s http://localhost:9933 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "vex_setValidatorMetadata",
    "params": ["YOUR_ADDRESS", "My Validator", "Reliable validator node", "https://mysite.com", "https://mysite.com/avatar.png"],
    "id": 1
  }' | jq .
```

### Set commission rate

Commission is the percentage of delegator rewards you keep (in basis points, max 5000 = 50%):

```bash
curl -s http://localhost:9933 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "vex_setCommission",
    "params": ["YOUR_ADDRESS", 500],
    "id": 1
  }' | jq .
```

`500` = 5% commission. Default is 0%.

### Gas and Transaction Fees

Vexidus uses a gas-based fee model similar to Ethereum, but with Solana-competitive pricing:

| Parameter | Value |
|-----------|-------|
| Base gas price | 10 nanoVXS per gas unit (configurable via `--gas-price`) |
| Transfer gas | ~21,000 gas units |
| Cost per transfer | ~0.00021 VXS (~$0.0002 at $1/VXS) |
| Fee recipient | 100% to block proposer (no burn) |

Set `--gas-price 0` in your systemd service for free testnet transactions.

---

## Connect to the Network

### Bootstrap Nodes

Your node needs at least one bootstrap peer to discover the network. Current bootstrap:

```
/ip4/51.255.80.34/udp/9945/quic-v1/p2p/12D3KooWHhBs5eZFBePWtZhgQfcB7Ds55uRjQqmS6ara7aF8hV1U
```

Updated bootstrap lists are published at [github.com/Mashiyu-Devs/Vexidus](https://github.com/Mashiyu-Devs/Vexidus).

### NAT and Firewalls

If your server is behind NAT:

1. **Forward port 9944 (UDP)** from your router to your server
2. **Set `--external-addr`** so peers can find you:
   ```
   --external-addr /ip4/YOUR_PUBLIC_IP/udp/9944/quic-v1
   ```

### Verify peer connectivity

```bash
sudo journalctl -u vexidus-validator --no-pager | grep -i "peer\|connect\|dial"
```

### Firewall setup (ufw)

```bash
# Allow P2P from anywhere (QUIC uses UDP)
sudo ufw allow 9944/udp

# Block public RPC access (use localhost only)
sudo ufw deny 9933/tcp

# Enable firewall
sudo ufw enable
```

If you need remote RPC access from specific IPs:

```bash
sudo ufw allow from YOUR_TRUSTED_IP to any port 9933 proto tcp
```

---

## Monitoring

### Quick health check

```bash
curl -s http://localhost:9933 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' | jq .result
```

### Logs

```bash
# Last 50 lines
sudo journalctl -u vexidus-validator -n 50 --no-pager

# Live follow
sudo journalctl -u vexidus-validator -f
```

### Monitoring script

Save as `/opt/vexidus/monitor.sh`:

```bash
#!/bin/bash
# Vexidus Validator Health Monitor

RPC="http://localhost:9933"

rpc_call() {
  curl -sf "$RPC" \
    -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"method\":\"$1\",\"params\":$2,\"id\":1}" 2>/dev/null
}

block=$(rpc_call "eth_blockNumber" "[]" | jq -r '.result // "unreachable"')
validators=$(rpc_call "vex_listValidators" "[100]" | jq -r '.result | length // 0')
staking=$(rpc_call "vex_stakingInfo" "[]" | jq -r '.result.total_staked // "0"')
status=$(systemctl is-active vexidus-validator)
uptime=$(systemctl show vexidus-validator --property=ActiveEnterTimestamp --value)

echo "=============================="
echo "  Vexidus Validator Status"
echo "=============================="
echo "  Service:       $status"
echo "  Block height:  $block"
echo "  Validators:    $validators"
echo "  Total staked:  $staking VXS"
echo "  Up since:      $uptime"
echo "=============================="
```

```bash
sudo chmod +x /opt/vexidus/monitor.sh
sudo chown vexidus:vexidus /opt/vexidus/monitor.sh
```

---

## Security Best Practices

### Key Management

- Store `validator.key` with permissions `600`, owned by `vexidus` user only
- Maintain an encrypted offline backup (USB, password manager, or HSM)
- Never share your secret key or commit it to git
- Key rotation will be available via VSA v2 (coming soon)

### Network

- **RPC port (9933):** Never expose to the public internet. Bind to `127.0.0.1` or restrict via firewall.
- **P2P port (9944):** Must be open to all peers for consensus participation.

### System Hardening

- Keep your OS updated: `sudo apt update && sudo apt upgrade -y`
- Enable unattended security updates: `sudo apt install -y unattended-upgrades`
- Use SSH key authentication and disable password login
- Run the node as the dedicated `vexidus` user, never as root
- The systemd service includes `ProtectSystem=full`, `ProtectHome=read-only`, and `NoNewPrivileges=true` by default

---

## Maintenance

### Upgrading the node

```bash
cd /path/to/vexidus-blockchain
git pull origin main
cargo build --release
sudo systemctl stop vexidus-validator
sudo cp target/release/vexidus-node /usr/local/bin/
sudo systemctl start vexidus-validator

# Verify clean startup
sudo journalctl -u vexidus-validator -n 20 --no-pager
```

### Unstaking

To leave the validator set, begin the 21-day unbonding process:

```bash
curl -s http://localhost:9933 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "vex_unstake",
    "params": ["YOUR_ADDRESS", "1000"],
    "id": 1
  }' | jq .
```

After 21 days, submit a `ClaimUnstake` transaction to withdraw your VXS.

### Full resync (wipe chain data)

If your node is corrupted or you need a clean start:

```bash
sudo systemctl stop vexidus-validator
sudo rm -rf /opt/vexidus/data
sudo systemctl start vexidus-validator
```

Your `validator.key` and `node_key` (PeerId) are preserved — only chain state is wiped.

---

## Troubleshooting

### Node won't start

```bash
sudo systemctl status vexidus-validator
sudo journalctl -u vexidus-validator -n 100 --no-pager
```

| Symptom | Fix |
|---------|-----|
| Permission denied on key file | `sudo chown vexidus:vexidus /opt/vexidus/validator.key && sudo chmod 600 /opt/vexidus/validator.key` |
| Port already in use | Check for conflicts: `sudo lsof -i :9944` |
| Missing data directory | `sudo mkdir -p /opt/vexidus/data && sudo chown vexidus:vexidus /opt/vexidus/data` |
| Missing genesis files | Copy `genesis_vxs.json` and `genesis_tokens.json` to `/opt/vexidus/` |

### No peers connecting

- Verify port 9944/udp is open: `nc -zuv YOUR_PUBLIC_IP 9944` from another machine
- Confirm `--external-addr` matches your actual public IP
- Check bootnode is reachable: `nc -zuv 51.255.80.34 9945`
- Inspect dial attempts: `sudo journalctl -u vexidus-validator --no-pager | grep -i "dial\|error\|failed"`

### Node not producing blocks

- Ensure your node is fully synced (block height matches the network)
- Verify you are in the active set: `vex_listValidators`
- Confirm your stake meets the 1,000 VXS minimum: `vex_getValidator`
- Check if your validator is jailed (see below)
- Check for epoch transitions in logs

### Validator is jailed

Validators are jailed after missing 5 consecutive blocks. Jailed validators cannot produce blocks or earn rewards. After a 1-hour cooldown, unjail yourself:

```bash
curl -s http://localhost:9933 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "vex_unjail",
    "params": ["YOUR_ADDRESS"],
    "id": 1
  }' | jq .
```

Check jail status with `vex_getValidator` — look for `is_jailed` and `jail_release_time` fields.

### RPC not responding

- Confirm the service is running: `systemctl is-active vexidus-validator`
- Check port is bound: `ss -tlnp | grep 9933`
- Test locally: `curl http://localhost:9933 -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'`

---

## Key Files Reference

| File | Purpose |
|------|---------|
| `/opt/vexidus/validator.key` | Ed25519 secret key (64 hex chars). **Keep secret.** |
| `/opt/vexidus/validator.toml` | Node configuration |
| `/opt/vexidus/genesis_vxs.json` | VXS allocations (must be identical across all validators) |
| `/opt/vexidus/genesis_tokens.json` | Bridged token registry (must be identical) |
| `/opt/vexidus/data/node_key` | libp2p identity key (auto-generated, stable PeerId) |
| `/opt/vexidus/data/` | RocksDB chain state |
| `/usr/local/bin/vexidus-node` | Node binary |

## CLI Reference

```
vexidus-node [OPTIONS]

Options:
  -d, --data-dir <PATH>              Data directory [default: ./data]
  -v, --verbose                      Enable verbose logging
      --rpc-port <PORT>              RPC listen port [default: 9933]
      --p2p-port <PORT>              P2P listen port, QUIC/UDP [default: 9944]
      --bootnodes <MULTIADDRS>       Comma-separated bootstrap peer multiaddrs
      --validator-key <PATH>         Path to Ed25519 signing key file
      --external-addr <MULTIADDR>    Public address to advertise (NAT traversal)
      --gas-price <N>                Base gas price in nanoVXS/gas [default: 10]
      --block-time <SECS>            Block production interval [default: 12]
      --max-txs-per-block <N>        Max transactions per block [default: 10000]
      --min-validators <N>           Minimum validators required [default: 1]
      --reject-unsigned-bundles      Reject unsigned transaction bundles
      --no-leader-check              Disable leader rotation (solo testnet only)
      --pruning <MODE>               Pruning mode: archive, validator, aggressive [default: archive]
      --snapshot-url <URL>           Download and restore snapshot on first start
      --serve-snapshots              Serve checkpoint snapshots for state sync
      --light                        Run in light mode (headers only, minimal resources)
      --backfill-indexes             Backfill explorer indexes on startup
  -h, --help                         Print help
```

---

## Testnet Rewards

Testnet validators who maintain high performance earn **mainnet VXS airdrops** at launch. Rewards are based on your on-chain reputation score (VSC-REP), which tracks 7 factors:

- Uptime and block production consistency
- Vote participation rate
- Stake amount and duration
- Commission rate fairness
- Transaction processing volume
- Network contribution (peer connectivity)
- Governance participation (future)

Check your validator's reputation score via `vex_getValidator` — the `performance_score` field (0.5 to 1.0) directly affects your leader selection weight and reward share. New validators start at 0.8.

---

## Links

- **Documentation:** [docs.vexidus.io](https://docs.vexidus.io)
- **Repository:** [github.com/Mashiyu-Devs/Vexidus](https://github.com/Mashiyu-Devs/Vexidus)
- **Explorer:** [vexscan.io](https://vexscan.io)
- **Faucet:** [vexswap.xyz](https://vexswap.xyz) (testnet VXS for staking)
- **Developer Studio:** [vexforge.xyz](https://vexforge.xyz)
- **SDK Crate:** `sdk/` directory in this repository

---

*Vexidus is its own L1 blockchain. Native addresses use the `Vx` prefix. The `0x` format exists only for EVM compatibility and internal system addresses.*
