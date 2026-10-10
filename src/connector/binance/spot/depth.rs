//! Spot SBE stream schema 1:0, template 10003 (Diff Depth).
//!
//! Official [Diff. Depth Streams](https://github.com/binance/binance-spot-api-docs/blob/master/sbe-market-data-streams.md#diff-depth-streams):
//! `<symbol>@depth`, message `DepthDiffStreamEvent`.
//! [Wire schema 1:0](https://github.com/binance/binance-spot-api-docs/blob/master/sbe/schemas/stream_1_0.xml):
//! template 10003, `eventTime` in UTC microseconds, `firstBookUpdateId`,
//! `lastBookUpdateId`, price/quantity exponents, bid/ask groups, then symbol.

use super::sbe::{DecodeError, DecodeLimits, LayoutCursor, LevelBuffer, Scales, update_id};
use crate::market_data::{Level, ScaleConversion};
use spot_stream::{
    ReadBuf, SBE_SCHEMA_ID, SBE_SCHEMA_VERSION,
    depth_diff_stream_event_codec::{
        SBE_BLOCK_LENGTH, SBE_TEMPLATE_ID, decoder::DepthDiffStreamEventDecoder,
    },
    message_header_codec::ENCODED_LENGTH,
};

#[derive(Debug)]
pub struct DepthUpdate<'a> {
    /// Internal normalization scales for all bids and asks in this batch.
    /// Validate against the instrument context before book application.
    pub scales: Scales,
    pub symbol: &'a str,
    pub event_time_us: i64,
    pub first_update_id: i64,
    pub last_update_id: i64,
    pub bids: &'a [Level],
    pub asks: &'a [Level],
}

/// Decode a complete application message. `expected_symbol` uses the wire payload
/// case, not the lowercase subscription URL. Resolve it during setup.
pub(crate) fn decode<'a>(
    input: &'a [u8],
    expected_symbol: &str,
    scales: Scales,
    limits: DecodeLimits,
    storage: &'a mut LevelBuffer,
) -> Result<DepthUpdate<'a>, DecodeError> {
    storage.clear();
    let result = (|| {
        limits.check_message(input)?;
        // Generated accessors use indexing: prove the entire layout is in bounds
        // before calling any of them. This checks framing, not level values.
        validate_layout(input, limits)?;
        let message = DepthDiffStreamEventDecoder::default().wrap(
            ReadBuf::new(input),
            ENCODED_LENGTH,
            SBE_BLOCK_LENGTH,
            SBE_SCHEMA_VERSION,
        );
        let event_time_us = update_id(message.event_time())?;
        let first_update_id = update_id(message.first_book_update_id())?;
        let last_update_id = update_id(message.last_book_update_id())?;
        if first_update_id > last_update_id {
            return Err(DecodeError::InvalidField);
        }
        let conversions = (
            scales.price.prepare(message.price_exponent())?,
            scales.quantity.prepare(message.qty_exponent())?,
        );
        let (offset, length) = decode_levels_and_symbol(message, conversions, storage)?;
        let symbol = std::str::from_utf8(&input[offset..offset + length])
            .map_err(|_| DecodeError::InvalidText)?;
        if symbol != expected_symbol {
            return Err(DecodeError::InvalidField);
        }
        Ok((symbol, event_time_us, first_update_id, last_update_id))
    })();
    match result {
        Ok((symbol, event_time_us, first_update_id, last_update_id)) => {
            let (bids, asks) = storage.sides();
            Ok(DepthUpdate {
                scales,
                symbol,
                event_time_us,
                first_update_id,
                last_update_id,
                bids,
                asks,
            })
        }
        Err(error) => {
            storage.clear();
            Err(error)
        }
    }
}

/// Decode the validated variable-length fields in wire order.
fn decode_levels_and_symbol(
    message: DepthDiffStreamEventDecoder<'_>,
    conversions: (ScaleConversion, ScaleConversion),
    storage: &mut LevelBuffer,
) -> Result<(usize, usize), DecodeError> {
    let mut bids = message.bids_decoder();
    while bids
        .advance()
        .map_err(|_| DecodeError::InvalidField)?
        .is_some()
    {
        storage.push_level(bids.price(), bids.qty(), conversions)?;
    }
    storage.finish_bids();
    let mut asks = bids
        .parent()
        .map_err(|_| DecodeError::InvalidField)?
        .asks_decoder();
    while asks
        .advance()
        .map_err(|_| DecodeError::InvalidField)?
        .is_some()
    {
        storage.push_level(asks.price(), asks.qty(), conversions)?;
    }
    let mut message = asks.parent().map_err(|_| DecodeError::InvalidField)?;
    Ok(message.symbol_decoder())
}

/// The generated codec assumes valid lengths and does not validate group layouts.
/// Check both groups without scanning/copying their entries; offsets cannot escape
/// the bounded input. Do not use generated decoders directly on network bytes.
fn validate_layout(input: &[u8], limits: DecodeLimits) -> Result<(), DecodeError> {
    let mut reader = LayoutCursor::new(input);
    reader.header(
        SBE_SCHEMA_ID,
        SBE_SCHEMA_VERSION,
        SBE_TEMPLATE_ID,
        SBE_BLOCK_LENGTH,
    )?;
    reader.bytes(usize::from(SBE_BLOCK_LENGTH))?;
    let mut total = 0usize;
    for _ in 0..2 {
        if reader.u16()? != 16 {
            return Err(DecodeError::UnsupportedLayout);
        }
        let count = usize::from(reader.u16()?);
        total = total.checked_add(count).ok_or(DecodeError::LimitExceeded)?;
        if total > limits.max_levels {
            return Err(DecodeError::LimitExceeded);
        }
        reader.bytes(count.checked_mul(16).ok_or(DecodeError::LimitExceeded)?)?;
    }
    let length = usize::from(reader.u8()?);
    if length == 0 || length > limits.max_symbol_bytes {
        return Err(DecodeError::LimitExceeded);
    }
    reader.bytes(length)?;
    reader.finish()
}
