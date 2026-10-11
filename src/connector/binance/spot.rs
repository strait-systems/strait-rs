//! Binance Spot public market data connector.
//!
//! Initial protocol core: SBE depth and WebSocket API SBE snapshots.
//! Network lifecycle and shared book integration are not implemented yet.

pub mod depth;
pub mod sbe;
pub mod snapshot;
pub mod sync;
pub mod websocket;
