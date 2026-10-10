use strait::connector::binance::spot::sbe::{DecodeError, DecodeLimits, Decoder, Scales};
use strait::connector::binance::spot::snapshot;
use strait::connector::binance::spot::sync::{
    CycleToken, Decision, SequenceGate, SequenceState, SyncError,
};
use strait::connector::binance::spot::websocket::SnapshotRequest;
use strait::market_data::{NumericError, Price, Quantity, Scale};

const DEPTH: &[u8] = include_bytes!("fixtures/binance/spot/depth.bin");
const SNAPSHOT: &[u8] = include_bytes!("fixtures/binance/spot/snapshot.bin");
const RATE_LIMIT: &[u8] = include_bytes!("fixtures/binance/spot/rate-limit.bin");

#[test]
fn full_depth_snapshot_checks_total_count_boundary_and_order() {
    let bytes = include_bytes!("fixtures/binance/spot/snapshot-10000.bin");
    let mut l = limits();
    l.max_message_bytes = bytes.len();
    l.max_levels = 10000;
    let mut buffer = Decoder::new(l).unwrap();
    let response = buffer.decode_snapshot(bytes, "snap1", scales()).unwrap();
    let snapshot::SnapshotResult::Depth(d) = response.result else {
        panic!()
    };
    assert_eq!((d.bids.len(), d.asks.len()), (5000, 5000));
    assert_eq!(d.bids[4999].price.units(), 6_495_001);
    assert_eq!(d.asks[4999].price.units(), 6_505_099);
    l.max_levels = 9999;
    let mut limited = Decoder::new(l).unwrap();
    assert_eq!(
        limited
            .decode_snapshot(bytes, "snap1", scales())
            .unwrap_err(),
        DecodeError::LimitExceeded
    );
    let mut malformed = SNAPSHOT.to_vec();
    // Give the second bid the first bid's price: duplicates/unsorted snapshot must fail.
    malformed[84..92].copy_from_slice(&6_500_000_i64.to_le_bytes());
    assert_eq!(
        buffer
            .decode_snapshot(&malformed, "snap1", scales())
            .unwrap_err(),
        DecodeError::InvalidField
    );
}

fn limits() -> DecodeLimits {
    DecodeLimits {
        max_message_bytes: 4096,
        max_levels: 16,
        max_symbol_bytes: 32,
        max_rate_limits: 8,
    }
}
fn scales() -> Scales {
    Scales {
        price: Scale::new(2).unwrap(),
        quantity: Scale::new(3).unwrap(),
    }
}
fn token() -> CycleToken {
    CycleToken {
        connection_generation: 1,
        sync_cycle: 1,
    }
}
fn committed(gate: &mut SequenceGate, decision: Decision) {
    let Decision::Apply(commit) = decision else {
        panic!("expected apply action")
    };
    gate.commit(commit).unwrap();
}

#[test]
fn depth_fixture_has_independently_specified_values() {
    let mut buffer = Decoder::new(limits()).unwrap();
    let d = buffer.decode_depth(DEPTH, "BTCUSDT", scales()).unwrap();
    assert_eq!(
        (d.symbol, d.first_update_id, d.last_update_id),
        ("BTCUSDT", 101, 103)
    );
    assert_eq!(d.event_time_us, 1_700_000_000_000_000);
    assert_eq!(d.bids.len(), 2);
    assert_eq!(d.asks.len(), 1);
    assert_eq!(d.bids[0].price.units(), 6_500_000);
    assert_eq!(d.bids[0].quantity.units(), 1250);
    assert_eq!(d.bids[1].quantity.units(), 0);
    assert_eq!(d.asks[0].price.units(), 6_500_100);
    assert_eq!(d.asks[0].quantity.units(), 2000);
    assert_eq!(d.scales, scales());
}

