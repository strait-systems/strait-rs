//! Exchange-independent local order book components.
//!
//! This module is intended to group book data structures and update operations,
//! keeping exchange-specific transport and sequencing rules in the connectors.

pub mod l2;
