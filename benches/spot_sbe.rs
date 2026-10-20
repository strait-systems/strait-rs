use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use strait::connector::binance::spot::sbe::{DecodeLimits, Decoder, Scales};
use strait::market_data::Scale;

fn protocol(c: &mut Criterion) {
    let limits = DecodeLimits {
        max_message_bytes: 256 * 1024,
        max_levels: 10000,
        max_symbol_bytes: 32,
        max_rate_limits: 8,
    };
    let scales = Scales {
        price: Scale::new(2).unwrap(),
        quantity: Scale::new(3).unwrap(),
    };
    let mut storage = Decoder::new(limits).unwrap();
    c.bench_function("spot_sbe/depth_3_levels", |b| {
        b.iter(|| {
            let update = storage
                .decode_depth(
                    black_box(include_bytes!("../tests/fixtures/binance/spot/depth.bin")),
                    "BTCUSDT",
                    scales,
                )
                .unwrap();
            black_box((update.last_update_id, update.bids, update.asks));
        })
    });
    c.bench_function("spot_sbe/ws_snapshot_3_levels", |b| {
        b.iter(|| {
            let response = storage
                .decode_snapshot(
                    black_box(include_bytes!(
                        "../tests/fixtures/binance/spot/snapshot.bin"
                    )),
                    "snap1",
                    scales,
                )
                .unwrap();
            black_box(response);
        })
    });
    c.bench_function("spot_sbe/ws_rate_limit", |b| {
        b.iter(|| {
            black_box(
                storage
                    .decode_snapshot(
                        black_box(include_bytes!(
                            "../tests/fixtures/binance/spot/rate-limit.bin"
                        )),
                        "snap1",
                        scales,
                    )
                    .unwrap(),
            );
        })
    });
    c.bench_function("spot_sbe/depth_128_levels", |b| {
        b.iter(|| {
            let update = storage
                .decode_depth(
                    black_box(include_bytes!(
                        "../tests/fixtures/binance/spot/depth-128.bin"
                    )),
                    "BTCUSDT",
                    scales,
                )
                .unwrap();
            black_box((update.last_update_id, update.bids, update.asks));
        })
    });
    c.bench_function("spot_sbe/ws_snapshot_10000_levels", |b| {
        b.iter(|| {
            black_box(
                storage
                    .decode_snapshot(
                        black_box(include_bytes!(
                            "../tests/fixtures/binance/spot/snapshot-10000.bin"
                        )),
                        "snap1",
                        scales,
                    )
                    .unwrap(),
            );
        })
    });
}
criterion_group!(benches, protocol);
criterion_main!(benches);