#[test]
fn snapshot_wrapper_request_and_limits_are_preserved() {
    let mut buffer = Decoder::new(limits()).unwrap();
    let response = buffer.decode_snapshot(SNAPSHOT, "snap1", scales()).unwrap();
    assert_eq!(response.request_id, "snap1");
    assert_eq!(response.status, 200);
    assert!(!response.schema_deprecated);
    let rates: Vec<_> = response.rate_limits.iter().collect();
    assert_eq!(rates.len(), 1);
    assert_eq!(
        (rates[0].kind, rates[0].interval, rates[0].interval_num),
        (2, 1, 1)
    );
    assert_eq!((rates[0].limit, rates[0].count), (6000, 250));
    let snapshot::SnapshotResult::Depth(d) = response.result else {
        panic!("expected depth")
    };
    assert_eq!(d.scales, scales());
    assert_eq!(d.last_update_id, 100);
    assert_eq!(d.bids[0].price.units(), 6_500_000);
    assert_eq!(d.bids[0].quantity.units(), 1500);
    assert_eq!(d.bids[1].price.units(), 6_499_900);
    assert_eq!(d.bids[1].quantity.units(), 2500);
    assert_eq!(d.asks[0].price.units(), 6_500_100);
}

#[test]
fn rate_limit_error_keeps_absolute_server_cooldown() {
    let mut buffer = Decoder::new(limits()).unwrap();
    let response = buffer
        .decode_snapshot(RATE_LIMIT, "snap1", scales())
        .unwrap();
    assert_eq!(response.status, 429);
    let snapshot::SnapshotResult::Error(error) = response.result else {
        panic!("expected error")
    };
    assert_eq!(error.code, -1003);
    assert_eq!(error.message, "Too many requests");
    assert_eq!(error.server_time_us, Some(1_700_000_000_000_000));
    assert_eq!(error.retry_after_us, Some(1_700_000_005_000_000));
}

#[test]
fn every_truncated_fixture_fails_and_storage_is_reusable() {
    let mut buffer = Decoder::new(limits()).unwrap();
    for end in 0..DEPTH.len() {
        assert!(
            buffer
                .decode_depth(&DEPTH[..end], "BTCUSDT", scales())
                .is_err()
        );
        assert!(buffer.decode_depth(DEPTH, "BTCUSDT", scales()).is_ok());
    }
    for fixture in [SNAPSHOT, RATE_LIMIT] {
        for end in 0..fixture.len() {
            assert!(
                buffer
                    .decode_snapshot(&fixture[..end], "snap1", scales())
                    .is_err()
            );
            assert!(buffer.decode_snapshot(SNAPSHOT, "snap1", scales()).is_ok());
        }
    }
}

#[test]
fn limits_layout_identity_and_corruption_fail_closed() {
    let mut buffer = Decoder::new(limits()).unwrap();
    assert_eq!(
        buffer.decode_depth(DEPTH, "ETHUSDT", scales()).unwrap_err(),
        DecodeError::InvalidField
    );
    assert_eq!(
        buffer
            .decode_snapshot(SNAPSHOT, "snap2", scales())
            .unwrap_err(),
        DecodeError::InvalidField
    );
    for offset in [0, 2, 4, 6, 34] {
        // Root and first group layout.
        let mut bytes = DEPTH.to_vec();
        bytes[offset] ^= 1;
        assert!(buffer.decode_depth(&bytes, "BTCUSDT", scales()).is_err());
    }
    let mut bytes = DEPTH.to_vec();
    bytes.push(0);
    assert_eq!(
        buffer
            .decode_depth(&bytes, "BTCUSDT", scales())
            .unwrap_err(),
        DecodeError::TrailingData
    );
    let mut small = limits();
    small.max_levels = 2;
    let mut bounded = Decoder::new(small).unwrap();
    assert_eq!(
        bounded
            .decode_depth(DEPTH, "BTCUSDT", scales())
            .unwrap_err(),
        DecodeError::LimitExceeded
    );
    small.max_levels = 3;
    let mut boundary = Decoder::new(small).unwrap();
    assert!(boundary.decode_depth(DEPTH, "BTCUSDT", scales()).is_ok());
    small.max_message_bytes = DEPTH.len() - 1;
    let mut bounded = Decoder::new(small).unwrap();
    assert_eq!(
        bounded
            .decode_depth(DEPTH, "BTCUSDT", scales())
            .unwrap_err(),
        DecodeError::LimitExceeded
    );
    let mut corrupt = SNAPSHOT.to_vec();
    // Wrapper (11), rate group (4+19), id (1+5), payload length (4), header (8),
    // root (10), group (6), price (8): first bid quantity is at byte 76.
    corrupt[76..84].copy_from_slice(&0_i64.to_le_bytes());
    assert!(buffer.decode_snapshot(&corrupt, "snap1", scales()).is_err());
}

