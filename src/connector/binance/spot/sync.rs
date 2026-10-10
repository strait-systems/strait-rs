//! Spot-only sequence gate. No network I/O, book mutation, buffering, or publication.
//! An owning writer validates decoded inputs, applies actions, then commits the gate.
//!
//! Official [Spot local order book procedure](https://github.com/binance/binance-spot-api-docs/blob/master/web-socket-streams.md#how-to-manage-a-local-order-book-correctly),
//! also referenced by [SBE Diff. Depth Streams](https://github.com/binance/binance-spot-api-docs/blob/master/sbe-market-data-streams.md#diff-depth-streams).
//! SBE `firstBookUpdateId`/`lastBookUpdateId` correspond to JSON `U`/`u`;
//! the snapshot supplies `lastUpdateId`. These are Spot rules, not USD-M `pu` rules.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CycleToken {
    pub connection_generation: u64,
    pub sync_cycle: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SequenceState {
    AwaitingSnapshot,
    /// Snapshot installed, but no increment has bridged it yet.
    Bridging {
        last_update_id: i64,
    },
    /// Sequence valid only. This is deliberately not a consumer `Live/Fresh` status.
    Following {
        last_update_id: i64,
    },
    Invalid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SyncError {
    #[error("a book action has not been committed")]
    PendingCommit,
    #[error("snapshot is not expected")]
    UnexpectedSnapshot,
    #[error("invalid sequence range")]
    InvalidRange,
    #[error("sequence gap")]
    Gap,
    #[error("sequence range exhausted")]
    Exhausted,
    #[error("cycle must advance before restarting")]
    ReusedCycle,
    #[error("wrong commit or stale cycle")]
    WrongCommit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Decision {
    IgnoreStaleCycle,
    IgnoreDuplicate,
    AwaitSnapshot,
    Apply(Commit),
}

/// Opaque acknowledgement for a specific inspected action in the current cycle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Commit {
    token: CycleToken,
    state: SequenceState,
}

#[derive(Debug)]
pub struct SequenceGate {
    token: CycleToken,
    state: SequenceState,
    pending: Option<Commit>,
}

impl SequenceGate {
    pub fn new(token: CycleToken) -> Self {
        Self {
            token,
            state: SequenceState::AwaitingSnapshot,
            pending: None,
        }
    }
    pub fn state(&self) -> SequenceState {
        self.state
    }
    pub fn token(&self) -> CycleToken {
        self.token
    }
    /// Sync cycle is monotonic for the lifetime of this instrument, across connections.
    pub fn restart(&mut self, token: CycleToken) -> Result<(), SyncError> {
        if token.connection_generation < self.token.connection_generation
            || token.sync_cycle <= self.token.sync_cycle
        {
            return Err(SyncError::ReusedCycle);
        }
        self.token = token;
        self.state = SequenceState::AwaitingSnapshot;
        self.pending = None;
        Ok(())
    }
    pub fn invalidate(&mut self) {
        self.state = SequenceState::Invalid;
        self.pending = None;
    }
    pub fn snapshot(
        &mut self,
        token: CycleToken,
        last_update_id: i64,
    ) -> Result<Decision, SyncError> {
        if token != self.token {
            return Ok(Decision::IgnoreStaleCycle);
        }
        if self.pending.is_some() {
            return Err(SyncError::PendingCommit);
        }
        if self.state != SequenceState::AwaitingSnapshot {
            return Err(SyncError::UnexpectedSnapshot);
        }
        if last_update_id < 0 {
            self.invalidate();
            return Err(SyncError::InvalidRange);
        }
        Ok(self.stage(SequenceState::Bridging { last_update_id }))
    }
    pub fn update(
        &mut self,
        token: CycleToken,
        first: i64,
        last: i64,
    ) -> Result<Decision, SyncError> {
        if token != self.token {
            return Ok(Decision::IgnoreStaleCycle);
        }
        if self.pending.is_some() {
            return Err(SyncError::PendingCommit);
        }
        if first < 0 || first > last {
            self.invalidate();
            return Err(SyncError::InvalidRange);
        }
        let previous = match self.state {
            SequenceState::AwaitingSnapshot => return Ok(Decision::AwaitSnapshot),
            SequenceState::Invalid => return Err(SyncError::UnexpectedSnapshot),
            SequenceState::Bridging { last_update_id }
            | SequenceState::Following { last_update_id } => last_update_id,
        };
        // Spot procedure: discard covered ranges (u <= lastUpdateId); an
        // advancing range must cover lastUpdateId + 1, otherwise resynchronize.
        if last <= previous {
            return Ok(Decision::IgnoreDuplicate);
        }
        let Some(next) = previous.checked_add(1) else {
            self.invalidate();
            return Err(SyncError::Exhausted);
        };
        if first > next {
            self.invalidate();
            return Err(SyncError::Gap);
        }
        Ok(self.stage(SequenceState::Following {
            last_update_id: last,
        }))
    }
    fn stage(&mut self, state: SequenceState) -> Decision {
        let commit = Commit {
            token: self.token,
            state,
        };
        self.pending = Some(commit);
        Decision::Apply(commit)
    }
    /// Call only after successfully applying the entire corresponding book action.
    /// On book failure call `invalidate`; do not commit or publish a usable view.
    pub fn commit(&mut self, commit: Commit) -> Result<(), SyncError> {
        if self.pending != Some(commit) || commit.token != self.token {
            return Err(SyncError::WrongCommit);
        }
        self.state = commit.state;
        self.pending = None;
        Ok(())
    }
}
