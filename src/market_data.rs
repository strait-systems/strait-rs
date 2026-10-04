//! Exchange-independent market data types shared by connectors and the book.
//!
//! Instrument identity (venue, product, and venue symbol), exact prices and quantities,
//! snapshots, and absolute price-level updates will be agreed here before parallel connector
//! implementation. Quantity units must remain explicit.
//!
//! Wire messages, sequence rules, subscription parameters, synchronization state,
//! and recovery cycle identifiers remain inside the product connectors.