#[test]
fn exact_scaling_rejects_rounding_negative_null_and_overflow() {
    let s = Scale::new(2).unwrap();
    assert_eq!(s.convert(12300, -4), Ok(123));
    assert_eq!(s.convert(12301, -4), Err(NumericError::PrecisionLoss));
    assert_eq!(s.convert(123, 1), Ok(123000));
    assert_eq!(s.convert(-1, -2), Err(NumericError::InvalidValue));
    assert_eq!(s.convert(i64::MIN, -2), Err(NumericError::InvalidValue));
    assert_eq!(s.convert(1, i8::MIN), Err(NumericError::InvalidValue));
    assert_eq!(s.convert(i64::MAX, 30), Err(NumericError::Overflow));
    assert_eq!(Scale::new(19), Err(NumericError::UnsupportedScale));
    assert!(Price::from_sbe(0, -2, s).is_err());
    assert_eq!(Quantity::from_sbe(0, -2, s).unwrap().units(), 0);
    // This identity property is independent of wire decoding.
    for decimal_places in 0..=18 {
        let scale = Scale::new(decimal_places).unwrap();
        for m in [0, 1, 99, 12345, i64::MAX] {
            assert_eq!(
                scale.convert(m, -(decimal_places as i8)),
                Ok(u128::try_from(m).unwrap())
            );
        }
    }
}

#[test]
fn unsigned_scaling_checks_extended_range_and_factor_boundaries() {
    let scale = Scale::new(0).unwrap();
    let ten_to_38 = 10_u128.pow(38);
    // Independently specified decimal value beyond the former signed range.
    let extended = 200_000_000_000_000_000_000_000_000_000_000_000_000_u128;
    assert_eq!(scale.convert(2, 38), Ok(extended));
    assert_eq!(Price::from_sbe(2, 38, scale).unwrap().units(), extended);
    assert_eq!(Quantity::from_sbe(2, 38, scale).unwrap().units(), extended);
    assert_eq!(scale.convert(3, 38), Ok(3 * ten_to_38));
    assert_eq!(scale.convert(4, 38), Err(NumericError::Overflow));
    assert_eq!(scale.convert(1, 39), Err(NumericError::Overflow));
    // Zero must still validate the exponent, including empty decoder groups.
    assert_eq!(scale.convert(0, 38), Ok(0));
    assert_eq!(scale.convert(0, 39), Err(NumericError::Overflow));
    assert_eq!(scale.convert(1, -38), Err(NumericError::PrecisionLoss));
    assert_eq!(scale.convert(0, -38), Ok(0));
    assert_eq!(scale.convert(0, -39), Err(NumericError::Overflow));
    assert_eq!(
        Price::from_sbe(-1, 0, scale),
        Err(NumericError::InvalidValue)
    );
    assert_eq!(
        Quantity::from_sbe(-1, 0, scale),
        Err(NumericError::InvalidValue)
    );
    // Maximum supported scale still preserves the largest nonnegative wire tail.
    assert_eq!(
        Scale::new(18).unwrap().convert(i64::MAX, 0),
        Ok(9_223_372_036_854_775_807_000_000_000_000_000_000_u128)
    );
}

