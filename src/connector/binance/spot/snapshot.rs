//! WebSocket API schema 3:5: wrapper 50 containing DepthResponse 200 or ErrorResponse 100.
//! This module decodes complete responses; it does not own a network session.
//!
//! Official [order book request](https://github.com/binance/binance-spot-api-docs/blob/master/web-socket-api.md#order-book) (`depth`)
//! and [rate limits](https://github.com/binance/binance-spot-api-docs/blob/master/web-socket-api.md#rate-limits).
//! [Wire schema 3:5](https://github.com/binance/binance-spot-api-docs/blob/master/sbe/schemas/spot_3_5.xml):
//! `WebSocketResponse` (50), `DepthResponse` (200), `ErrorResponse` (100),
//! `rateLimitType`, `rateLimitInterval`, and optional UTC microsecond timestamps.

use super::sbe::{DecodeError, DecodeLimits, LayoutCursor, LevelBuffer, Scales, update_id};
use crate::market_data::{Level, ScaleConversion};
use spot_sbe::{
    ReadBuf, SBE_SCHEMA_ID, SBE_SCHEMA_VERSION,
    bool_enum::BoolEnum,
    depth_response_codec::{self, decoder::DepthResponseDecoder},
    error_response_codec::{self, decoder::ErrorResponseDecoder},
    message_header_codec::ENCODED_LENGTH,
    web_socket_response_codec::{self, decoder::WebSocketResponseDecoder},
};

/// Server limits remain visible to the future admission manager.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RateLimit {
    /// 0 raw requests, 1 connections, 2 request weight, 3 orders.
    pub kind: u8,
    /// 0 seconds, 1 minutes, 2 hours, 3 days.
    pub interval: u8,
    pub interval_num: u8,
    pub limit: i64,
    pub count: i64,
}

#[derive(Clone, Copy, Debug)]
pub struct RateLimits<'a> {
    // Constructed only after the complete wrapper and all rate entries validate.
    input: &'a [u8],
}
impl<'a> RateLimits<'a> {
    pub fn iter(self) -> impl Iterator<Item = RateLimit> + 'a {
        let mut group = wrapper(self.input).rate_limits_decoder();
        std::iter::from_fn(move || match group.advance() {
            Ok(Some(_)) => Some(RateLimit {
                kind: group.rate_limit_type().into(),
                interval: group.interval().into(),
                interval_num: group.interval_num(),
                limit: group.rate_limit(),
                count: group.current(),
            }),
            Ok(None) => None,
            // Internal lifecycle invariant: this iterator never takes the parent
            // out of its private initialized group. Network bytes cannot do so.
            Err(_) => unreachable!("initialized rate-limit group lost its parent"),
        })
    }
}

#[derive(Debug)]
pub struct Snapshot<'a> {
    /// Internal normalization scales for all bids and asks in this batch.
    /// Validate against the instrument context before book application.
    pub scales: Scales,
    pub last_update_id: i64,
    pub bids: &'a [Level],
    pub asks: &'a [Level],
}

#[derive(Debug)]
pub struct ApiError<'a> {
    pub code: i16,
    pub message: &'a str,
    pub server_time_us: Option<i64>,
    /// Absolute server UTC microseconds, not a local monotonic deadline or duration.
    pub retry_after_us: Option<i64>,
}

#[derive(Debug)]
pub enum SnapshotResult<'a> {
    Depth(Snapshot<'a>),
    Error(ApiError<'a>),
}

#[derive(Debug)]
pub struct SnapshotResponse<'a> {
    pub request_id: &'a str,
    pub status: u16,
    pub schema_deprecated: bool,
    pub rate_limits: RateLimits<'a>,
    pub result: SnapshotResult<'a>,
}

