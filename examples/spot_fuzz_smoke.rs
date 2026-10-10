//! Deterministic mutation/random smoke fuzzer, not a coverage-guided campaign.
use strait::connector::binance::spot::sbe::{DecodeLimits, Decoder, Scales};
use strait::market_data::Scale;

fn random(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

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
    let fixtures: [&[u8]; 3] = [
        include_bytes!("../tests/fixtures/binance/spot/depth.bin"),
        include_bytes!("../tests/fixtures/binance/spot/snapshot.bin"),
        include_bytes!("../tests/fixtures/binance/spot/rate-limit.bin"),
    ];
    let mut seed = 0x5354_5241_4954_u64;
    let mut cases = 0;
    // Every possible byte substitution in each fixed valid/error fixture.
    for fixture in fixtures {
        let mut bytes = fixture.to_vec();
        for offset in 0..bytes.len() {
            let original = bytes[offset];
            for value in 0..=u8::MAX {
                bytes[offset] = value;
                let _ = storage.decode_depth(&bytes, "BTCUSDT", scales);
                let _ = storage.decode_snapshot(&bytes, "snap1", scales);
                cases += 1;
            }
            bytes[offset] = original;
        }
    }
    let mut bytes = vec![0; 8192];
    for _ in 0..20_000 {
        let length = (random(&mut seed) as usize) % bytes.len();
        for byte in &mut bytes[..length] {
            *byte = random(&mut seed) as u8;
        }
        let _ = storage.decode_depth(&bytes[..length], "BTCUSDT", scales);
        let _ = storage.decode_snapshot(&bytes[..length], "snap1", scales);
        let _ = scales
            .price
            .convert(random(&mut seed) as i64, random(&mut seed) as i8);
        cases += 1;
    }
    // Verify successful reuse after malformed storms.
    storage.decode_depth(fixtures[0], "BTCUSDT", scales)?;
    storage.decode_snapshot(fixtures[1], "snap1", scales)?;
    println!("{cases} cases passed; initial seed=0x535452414954; no input-driven panic");
    Ok(())
}