#[test]
fn sequence_moves_only_after_successful_application() {
    let mut gate = SequenceGate::new(token());
    assert_eq!(gate.update(token(), 101, 103), Ok(Decision::AwaitSnapshot));
    let action = gate.snapshot(token(), 100).unwrap();
    assert_eq!(gate.state(), SequenceState::AwaitingSnapshot);
    assert_eq!(
        gate.update(token(), 101, 103),
        Err(SyncError::PendingCommit)
    );
    committed(&mut gate, action);
    assert_eq!(
        gate.state(),
        SequenceState::Bridging {
            last_update_id: 100
        }
    );
    assert_eq!(gate.update(token(), 99, 100), Ok(Decision::IgnoreDuplicate));
    let action = gate.update(token(), 99, 103).unwrap(); // Overlap is valid.
    committed(&mut gate, action);
    assert_eq!(
        gate.state(),
        SequenceState::Following {
            last_update_id: 103
        }
    );
    assert_eq!(gate.update(token(), 105, 106), Err(SyncError::Gap));
    assert_eq!(gate.state(), SequenceState::Invalid);
}

#[test]
fn old_cycle_and_commit_cannot_restore_invalid_state() {
    let mut gate = SequenceGate::new(token());
    let Decision::Apply(commit) = gate.snapshot(token(), 100).unwrap() else {
        panic!()
    };
    gate.invalidate();
    assert_eq!(gate.commit(commit), Err(SyncError::WrongCommit));
    assert_eq!(gate.restart(token()), Err(SyncError::ReusedCycle));
    let next = CycleToken {
        connection_generation: 2,
        sync_cycle: 2,
    };
    gate.restart(next).unwrap();
    assert_eq!(gate.snapshot(token(), 999), Ok(Decision::IgnoreStaleCycle));
    assert_eq!(gate.update(token(), 1, 2), Ok(Decision::IgnoreStaleCycle));
    assert_eq!(gate.commit(commit), Err(SyncError::WrongCommit));
    assert_eq!(gate.state(), SequenceState::AwaitingSnapshot);
}

#[test]
fn generated_sequence_ranges_obey_independent_interval_oracle() {
    for previous in 0..32 {
        for first in 0..40 {
            for last in first..40 {
                let mut gate = SequenceGate::new(token());
                let action = gate.snapshot(token(), previous).unwrap();
                committed(&mut gate, action);
                let decision = gate.update(token(), first, last);
                if last <= previous {
                    assert_eq!(decision, Ok(Decision::IgnoreDuplicate));
                } else if first > previous + 1 {
                    assert_eq!(decision, Err(SyncError::Gap));
                    assert_eq!(gate.state(), SequenceState::Invalid);
                } else {
                    committed(&mut gate, decision.unwrap());
                    assert_eq!(
                        gate.state(),
                        SequenceState::Following {
                            last_update_id: last
                        }
                    );
                }
            }
        }
    }
}

#[test]
fn request_construction_is_bounded_and_matches_response_id() {
    let mut output = Vec::new();
    SnapshotRequest::new("snap1", "BTCUSDT", 5000)
        .unwrap()
        .encode(&mut output)
        .unwrap();
    assert_eq!(
        std::str::from_utf8(&output).unwrap(),
        r#"{"id":"snap1","method":"depth","params":{"symbol":"BTCUSDT","limit":5000}}"#
    );
    for limit in [0, 5001, u16::MAX] {
        assert!(SnapshotRequest::new("snap1", "BTCUSDT", limit).is_err());
    }
    assert!(SnapshotRequest::new("", "BTCUSDT", 5).is_err());
    assert!(SnapshotRequest::new("snap1", "BTC\nUSDT", 5).is_err());
    assert!(SnapshotRequest::new("snap1", "", 5).is_err());
}

