//! MCP hot-reload proxy entry point.

#![forbid(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

use clap::Parser;
use tracing_subscriber::EnvFilter;

use nexcore_mcp_hot::capture::McpCapture;
use nexcore_mcp_hot::config::McpProxyConfig;
use nexcore_stdio_proxy::StdioProxy;

fn init_logging(verbose: bool) {
    let filter = if verbose {
        EnvFilter::new("nexcore_mcp_hot=debug,nexcore_stdio_proxy=debug")
    } else {
        EnvFilter::new("nexcore_mcp_hot=info,nexcore_stdio_proxy=info")
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_target(false)
        .compact()
        .init();
}

#[tokio::main]
async fn main() -> nexcore_error::Result<()> {
    let config = McpProxyConfig::parse();
    init_logging(config.verbose);

    tracing::info!(
        "nexcore-mcp-hot starting: binary={}",
        config.binary.display()
    );

    let proxy_config = config.into_proxy_config();
    let mut proxy = StdioProxy::<McpCapture>::new(proxy_config);
    proxy.run().await?;

    Ok(())
}
