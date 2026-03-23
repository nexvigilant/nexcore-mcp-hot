//! CLI configuration via clap.
//!
//! ## Primitive Foundation
//!
//! | Primitive | Manifestation |
//! |-----------|---------------|
//! | T1: Mapping (μ) | CLI args → config struct |
//! | T1: State (ς) | Immutable runtime config |

use std::path::PathBuf;
use std::time::Duration;

use nexcore_stdio_proxy::proxy::ProxyConfig;

/// Tier: T2-C — MCP proxy CLI configuration.
#[derive(Debug, Clone, clap::Parser)]
#[command(name = "nexcore-mcp-hot")]
#[command(about = "MCP server hot-reload proxy")]
pub struct McpProxyConfig {
    /// Path to the MCP server binary to proxy.
    #[arg(long)]
    pub binary: PathBuf,

    /// Additional arguments to pass to the child binary.
    #[arg(trailing_var_arg = true)]
    pub child_args: Vec<String>,

    /// Debounce duration in seconds for file change events.
    #[arg(long, default_value = "2")]
    pub debounce_secs: u64,

    /// Grace period in seconds before SIGKILL on reload.
    #[arg(long, default_value = "3")]
    pub grace_secs: u64,

    /// Maximum number of messages to queue during reload.
    #[arg(long, default_value = "1000")]
    pub queue_capacity: usize,

    /// Enable verbose logging.
    #[arg(long, short)]
    pub verbose: bool,
}

impl McpProxyConfig {
    /// Convert to generic `ProxyConfig`.
    pub fn into_proxy_config(self) -> ProxyConfig {
        ProxyConfig {
            binary: self.binary,
            child_args: self.child_args,
            debounce: Duration::from_secs(self.debounce_secs),
            grace_period: Duration::from_secs(self.grace_secs),
            queue_capacity: self.queue_capacity,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parse_minimal_args() {
        let cfg = McpProxyConfig::try_parse_from(["nexcore-mcp-hot", "--binary", "/usr/bin/test"]);
        assert!(cfg.is_ok());
        let cfg = cfg.unwrap_or_else(|e| panic!("parse failed: {e}"));
        assert_eq!(cfg.binary, PathBuf::from("/usr/bin/test"));
        assert_eq!(cfg.debounce_secs, 2);
        assert_eq!(cfg.grace_secs, 3);
        assert_eq!(cfg.queue_capacity, 1000);
    }

    #[test]
    fn parse_all_args() {
        let cfg = McpProxyConfig::try_parse_from([
            "nexcore-mcp-hot",
            "--binary",
            "/usr/bin/test",
            "--debounce-secs",
            "5",
            "--grace-secs",
            "10",
            "--queue-capacity",
            "500",
            "--verbose",
        ]);
        assert!(cfg.is_ok());
        let cfg = cfg.unwrap_or_else(|e| panic!("parse failed: {e}"));
        assert_eq!(cfg.debounce_secs, 5);
        assert_eq!(cfg.grace_secs, 10);
        assert_eq!(cfg.queue_capacity, 500);
        assert!(cfg.verbose);
    }

    #[test]
    fn duration_conversions() {
        let cfg = McpProxyConfig::try_parse_from(["nexcore-mcp-hot", "--binary", "/bin/sh"])
            .unwrap_or_else(|e| panic!("parse failed: {e}"));
        let proxy = cfg.into_proxy_config();
        assert_eq!(proxy.debounce, Duration::from_secs(2));
        assert_eq!(proxy.grace_period, Duration::from_secs(3));
    }

    #[test]
    fn into_proxy_config_maps_all_fields() {
        let cfg = McpProxyConfig::try_parse_from([
            "nexcore-mcp-hot",
            "--binary",
            "/usr/bin/test",
            "--debounce-secs",
            "5",
            "--grace-secs",
            "10",
            "--queue-capacity",
            "500",
        ])
        .unwrap_or_else(|e| panic!("parse failed: {e}"));

        let proxy = cfg.into_proxy_config();
        assert_eq!(proxy.binary, PathBuf::from("/usr/bin/test"));
        assert_eq!(proxy.debounce, Duration::from_secs(5));
        assert_eq!(proxy.grace_period, Duration::from_secs(10));
        assert_eq!(proxy.queue_capacity, 500);
    }
}
