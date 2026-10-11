//! Offline protocol replay; no network connection or local L2 book is implied.
use strait::connector::binance::spot::sbe::{DecodeLimits, Decoder, Scales};
use strait::connector::binance::spot::snapshot;
use strait::connector::binance::spot::sync::{CycleToken, Decision, SequenceGate};
use strait::market_data::Scale;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let limits = DecodeLimits {
        max_message_bytes: 4096,
        max_levels: 16,
        max_symbol_bytes: 32,
        max_rate_limits: 8,
    };
    let scales = Scales {
        price: Scale::new(2)?,
        quantity: Scale::new(3)?,
    };
    let mut storage = Decoder::new(limits)?;
    let token = CycleToken {
        connection_generation: 1,
        sync_cycle: 1,
    };
    let mut gate = SequenceGate::new(token);
    let response = storage.decode_snapshot(
        include_bytes!("../tests/fixtures/binance/spot/snapshot.bin"),
        "snap1",
        scales,
    )?;
    if let snapshot::SnapshotResult::Depth(snapshot) = response.result
        && let Decision::Apply(commit) = gate.snapshot(token, snapshot.last_update_id)?
    {
        println!(
            "snapshot action: {} bids, {} asks",
            snapshot.bids.len(),
            snapshot.asks.len()
        );
        // Diagnostic acceptance only; an integrated writer must actually replace the book first.
        gate.commit(commit)?;
    }
    let update = storage.decode_depth(
        include_bytes!("../tests/fixtures/binance/spot/depth.bin"),
        "BTCUSDT",
        scales,
    )?;
    if let Decision::Apply(commit) =
        gate.update(token, update.first_update_id, update.last_update_id)?
    {
        println!(
            "depth action: [{}..{}], {} levels",
            update.first_update_id,
            update.last_update_id,
            update.bids.len() + update.asks.len()
        );
        gate.commit(commit)?;
    }
    println!(
        "protocol state: {:?}; no Live/Fresh book has been published",
        gate.state()
    );
    Ok(())
}
