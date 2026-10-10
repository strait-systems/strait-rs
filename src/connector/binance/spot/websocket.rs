//! Setup/recovery request construction for the planned persistent snapshot session.
//! Transport, retry, supervision and credentials are intentionally not exposed yet.
//!
//! Official [SBE stream connection rules](https://github.com/binance/binance-spot-api-docs/blob/master/sbe-market-data-streams.md#general-information),
//! [WebSocket API connection rules](https://github.com/binance/binance-spot-api-docs/blob/master/web-socket-api.md#general-api-information),
//! and [SBE response negotiation](https://github.com/binance/binance-spot-api-docs/blob/master/faqs/sbe_faq.md).
//! Snapshot requests use the [order book method](https://github.com/binance/binance-spot-api-docs/blob/master/web-socket-api.md#order-book).

/// SBE market stream endpoint; see the official connection rules above.
pub const DEPTH_STREAM_BASE: &str = "wss://stream-sbe.binance.com:9443";
/// WebSocket API endpoint with SBE response format and pinned API schema 3:5.
pub const SNAPSHOT_API_URL: &str =
    "wss://ws-api.binance.com:443/ws-api/v3?responseFormat=sbe&sbeSchemaId=3&sbeSchemaVersion=5";

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum RequestError {
    #[error("invalid snapshot request ID, symbol or depth limit")]
    InvalidRequest,
    #[error("snapshot request serialization failed")]
    Serialization,
}

#[derive(Debug, serde::Serialize)]
pub struct SnapshotRequest<'a> {
    id: &'a str,
    method: &'static str,
    params: Parameters<'a>,
}

#[derive(Debug, serde::Serialize)]
struct Parameters<'a> {
    symbol: &'a str,
    limit: u16,
}

impl<'a> SnapshotRequest<'a> {
    /// IDs are bounded ASCII alphanumeric identifiers, unique among in-flight requests.
    /// The session owns uniqueness and binds each ID to an instrument and cycle.
    /// ID/symbol restrictions here are local policy; the official `depth` method
    /// defines the depth limit range of 1 through 5000.
    pub fn new(id: &'a str, symbol: &'a str, limit: u16) -> Result<Self, RequestError> {
        if id.is_empty()
            || id.len() > 36
            || !id.bytes().all(|b| b.is_ascii_alphanumeric())
            || symbol.is_empty()
            || symbol.len() > 255
            || symbol.chars().any(|c| c.is_control() || c.is_whitespace())
            || !(1..=5000).contains(&limit)
        {
            return Err(RequestError::InvalidRequest);
        }
        Ok(Self {
            id,
            method: "depth",
            params: Parameters { symbol, limit },
        })
    }

    /// Setup/recovery only. The caller may reuse this buffer; allocation is not claimed
    /// absent until its capacity is sufficient. Serialization escapes untrusted text.
    pub fn encode(&self, output: &mut Vec<u8>) -> Result<(), RequestError> {
        output.clear();
        if serde_json::to_writer(&mut *output, self).is_err() {
            output.clear();
            return Err(RequestError::Serialization);
        }
        Ok(())
    }
}