#[test]
fn batch_scales_follow_each_exact_normalization_target() {
    let mut decoder = Decoder::new(limits()).unwrap();
    let alternate = Scales {
        price: Scale::new(3).unwrap(),
        quantity: Scale::new(4).unwrap(),
    };
    let update = decoder.decode_depth(DEPTH, "BTCUSDT", alternate).unwrap();
    assert_eq!(update.scales, alternate);
    assert_eq!(update.bids[0].price.units(), 65_000_000);
    assert_eq!(update.bids[0].quantity.units(), 12_500);
    assert_eq!(update.bids[1].quantity.units(), 0);
    let response = decoder
        .decode_snapshot(SNAPSHOT, "snap1", alternate)
        .unwrap();
    let snapshot::SnapshotResult::Depth(snapshot) = response.result else {
        panic!("expected depth")
    };
    assert_eq!(snapshot.scales, alternate);
    assert_eq!(snapshot.bids[0].price.units(), 65_000_000);
    assert_eq!(snapshot.bids[0].quantity.units(), 15_000);
    let update = decoder.decode_depth(DEPTH, "BTCUSDT", scales()).unwrap();
    assert_eq!(update.scales, scales());
    assert_eq!(update.bids[0].price.units(), 6_500_000);
    assert_eq!(update.bids[0].quantity.units(), 1_250);
}

#[test]
fn total_level_limit_handles_all_side_splits_and_reuse() {
    // Independently encode the two repeating groups after the fixture's fixed
    // header/body. Each side uses distinct price/quantity ranges.
    fn message(bid_count: u16, ask_count: u16) -> Vec<u8> {
        let mut bytes = DEPTH[..34].to_vec();
        for (count, base_price, base_quantity) in
            [(bid_count, 1000_i64, 100_i64), (ask_count, 2000, 200)]
        {
            bytes.extend_from_slice(&16_u16.to_le_bytes());
            bytes.extend_from_slice(&count.to_le_bytes());
            for i in 0..i64::from(count) {
                bytes.extend_from_slice(&(base_price + i).to_le_bytes());
                bytes.extend_from_slice(&(base_quantity + i).to_le_bytes());
            }
        }
        bytes.push(7);
        bytes.extend_from_slice(b"BTCUSDT");
        bytes
    }

    let mut bounded = limits();
    bounded.max_levels = 4;
    let mut decoder = Decoder::new(bounded).unwrap();
    // Includes each side occupying the entire total bound and both sides empty.
    for (bids, asks) in [(4, 0), (0, 4), (3, 1), (1, 3), (2, 2), (0, 0), (4, 0)] {
        let bytes = message(bids, asks);
        let d = decoder.decode_depth(&bytes, "BTCUSDT", scales()).unwrap();
        assert_eq!((d.bids.len(), d.asks.len()), (bids as usize, asks as usize));
        for (rows, base_price, base_quantity) in
            [(d.bids, 1000_u128, 100_u128), (d.asks, 2000, 200)]
        {
            for (i, level) in rows.iter().enumerate() {
                assert_eq!(level.price.units(), base_price + i as u128);
                assert_eq!(level.quantity.units(), base_quantity + i as u128);
            }
        }
    }
    for bytes in [message(4, 1), message(0, 5), message(5, 0)] {
        assert_eq!(
            decoder
                .decode_depth(&bytes, "BTCUSDT", scales())
                .unwrap_err(),
            DecodeError::LimitExceeded
        );
        let valid = message(0, 4);
        let d = decoder.decode_depth(&valid, "BTCUSDT", scales()).unwrap();
        assert!(d.bids.is_empty());
        assert_eq!(d.asks[0].price.units(), 2000);
    }
    // Fail after both groups have been decoded; reuse must reset the split.
    let mut bad_symbol = message(3, 1);
    *bad_symbol.last_mut().unwrap() = b'X';
    assert_eq!(
        decoder
            .decode_depth(&bad_symbol, "BTCUSDT", scales())
            .unwrap_err(),
        DecodeError::InvalidField
    );
    let valid = message(4, 0);
    let d = decoder.decode_depth(&valid, "BTCUSDT", scales()).unwrap();
    assert_eq!(d.bids.len(), 4);
    assert!(d.asks.is_empty());

    bounded.max_levels = 0;
    let mut empty_decoder = Decoder::new(bounded).unwrap();
    let empty = message(0, 0);
    let d = empty_decoder
        .decode_depth(&empty, "BTCUSDT", scales())
        .unwrap();
    assert!(d.bids.is_empty() && d.asks.is_empty());
    assert_eq!(
        empty_decoder
            .decode_depth(&valid, "BTCUSDT", scales())
            .unwrap_err(),
        DecodeError::LimitExceeded
    );
}

