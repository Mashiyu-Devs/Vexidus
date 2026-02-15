//! Validator configuration — TOML-based config and systemd service generation.

use serde::{Deserialize, Serialize};
use std::path::Path;
use anyhow::Result;

/// Validator node configuration (stored as `validator.toml`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorConfig {
    /// Path to Ed25519 signing key file (64 hex chars)
    pub keypair_path: String,

    /// RPC URL of the node to connect to
    #[serde(default = "default_rpc_url")]
    pub rpc_url: String,

    /// P2P listen port
    #[serde(default = "default_p2p_port")]
    pub p2p_port: u16,

    /// RPC listen port
    #[serde(default = "default_rpc_port")]
    pub rpc_port: u16,

    /// External address for NAT traversal (e.g. "/ip4/51.255.80.34/tcp/9944")
    pub external_addr: Option<String>,

    /// Staker address (the account that holds the staked VXS)
    pub staker_address: Option<String>,

    /// Data directory for chain state
    #[serde(default = "default_data_dir")]
    pub data_dir: String,

    /// Comma-separated bootstrap node multiaddrs
    pub bootnodes: Option<String>,

    /// Enable verbose logging
    #[serde(default)]
    pub verbose: bool,
}

fn default_rpc_url() -> String { "http://localhost:9933".into() }
fn default_p2p_port() -> u16 { 9944 }
fn default_rpc_port() -> u16 { 9933 }
fn default_data_dir() -> String { "./data".into() }

impl Default for ValidatorConfig {
    fn default() -> Self {
        Self {
            keypair_path: "./validator.key".into(),
            rpc_url: default_rpc_url(),
            p2p_port: default_p2p_port(),
            rpc_port: default_rpc_port(),
            external_addr: None,
            staker_address: None,
            data_dir: default_data_dir(),
            bootnodes: None,
            verbose: false,
        }
    }
}

impl ValidatorConfig {
    /// Load config from a TOML file.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save config to a TOML file.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Generate CLI args for `vexidus-node` from this config.
    pub fn to_node_args(&self) -> Vec<String> {
        let mut args = vec![
            "--data-dir".into(), self.data_dir.clone(),
            "--rpc-port".into(), self.rpc_port.to_string(),
            "--p2p-port".into(), self.p2p_port.to_string(),
            "--validator-key".into(), self.keypair_path.clone(),
        ];
        if let Some(addr) = &self.external_addr {
            args.push("--external-addr".into());
            args.push(addr.clone());
        }
        if let Some(nodes) = &self.bootnodes {
            args.push("--bootnodes".into());
            args.push(nodes.clone());
        }
        if self.verbose {
            args.push("--verbose".into());
        }
        args
    }

    /// Generate a systemd service unit file for this validator.
    pub fn generate_systemd_service(&self, binary_path: &str, working_dir: &str) -> String {
        let args = self.to_node_args().join(" ");
        format!(
r#"[Unit]
Description=Vexidus Validator Node
After=network.target
Wants=network-online.target

[Service]
Type=simple
User=vexidus
Group=vexidus
WorkingDirectory={working_dir}
ExecStart={binary_path} {args}
Restart=always
RestartSec=5
LimitNOFILE=65536

# Hardening
ProtectSystem=full
ProtectHome=read-only
NoNewPrivileges=true

[Install]
WantedBy=multi-user.target
"#)
    }

    /// Write the systemd service file to a given path.
    pub fn write_systemd_service<P: AsRef<Path>>(
        &self,
        path: P,
        binary_path: &str,
        working_dir: &str,
    ) -> Result<()> {
        let content = self.generate_systemd_service(binary_path, working_dir);
        std::fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ValidatorConfig::default();
        assert_eq!(config.p2p_port, 9944);
        assert_eq!(config.rpc_port, 9933);
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("validator.toml");

        let config = ValidatorConfig {
            keypair_path: "/opt/vexidus/validator.key".into(),
            external_addr: Some("/ip4/51.255.80.34/tcp/9944".into()),
            bootnodes: Some("/ip4/10.0.0.1/tcp/9944/p2p/12D3KooW...".into()),
            ..Default::default()
        };
        config.save(&path).unwrap();

        let loaded = ValidatorConfig::load(&path).unwrap();
        assert_eq!(loaded.keypair_path, "/opt/vexidus/validator.key");
        assert_eq!(loaded.external_addr.unwrap(), "/ip4/51.255.80.34/tcp/9944");
    }

    #[test]
    fn test_systemd_generation() {
        let config = ValidatorConfig {
            keypair_path: "/opt/vexidus/validator.key".into(),
            p2p_port: 9945,
            ..Default::default()
        };
        let service = config.generate_systemd_service(
            "/usr/local/bin/vexidus-node",
            "/opt/vexidus",
        );
        assert!(service.contains("ExecStart=/usr/local/bin/vexidus-node"));
        assert!(service.contains("--validator-key"));
        assert!(service.contains("--p2p-port 9945"));
    }

    #[test]
    fn test_to_node_args() {
        let config = ValidatorConfig {
            verbose: true,
            bootnodes: Some("/ip4/10.0.0.1/tcp/9944".into()),
            ..Default::default()
        };
        let args = config.to_node_args();
        assert!(args.contains(&"--verbose".to_string()));
        assert!(args.contains(&"--bootnodes".to_string()));
    }
}
