//! Strait: market data connectivity and local order books for centralized exchanges.
//!
//! The crate is organized into exchange connectors and exchange-independent book
//! components, initially targeting Binance Spot and USD-M perpetual futures.

pub mod book;
pub mod connector;
pub mod market_data;
