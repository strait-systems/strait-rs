//! Synchronization of Binance depth snapshots and incremental updates.
//!
//! This module is intended to buffer updates during snapshot retrieval, align the
//! snapshot with the stream, validate sequence continuity, and trigger
//! resynchronization when the local book's integrity can no longer be established.
//! It will emit ordered book actions without network I/O; the shared L2 book
//! is integrated after both product connectors have been implemented.
