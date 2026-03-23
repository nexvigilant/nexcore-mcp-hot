//! # NexVigilant Core — mcp-hot
//!
//! MCP server hot-reload proxy. Watches a binary file and transparently
//! restarts the child MCP server when it changes, replaying the initialize
//! handshake so the client sees an uninterrupted connection.
//!
//! ## Architecture
//!
//! ```text
//! Client ←stdio→ StdioProxy<McpCapture> ←stdio→ MCP Server (child)
//!                       ↑
//!                  BinaryWatcher
//!                  (binary file)
//! ```
//!
//! ## Primitive Foundation
//!
//! | Primitive | Module | Manifestation |
//! |-----------|--------|---------------|
//! | T1: State (ς) | (via stdio-proxy) | State machine lifecycle |
//! | T1: Sequence (σ) | (via stdio-proxy) | Line I/O, FIFO buffering |
//! | T1: Mapping (μ) | capture | MCP init handshake capture/replay |
//!
//! This crate is a thin MCP-specific wrapper around `nexcore-stdio-proxy`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod capture;
pub mod config;