#[test]
fn generated_api_enums_fail_closed_and_rate_views_are_repeatable() {
    let mut decoder = Decoder::new(limits()).unwrap();
    for value in 0..=255u8 {
        for offset in [8, 15, 16] {
            let mut bytes = SNAPSHOT.to_vec();
            bytes[offset] = value;
            let response = decoder.decode_snapshot(&bytes, "snap1", scales());
            let valid = if offset == 8 { value <= 1 } else { value <= 3 };
            assert_eq!(response.is_ok(), valid, "offset={offset}, value={value}");
            if let Ok(response) = response {
                let first: Vec<_> = response.rate_limits.iter().collect();
                assert_eq!(first, response.rate_limits.iter().collect::<Vec<_>>());
                assert_eq!(first.len(), 1);
                assert_eq!(first[0].limit, 6000);
                assert_eq!(first[0].count, 250);
                if offset == 8 {
                    assert_eq!(response.schema_deprecated, value == 1);
                }
                if offset == 15 {
                    assert_eq!(first[0].kind, value);
                }
                if offset == 16 {
                    assert_eq!(first[0].interval, value);
                }
            }
        }
    }
    let mut bytes = SNAPSHOT.to_vec();
    bytes[17] = 0; // intervalNum must remain positive.
    assert!(decoder.decode_snapshot(&bytes, "snap1", scales()).is_err());
    assert!(decoder.decode_snapshot(SNAPSHOT, "snap1", scales()).is_ok());
}

#[test]
fn generated_api_optional_times_and_unsupported_nested_data() {
    let mut decoder = Decoder::new(limits()).unwrap();
    // Independent fixture layout: wrapper result starts at 44; error header is 8
    // bytes, then int16 code, int64 serverTime, int64 retryAfter.
    for offset in [54, 62] {
        for value in [i64::MIN, -1, 0, i64::MAX] {
            let mut bytes = RATE_LIMIT.to_vec();
            bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
            let result = decoder.decode_snapshot(&bytes, "snap1", scales());
            if value == -1 {
                assert!(result.is_err());
            } else {
                let response = result.unwrap();
                let snapshot::SnapshotResult::Error(error) = response.result else {
                    panic!()
                };
                let actual = if offset == 54 {
                    error.server_time_us
                } else {
                    error.retry_after_us
                };
                assert_eq!(actual, if value == i64::MIN { None } else { Some(value) });
            }
        }
    }
    let mut bytes = RATE_LIMIT.to_vec();
    bytes[72] = 0xff; // Invalid UTF-8 in the error message.
    assert_eq!(
        decoder
            .decode_snapshot(&bytes, "snap1", scales())
            .unwrap_err(),
        DecodeError::InvalidText
    );
    let mut bytes = RATE_LIMIT.to_vec();
    let data_length_offset = bytes.len() - 4;
    bytes[data_length_offset..].copy_from_slice(&1u32.to_le_bytes());
    assert_eq!(
        decoder
            .decode_snapshot(&bytes, "snap1", scales())
            .unwrap_err(),
        DecodeError::UnsupportedLayout
    );
    assert!(
        decoder
            .decode_snapshot(RATE_LIMIT, "snap1", scales())
            .is_ok()
    );
}
