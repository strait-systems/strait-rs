//! Bounded official generated stream and API codecs with exact domain conversion.
//! No unsafe access or speculative parser optimization.
//!
//! Official wire definitions: [stream schema 1:0](https://github.com/binance/binance-spot-api-docs/blob/master/sbe/schemas/stream_1_0.xml)
//! and [API schema 3:5](https://github.com/binance/binance-spot-api-docs/blob/master/sbe/schemas/spot_3_5.xml).
//! See `messageHeader`, `groupSize16Encoding`, `groupSizeEncoding`,
//! `mantissa64`, `exponent8`, and the variable-length data composites.
//! [SBE configuration and compatibility](https://github.com/binance/binance-spot-api-docs/blob/master/faqs/sbe_faq.md).

use crate::market_data::{Level, NumericError, Price, Quantity, ScaleConversion};

// Preserve the existing connector import path; scales are shared domain metadata.
pub use crate::market_data::Scales;

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum DecodeError {
    #[error("truncated SBE message")]
    Truncated,
    /// The SBE `messageHeader` or a repeating-group block length differs from
    /// the explicitly supported layouts linked in this module documentation.
    /// Exact-layout rejection is this decoder's policy, not a Binance error code.
    #[error("unsupported schema, version, template or block length")]
    UnsupportedLayout,
    #[error("input exceeds configured limits")]
    LimitExceeded,
    #[error("invalid required field")]
    InvalidField,
    #[error("invalid UTF-8")]
    InvalidText,
    #[error("unexpected trailing data")]
    TrailingData,
    #[error(transparent)]
    Numeric(#[from] NumericError),
}

/// Bounds apply before retaining or decoding levels. No production defaults.
#[derive(Clone, Copy, Debug)]
pub struct DecodeLimits {
    pub max_message_bytes: usize,
    /// Total bids plus asks, not a per-side limit.
    pub max_levels: usize,
    pub max_symbol_bytes: usize,
    pub max_rate_limits: usize,
}
impl DecodeLimits {
    pub(crate) fn check_message(self, input: &[u8]) -> Result<(), DecodeError> {
        if input.len() > self.max_message_bytes {
            return Err(DecodeError::LimitExceeded);
        }
        Ok(())
    }
}

/// Caller-owned SBE decoder with fixed limits and reusable level storage.
/// Returned views borrow both the input and decoder, preventing premature reuse.
#[derive(Debug)]
pub struct Decoder {
    limits: DecodeLimits,
    levels: LevelBuffer,
}
impl Decoder {
    pub fn new(limits: DecodeLimits) -> Result<Self, DecodeError> {
        let mut levels = Vec::new();
        levels
            .try_reserve_exact(limits.max_levels)
            .map_err(|_| DecodeError::LimitExceeded)?;
        Ok(Self {
            limits,
            levels: LevelBuffer {
                levels,
                bid_count: 0,
            },
        })
    }
    pub fn limits(&self) -> DecodeLimits {
        self.limits
    }
    /// Decode a complete Diff Depth message for the setup-resolved wire symbol.
    pub fn decode_depth<'a>(
        &'a mut self,
        input: &'a [u8],
        expected_symbol: &str,
        scales: Scales,
    ) -> Result<super::depth::DepthUpdate<'a>, DecodeError> {
        super::depth::decode(
            input,
            expected_symbol,
            scales,
            self.limits,
            &mut self.levels,
        )
    }
    /// Decode a WebSocket API response correlated with the pending snapshot request.
    pub fn decode_snapshot<'a>(
        &'a mut self,
        input: &'a [u8],
        expected_request_id: &str,
        scales: Scales,
    ) -> Result<super::snapshot::SnapshotResponse<'a>, DecodeError> {
        super::snapshot::decode(
            input,
            expected_request_id,
            scales,
            self.limits,
            &mut self.levels,
        )
    }
}

#[derive(Debug)]
pub(crate) struct LevelBuffer {
    // Append bids first, then asks; never insert ahead of existing asks.
    levels: Vec<Level>,
    bid_count: usize,
}
impl LevelBuffer {
    pub(crate) fn clear(&mut self) {
        self.levels.clear();
        self.bid_count = 0;
    }
    pub(crate) fn sides(&self) -> (&[Level], &[Level]) {
        // Private invariant: clear resets the split; successful bid decoding
        // sets it to len before asks append. Failed decoding clears both.
        self.levels.split_at(self.bid_count)
    }
    /// Finish appending bids before any asks are appended.
    pub(crate) fn finish_bids(&mut self) {
        self.bid_count = self.levels.len();
    }

    /// Append one level after the caller validates layout/count bounds.
    #[inline]
    pub(crate) fn push_level(
        &mut self,
        price: i64,
        quantity: i64,
        conversions: (ScaleConversion, ScaleConversion),
    ) -> Result<(), DecodeError> {
        self.levels.push(Level {
            price: Price::from_prepared(price, conversions.0)?,
            quantity: Quantity::from_prepared(quantity, conversions.1)?,
        });
        Ok(())
    }
}

/// Bounds/layout validation around generated accessors, not a field decoder.
/// All scalar reads below concern framing and delegate byte interpretation to
/// the generated crate only after the corresponding span has been checked.
pub(crate) struct LayoutCursor<'a> {
    input: &'a [u8],
    offset: usize,
}
impl<'a> LayoutCursor<'a> {
    pub(crate) fn new(input: &'a [u8]) -> Self {
        Self { input, offset: 0 }
    }
    pub(crate) fn bytes(&mut self, len: usize) -> Result<&'a [u8], DecodeError> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or(DecodeError::LimitExceeded)?;
        let value = self
            .input
            .get(self.offset..end)
            .ok_or(DecodeError::Truncated)?;
        self.offset = end;
        Ok(value)
    }
    pub(crate) fn u8(&mut self) -> Result<u8, DecodeError> {
        let bytes = self.bytes(1)?;
        Ok(spot_stream::ReadBuf::new(bytes).get_u8_at(0))
    }
    pub(crate) fn u16(&mut self) -> Result<u16, DecodeError> {
        let bytes = self.bytes(2)?;
        Ok(spot_stream::ReadBuf::new(bytes).get_u16_at(0))
    }
    pub(crate) fn u32(&mut self) -> Result<u32, DecodeError> {
        let bytes = self.bytes(4)?;
        Ok(spot_stream::ReadBuf::new(bytes).get_u32_at(0))
    }
    pub(crate) fn finish(&self) -> Result<(), DecodeError> {
        if self.offset != self.input.len() {
            return Err(DecodeError::TrailingData);
        }
        Ok(())
    }
    pub(crate) fn header(
        &mut self,
        schema: u16,
        version: u16,
        template: u16,
        block: u16,
    ) -> Result<(), DecodeError> {
        use spot_stream::message_header_codec::{ENCODED_LENGTH, decoder::MessageHeaderDecoder};
        let bytes = self.bytes(ENCODED_LENGTH)?;
        let header = MessageHeaderDecoder::default().wrap(spot_stream::ReadBuf::new(bytes), 0);
        if (
            header.block_length(),
            header.template_id(),
            header.schema_id(),
            header.version(),
        ) != (block, template, schema, version)
        {
            return Err(DecodeError::UnsupportedLayout);
        }
        Ok(())
    }
}

pub(crate) fn update_id(value: i64) -> Result<i64, DecodeError> {
    if value < 0 {
        return Err(DecodeError::InvalidField);
    }
    Ok(value)
}
