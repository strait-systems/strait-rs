//! Binance Spot public market data connector.
//!
//! Product-specific transport, decoding, REST snapshots, synchronization, and recovery.

pub mod depth;
pub mod snapshot;
pub mod sync;
pub mod websocket;