/// Correlate with the pending request before accepting levels; the payload has no symbol.
/// The future session must also bind that request to an instrument and cycle token.
pub(crate) fn decode<'a>(
    input: &'a [u8],
    expected_request_id: &str,
    scales: Scales,
    limits: DecodeLimits,
    storage: &'a mut LevelBuffer,
) -> Result<SnapshotResponse<'a>, DecodeError> {
    storage.clear();
    let result = (|| {
        limits.check_message(input)?;
        validate_wrapper(input, limits)?;
        let message = wrapper(input);
        let schema_deprecated = match message.sbe_schema_id_version_deprecated() {
            BoolEnum::False => false,
            BoolEnum::True => true,
            _ => return Err(DecodeError::InvalidField),
        };
        let status = message.status();
        let mut rates = message.rate_limits_decoder();
        while rates
            .advance()
            .map_err(|_| DecodeError::InvalidField)?
            .is_some()
        {
            let kind: u8 = rates.rate_limit_type().into();
            let interval: u8 = rates.interval().into();
            if kind > 3
                || interval > 3
                || rates.interval_num() == 0
                || rates.rate_limit() < 0
                || rates.current() < 0
            {
                return Err(DecodeError::InvalidField);
            }
        }
        let rate_limits = RateLimits { input };
        let mut message = rates.parent().map_err(|_| DecodeError::InvalidField)?;
        let request_id = text(input, message.id_decoder())?;
        if expected_request_id.is_empty() || request_id != expected_request_id {
            return Err(DecodeError::InvalidField);
        }
        let (offset, length) = message.result_decoder();
        let payload = &input[offset..offset + length];
        if status == 200 {
            validate_depth(payload, limits)?;
            let message = DepthResponseDecoder::default().wrap(
                ReadBuf::new(payload),
                ENCODED_LENGTH,
                depth_response_codec::SBE_BLOCK_LENGTH,
                SBE_SCHEMA_VERSION,
            );
            let last_update_id = update_id(message.last_update_id())?;
            let conversions = (
                scales.price.prepare(message.price_exponent())?,
                scales.quantity.prepare(message.qty_exponent())?,
            );
            decode_snapshot_levels(storage, message, conversions)?;
            Ok((
                request_id,
                status,
                schema_deprecated,
                rate_limits,
                Ok(last_update_id),
            ))
        } else {
            if !(400..600).contains(&status) {
                return Err(DecodeError::InvalidField);
            }
            // Fixed fields are covered by the checked root. Before the generated
            // message-length accessor, prove its uint16 prefix is present.
            let mut layout = root(
                payload,
                error_response_codec::SBE_TEMPLATE_ID,
                error_response_codec::SBE_BLOCK_LENGTH,
            )?;
            layout.bytes(2)?;
            let mut message = ErrorResponseDecoder::default().wrap(
                ReadBuf::new(payload),
                ENCODED_LENGTH,
                error_response_codec::SBE_BLOCK_LENGTH,
                SBE_SCHEMA_VERSION,
            );
            let code = message.code();
            if code >= 0 || code == i16::MIN {
                return Err(DecodeError::InvalidField);
            }
            let server_time_us = optional_time(message.server_time())?;
            let retry_after_us = optional_time(message.retry_after())?;
            let (_, message_length) = message.msg_decoder();
            let error_text = std::str::from_utf8(layout.bytes(message_length)?)
                .map_err(|_| DecodeError::InvalidText)?;
            // Depth errors do not contain cancel-replace data.
            if layout.u32()? != 0 {
                return Err(DecodeError::UnsupportedLayout);
            }
            layout.finish()?;
            Ok((
                request_id,
                status,
                schema_deprecated,
                rate_limits,
                Err(ApiError {
                    code,
                    message: error_text,
                    server_time_us,
                    retry_after_us,
                }),
            ))
        }
    })();
    match result {
        Ok((request_id, status, schema_deprecated, rate_limits, result)) => {
            let result = match result {
                Ok(last_update_id) => {
                    let (bids, asks) = storage.sides();
                    SnapshotResult::Depth(Snapshot {
                        scales,
                        last_update_id,
                        bids,
                        asks,
                    })
                }
                Err(error) => SnapshotResult::Error(error),
            };
            Ok(SnapshotResponse {
                request_id,
                status,
                schema_deprecated,
                rate_limits,
                result,
            })
        }
        Err(error) => {
            storage.clear();
            Err(error)
        }
    }
}

/// API depth layout and combined count were checked before wrapping the codec.
fn decode_snapshot_levels(
    storage: &mut LevelBuffer,
    message: DepthResponseDecoder<'_>,
    conversions: (ScaleConversion, ScaleConversion),
) -> Result<(), DecodeError> {
    let mut bids = message.bids_decoder();
    append_snapshot_group(storage, conversions, || {
        bids.advance()
            .map_err(|_| DecodeError::InvalidField)
            .map(|index| index.map(|_| (bids.price(), bids.qty())))
    })?;
    storage.finish_bids();
    let mut asks = bids
        .parent()
        .map_err(|_| DecodeError::InvalidField)?
        .asks_decoder();
    append_snapshot_group(storage, conversions, || {
        asks.advance()
            .map_err(|_| DecodeError::InvalidField)
            .map(|index| index.map(|_| (asks.price(), asks.qty())))
    })?;
    validate_snapshot_order(storage)
}

