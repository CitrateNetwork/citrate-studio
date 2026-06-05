//! Citrate Studio — live chain reads (STUDIO-6).
//!
//! Real JSON-RPC against `rpc.citrate.ai` (chain 40204). Reads are
//! unauthenticated, so they run in the default build over ureq — the anchor
//! marks and chain status are real, not modeled. Chain *writes* (anchoring via
//! `RecorderClient`) need a funded signer + gas and are gated on a key seam
//! (`anchor_*` below) — surfaced honestly, never faked.

use std::time::Duration;

pub const RPC_URL: &str = "https://rpc.citrate.ai";
pub const CHAIN_ID: u64 = 40204;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChainStatus {
    pub online: bool,
    pub chain_id: u64,
    pub block: u64,
}

fn hex_to_u64(s: &str) -> Option<u64> {
    u64::from_str_radix(s.trim_start_matches("0x"), 16).ok()
}

/// One JSON-RPC call returning the `result` field as a string.
pub fn rpc_call(url: &str, method: &str, params: serde_json::Value) -> Option<String> {
    let body = serde_json::json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
    let resp = ureq::post(url)
        .timeout(Duration::from_secs(6))
        .set("content-type", "application/json")
        .send_string(&body.to_string())
        .ok()?;
    let v: serde_json::Value = serde_json::from_str(&resp.into_string().ok()?).ok()?;
    v.get("result").and_then(|r| r.as_str()).map(|s| s.to_string())
}

/// Live chain status — `eth_chainId` + `eth_blockNumber` against `rpc.citrate.ai`.
/// `online: false` (and zeros) when the endpoint is unreachable — surfaced, not faked.
pub fn status() -> ChainStatus {
    status_at(RPC_URL)
}

pub fn status_at(url: &str) -> ChainStatus {
    let chain_id = rpc_call(url, "eth_chainId", serde_json::json!([])).and_then(|s| hex_to_u64(&s));
    let block = rpc_call(url, "eth_blockNumber", serde_json::json!([])).and_then(|s| hex_to_u64(&s));
    ChainStatus {
        online: chain_id.is_some(),
        chain_id: chain_id.unwrap_or(0),
        block: block.unwrap_or(0),
    }
}

/// Anchoring (chain write) requires a funded signer + gas. The seam is present;
/// the capability is gated until a key + funding are configured (STUDIO-6+).
pub fn anchor_available() -> bool {
    std::env::var("CITRATE_ANCHOR_KEY").is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_parsing() {
        assert_eq!(hex_to_u64("0x9d0c"), Some(40204));
        assert_eq!(hex_to_u64("0x0"), Some(0));
        assert_eq!(hex_to_u64("nope"), None);
    }

    #[test]
    fn rpc_call_against_a_mock() {
        let srv = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let port = srv.server_addr().to_ip().unwrap().port();
        let h = std::thread::spawn(move || {
            if let Some(req) = srv.incoming_requests().next() {
                let body = r#"{"jsonrpc":"2.0","result":"0x9d0c","id":1}"#;
                let _ = req.respond(tiny_http::Response::from_string(body));
            }
        });
        let url = format!("http://127.0.0.1:{port}");
        let st = status_at(&url);
        // chainId mock answers; block uses the same mock's single response → online
        assert!(st.online);
        assert_eq!(st.chain_id, 40204);
        h.join().unwrap();
    }

    #[test]
    fn live_chain_is_40204_when_reachable() {
        // Tolerant of an offline sandbox — asserts only when the endpoint answers.
        let st = status();
        if st.online {
            assert_eq!(st.chain_id, CHAIN_ID, "rpc.citrate.ai is chain 40204");
            assert!(st.block > 0, "live block height");
            eprintln!("live chain: id={} block={}", st.chain_id, st.block);
        } else {
            eprintln!("skipping: rpc.citrate.ai unreachable in this environment");
        }
    }
}
