//! Offline comparison helper: run the identical source against baseline/candidate
//! crates and compare output. Only decoded values/acceptance enter each digest;
//! error precedence for multiply-invalid messages is intentionally excluded.
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{self, BufWriter, Write};
use strait::connector::binance::spot::sbe::{DecodeLimits, Decoder, Scales};
use strait::connector::binance::spot::snapshot::SnapshotResult;
use strait::market_data::{Level, Scale};

fn levels(levels: &[Level], hash: &mut DefaultHasher) {
    levels.len().hash(hash);
    for level in levels {
        (level.price.units(), level.quantity.units()).hash(hash);
    }
}

fn digest(decoder: &mut Decoder, input: &[u8], stream: bool, scales: Scales) -> Option<u64> {
    let mut hash = DefaultHasher::new();
    if stream {
        let update = decoder.decode_depth(input, "BTCUSDT", scales).ok()?;
        (
            update.symbol,
            update.event_time_us,
            update.first_update_id,
            update.last_update_id,
        )
            .hash(&mut hash);
        levels(update.bids, &mut hash);
        levels(update.asks, &mut hash);
    } else {
        let response = decoder.decode_snapshot(input, "snap1", scales).ok()?;
        (
            response.request_id,
            response.status,
            response.schema_deprecated,
        )
            .hash(&mut hash);
        for rate in response.rate_limits.iter() {
            (
                rate.kind,
                rate.interval,
                rate.interval_num,
                rate.limit,
                rate.count,
            )
                .hash(&mut hash);
        }
        match response.result {
            SnapshotResult::Depth(snapshot) => {
                0u8.hash(&mut hash);
                snapshot.last_update_id.hash(&mut hash);
                levels(snapshot.bids, &mut hash);
                levels(snapshot.asks, &mut hash);
            }
            SnapshotResult::Error(error) => {
                1u8.hash(&mut hash);
                (
                    error.code,
                    error.message,
                    error.server_time_us,
                    error.retry_after_us,
                )
                    .hash(&mut hash);
            }
        }
    }
    Some(hash.finish())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut decoder = Decoder::new(DecodeLimits {
        max_message_bytes: 256 * 1024,
        max_levels: 10000,
        max_symbol_bytes: 32,
        max_rate_limits: 8,
    })?;
    let scales = Scales {
        price: Scale::new(2)?,
        quantity: Scale::new(3)?,
    };
    let mut output = BufWriter::new(io::stdout().lock());
    for (fixture, stream) in [
        (
            &include_bytes!("../tests/fixtures/binance/spot/depth.bin")[..],
            true,
        ),
        (
            &include_bytes!("../tests/fixtures/binance/spot/depth-128.bin")[..],
            true,
        ),
        (
            &include_bytes!("../tests/fixtures/binance/spot/snapshot.bin")[..],
            false,
        ),
        (
            &include_bytes!("../tests/fixtures/binance/spot/rate-limit.bin")[..],
            false,
        ),
    ] {
        writeln!(
            output,
            "{:?}",
            digest(&mut decoder, fixture, stream, scales)
        )?;
        for end in 0..fixture.len() {
            writeln!(
                output,
                "{:?}",
                digest(&mut decoder, &fixture[..end], stream, scales)
            )?;
        }
        let mut bytes = fixture.to_vec();
        for offset in 0..bytes.len() {
            let original = bytes[offset];
            for value in 0..=255 {
                bytes[offset] = value;
                writeln!(output, "{:?}", digest(&mut decoder, &bytes, stream, scales))?;
            }
            bytes[offset] = original;
        }
        writeln!(
            output,
            "{:?}",
            digest(&mut decoder, fixture, stream, scales)
        )?;
    }
    writeln!(
        output,
        "{:?}",
        digest(
            &mut decoder,
            include_bytes!("../tests/fixtures/binance/spot/snapshot-10000.bin"),
            false,
            scales
        )
    )?;
    Ok(())
}