// Keep whole-group normalization out of the caller's two generated decoder
// loops. Both monomorphizations have equivalent generated group layouts.
#[inline(never)]
fn append_snapshot_group(
    storage: &mut LevelBuffer,
    conversions: (ScaleConversion, ScaleConversion),
    mut next: impl FnMut() -> Result<Option<(i64, i64)>, DecodeError>,
) -> Result<(), DecodeError> {
    while let Some((price, quantity)) = next()? {
        // Preserve numeric-error precedence. The enclosing decode clears storage
        // on every failure before exposing a view.
        storage.push_level(price, quantity, conversions)?;
        if quantity == 0 {
            return Err(DecodeError::InvalidField);
        }
    }
    Ok(())
}

/// Validate strict price ordering and reject duplicate snapshot prices.
fn validate_snapshot_order(storage: &LevelBuffer) -> Result<(), DecodeError> {
    let (bids, asks) = storage.sides();
    if bids
        .windows(2)
        .any(|w| w[0].price.units() <= w[1].price.units())
        || asks
            .windows(2)
            .any(|w| w[0].price.units() >= w[1].price.units())
    {
        return Err(DecodeError::InvalidField);
    }
    Ok(())
}

fn optional_time(value: Option<i64>) -> Result<Option<i64>, DecodeError> {
    value.map(update_id).transpose()
}

#[inline]
fn text(input: &[u8], (offset, length): (usize, usize)) -> Result<&str, DecodeError> {
    // All generated coordinates are covered by the matching framing preflight.
    std::str::from_utf8(&input[offset..offset + length]).map_err(|_| DecodeError::InvalidText)
}

fn wrapper(input: &[u8]) -> WebSocketResponseDecoder<'_> {
    WebSocketResponseDecoder::default().wrap(
        ReadBuf::new(input),
        ENCODED_LENGTH,
        web_socket_response_codec::SBE_BLOCK_LENGTH,
        SBE_SCHEMA_VERSION,
    )
}

#[inline]
fn root(input: &[u8], template: u16, block: u16) -> Result<LayoutCursor<'_>, DecodeError> {
    let mut cursor = LayoutCursor::new(input);
    cursor.header(SBE_SCHEMA_ID, SBE_SCHEMA_VERSION, template, block)?;
    cursor.bytes(usize::from(block))?;
    Ok(cursor)
}

/// Generated accessors assume valid lengths. Check dimensions/spans first,
/// without interpreting domain fields or copying group entries.
fn validate_wrapper(input: &[u8], limits: DecodeLimits) -> Result<(), DecodeError> {
    let mut cursor = root(
        input,
        web_socket_response_codec::SBE_TEMPLATE_ID,
        web_socket_response_codec::SBE_BLOCK_LENGTH,
    )?;
    if cursor.u16()? != 19 {
        return Err(DecodeError::UnsupportedLayout);
    }
    let count = usize::from(cursor.u16()?);
    if count > limits.max_rate_limits {
        return Err(DecodeError::LimitExceeded);
    }
    cursor.bytes(count.checked_mul(19).ok_or(DecodeError::LimitExceeded)?)?;
    let id_len = usize::from(cursor.u8()?);
    cursor.bytes(id_len)?;
    let length = usize::try_from(cursor.u32()?).map_err(|_| DecodeError::LimitExceeded)?;
    if length > i32::MAX as usize {
        return Err(DecodeError::LimitExceeded);
    }
    cursor.bytes(length)?;
    cursor.finish()
}

fn validate_depth(input: &[u8], limits: DecodeLimits) -> Result<(), DecodeError> {
    let mut cursor = root(
        input,
        depth_response_codec::SBE_TEMPLATE_ID,
        depth_response_codec::SBE_BLOCK_LENGTH,
    )?;
    let mut total = 0usize;
    for _ in 0..2 {
        if cursor.u16()? != 16 {
            return Err(DecodeError::UnsupportedLayout);
        }
        let count = usize::try_from(cursor.u32()?).map_err(|_| DecodeError::LimitExceeded)?;
        total = total.checked_add(count).ok_or(DecodeError::LimitExceeded)?;
        if total > limits.max_levels {
            return Err(DecodeError::LimitExceeded);
        }
        cursor.bytes(count.checked_mul(16).ok_or(DecodeError::LimitExceeded)?)?;
    }
    cursor.finish()
}
