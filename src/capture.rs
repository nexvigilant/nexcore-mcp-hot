//! MCP initialize handshake capture & replay.
//!
//! ## Primitive Foundation
//!
//! | Primitive | Manifestation |
//! |-----------|---------------|
//! | T1: State (ς) | Captured init request/response |
//! | T1: Sequence (σ) | Replay: request → response → notification |
//! | T1: Existence (∃) | Optional captured state |

use serde_json::Value;
use tokio::sync::mpsc;

use nexcore_stdio_proxy::child::{ChildLine, ManagedChild};
use nexcore_stdio_proxy::error::{ProxyError, Result};
use nexcore_stdio_proxy::protocol::ProtocolCapture;

/// Tier: T3 — MCP-specific handshake capture implementing `ProtocolCapture`.
#[derive(Debug, Clone, Default)]
pub struct McpCapture {
    /// The `initialize` request from the client.
    init_request: Option<String>,
    /// The `initialize` response from the server.
    init_response: Option<String>,
    /// The `notifications/initialized` from the client.
    initialized_notification: Option<String>,
}

// ── JSON-RPC detection helpers ──────────────────────────────────

/// Extract the "method" field from a JSON-RPC message.
fn parse_method(line: &str) -> Option<String> {
    let val: Value = serde_json::from_str(line).ok()?;
    val.get("method")?.as_str().map(String::from)
}

/// Check if a JSON-RPC message is an `initialize` request.
fn is_init_request(line: &str) -> bool {
    parse_method(line).as_deref() == Some("initialize")
}

/// Check if a JSON-RPC message is a `notifications/initialized`.
fn is_initialized_notification(line: &str) -> bool {
    parse_method(line).as_deref() == Some("notifications/initialized")
}

/// Check if a JSON-RPC message is an `initialize` response (has result with capabilities).
fn is_init_response(line: &str) -> bool {
    let val: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => return false,
    };
    val.get("result")
        .and_then(|r| r.get("capabilities"))
        .is_some()
}

/// Consume the init response from child (discard, don't forward to client).
async fn consume_init_response(child_rx: &mut mpsc::Receiver<ChildLine>) -> Result<()> {
    loop {
        let timeout =
            tokio::time::timeout(std::time::Duration::from_secs(10), child_rx.recv()).await;

        match timeout {
            Ok(Some(ChildLine::Stdout(_))) => {
                tracing::debug!("Consumed init response from new child");
                return Ok(());
            }
            Ok(Some(ChildLine::Stderr(s))) => {
                tracing::debug!("Child stderr during init: {s}");
                // Loop to try again
            }
            Ok(None) => {
                return Err(ProxyError::Reload("child closed during init replay".into()));
            }
            Err(_) => {
                return Err(ProxyError::Reload(
                    "timeout waiting for init response".into(),
                ));
            }
        }
    }
}

// ── ProtocolCapture impl ─────────────────────────────────────────

impl ProtocolCapture for McpCapture {
    fn try_capture_client(&mut self, line: &str) -> bool {
        if is_init_request(line) {
            tracing::debug!("Captured initialize request");
            self.init_request = Some(line.to_string());
            return true;
        }
        if is_initialized_notification(line) {
            tracing::debug!("Captured initialized notification");
            self.initialized_notification = Some(line.to_string());
            return true;
        }
        false
    }

    fn try_capture_server(&mut self, line: &str) -> bool {
        if self.init_request.is_some() && self.init_response.is_none() && is_init_response(line) {
            tracing::debug!("Captured initialize response");
            self.init_response = Some(line.to_string());
            return true;
        }
        false
    }

    fn is_complete(&self) -> bool {
        self.init_request.is_some() && self.init_response.is_some()
    }

    async fn replay_handshake(
        &self,
        child: &ManagedChild,
        child_rx: &mut mpsc::Receiver<ChildLine>,
    ) -> Result<()> {
        if !self.is_complete() {
            tracing::warn!("No captured init handshake to replay");
            return Ok(());
        }

        if let Some(ref req) = self.init_request {
            tracing::debug!("Replaying initialize request");
            child.send_line(req).await?;
        }

        consume_init_response(child_rx).await?;

        if let Some(ref notif) = self.initialized_notification {
            tracing::debug!("Replaying initialized notification");
            child.send_line(notif).await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INIT_REQ: &str =
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#;
    const INIT_RESP: &str = r#"{"jsonrpc":"2.0","id":1,"result":{"capabilities":{"tools":{}}}}"#;
    const INIT_NOTIF: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    const NORMAL_REQ: &str = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#;

    #[test]
    fn detect_init_request() {
        assert!(is_init_request(INIT_REQ));
        assert!(!is_init_request(NORMAL_REQ));
    }

    #[test]
    fn detect_init_response() {
        assert!(is_init_response(INIT_RESP));
        assert!(!is_init_response(INIT_REQ));
    }

    #[test]
    fn detect_initialized_notification() {
        assert!(is_initialized_notification(INIT_NOTIF));
        assert!(!is_initialized_notification(INIT_REQ));
    }

    #[test]
    fn capture_full_handshake() {
        let mut cap = McpCapture::default();
        assert!(!cap.is_complete());

        assert!(cap.try_capture_client(INIT_REQ));
        assert!(!cap.is_complete()); // no response yet

        assert!(cap.try_capture_server(INIT_RESP));
        assert!(cap.is_complete());

        assert!(cap.try_capture_client(INIT_NOTIF));
        assert_eq!(cap.init_request.as_deref(), Some(INIT_REQ));
        assert_eq!(cap.initialized_notification.as_deref(), Some(INIT_NOTIF));
    }

    #[test]
    fn normal_messages_not_captured() {
        let mut cap = McpCapture::default();
        assert!(!cap.try_capture_client(NORMAL_REQ));
        assert!(!cap.try_capture_server(NORMAL_REQ));
    }

    #[test]
    fn invalid_json_not_captured() {
        let mut cap = McpCapture::default();
        assert!(!cap.try_capture_client("not json"));
        assert!(!cap.try_capture_server("not json"));
    }
}
