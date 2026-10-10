//! Exact numeric primitives. Wire sequence and recovery state remain in connectors.

/// Decimal places in the internal integer representation (0 through 18).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Scale(u8);

impl Scale {
    pub fn new(decimal_places: u8) -> Result<Self, NumericError> {
        if decimal_places > 18 {
            return Err(NumericError::UnsupportedScale);
        }
        Ok(Self(decimal_places))
    }
    pub fn decimal_places(self) -> u8 {
        self.0
    }
    /// Convert `mantissa * 10^exponent` exactly; never round or wrap.
    /// Binance defines `mantissa64` and its preceding `exponent8` in the
    /// [Spot stream schema](https://github.com/binance/binance-spot-api-docs/blob/master/sbe/schemas/stream_1_0.xml)
    /// and [Spot API schema](https://github.com/binance/binance-spot-api-docs/blob/master/sbe/schemas/spot_3_5.xml).
    /// Returns nonnegative internal units in `0..=u128::MAX`. The power-of-ten
    /// factor must also fit in `u128`, even for zero values.
    pub fn convert(self, mantissa: i64, exponent: i8) -> Result<u128, NumericError> {
        if mantissa < 0 || exponent == i8::MIN {
            return Err(NumericError::InvalidValue);
        }
        self.prepare(exponent)?.convert(mantissa)
    }

    /// Prepare once for a batch sharing one wire exponent and target scale.
    pub(crate) fn prepare(self, exponent: i8) -> Result<ScaleConversion, NumericError> {
        if exponent == i8::MIN {
            return Err(NumericError::InvalidValue);
        }
        let shift = i16::from(exponent) + i16::from(self.0);
        // Even an empty batch must not mask an unsupported wire exponent.
        let factor = 10_u128
            .checked_pow(u32::from(shift.unsigned_abs()))
            .ok_or(NumericError::Overflow)?;
        Ok(ScaleConversion {
            factor,
            multiply: shift >= 0,
        })
    }
}

/// Checked conversion rule scoped to a batch's exponent and normalization target.
/// Contains no level storage and never changes the interpretation of existing values.
#[derive(Clone, Copy)]
pub(crate) struct ScaleConversion {
    factor: u128,
    multiply: bool,
}
impl ScaleConversion {
    fn convert(self, mantissa: i64) -> Result<u128, NumericError> {
        let value = u128::try_from(mantissa).map_err(|_| NumericError::InvalidValue)?;
        if self.multiply {
            value.checked_mul(self.factor).ok_or(NumericError::Overflow)
        } else if value % self.factor == 0 {
            Ok(value / self.factor)
        } else {
            Err(NumericError::PrecisionLoss)
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum NumericError {
    #[error("internal scale is unsupported")]
    UnsupportedScale,
    #[error("negative or null numeric value")]
    InvalidValue,
    #[error("numeric range exceeded")]
    Overflow,
    #[error("conversion would lose precision")]
    PrecisionLoss,
}

/// Positive scaled quote units per base unit (`1..=u128::MAX` internal units).
/// Scale lives in the enclosing instrument/batch context, not in each value.
/// Equality compares internal units and is meaningful only within the same scale.
/// Keep this value with its context when transporting or publishing it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Price {
    units: u128,
}
impl Price {
    pub fn from_sbe(mantissa: i64, exponent: i8, scale: Scale) -> Result<Self, NumericError> {
        Self::from_units(scale.convert(mantissa, exponent)?)
    }
    pub(crate) fn from_prepared(
        mantissa: i64,
        conversion: ScaleConversion,
    ) -> Result<Self, NumericError> {
        Self::from_units(conversion.convert(mantissa)?)
    }
    fn from_units(units: u128) -> Result<Self, NumericError> {
        if units == 0 {
            return Err(NumericError::InvalidValue);
        }
        Ok(Self { units })
    }
    pub fn units(self) -> u128 {
        self.units
    }
}

/// Nonnegative scaled quantity (`0..=u128::MAX` internal units).
/// Scale lives in the enclosing instrument/batch context. Equality compares units
/// only; do not compare or move values across contexts without exact conversion.
/// Spot interprets this in base-asset units;
/// derivatives must supply their own explicit quantity-unit context.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Quantity {
    units: u128,
}
impl Quantity {
    pub fn from_sbe(mantissa: i64, exponent: i8, scale: Scale) -> Result<Self, NumericError> {
        Ok(Self {
            units: scale.convert(mantissa, exponent)?,
        })
    }
    pub(crate) fn from_prepared(
        mantissa: i64,
        conversion: ScaleConversion,
    ) -> Result<Self, NumericError> {
        Ok(Self {
            units: conversion.convert(mantissa)?,
        })
    }
    pub fn units(self) -> u128 {
        self.units
    }
}

/// Fixed internal decimal scales shared by every level in an instrument/batch.
/// These are normalization targets, not per-message wire exponents. Consumers
/// must validate them against the instrument context before applying levels.
/// Changing them requires a controlled rebuild or exact conversion of old levels.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Scales {
    pub price: Scale,
    pub quantity: Scale,
}

/// Absolute quantity at a price. Zero quantity denotes deletion for an increment.
/// Both values use the enclosing batch/instrument `Scales`; retain that context.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Level {
    pub price: Price,
    pub quantity: Quantity,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Original one-value conversion, independent of the prepared rule.
    fn reference(mantissa: i64, exponent: i8, decimals: u8) -> Result<u128, NumericError> {
        if mantissa < 0 || exponent == i8::MIN {
            return Err(NumericError::InvalidValue);
        }
        let shift = i16::from(exponent) + i16::from(decimals);
        let factor = 10_u128
            .checked_pow(u32::from(shift.unsigned_abs()))
            .ok_or(NumericError::Overflow)?;
        let value = mantissa as u128;
        if shift >= 0 {
            value.checked_mul(factor).ok_or(NumericError::Overflow)
        } else if value.is_multiple_of(factor) {
            Ok(value / factor)
        } else {
            Err(NumericError::PrecisionLoss)
        }
    }

    #[test]
    fn prepared_conversion_matches_original_range_and_precision() {
        let values = [i64::MIN, -1, 0, 1, 9, 10, 100, 12345, i64::MAX];
        for decimals in 0..=18 {
            let scale = Scale::new(decimals).unwrap();
            for exponent in i8::MIN..=i8::MAX {
                for mantissa in values {
                    let expected = reference(mantissa, exponent, decimals);
                    assert_eq!(scale.convert(mantissa, exponent), expected);
                    // Decoder validates batch exponents before reading any mantissas.
                    if let Ok(rule) = scale.prepare(exponent) {
                        assert_eq!(rule.convert(mantissa), expected);
                        assert_eq!(
                            Price::from_prepared(mantissa, rule),
                            Price::from_sbe(mantissa, exponent, scale)
                        );
                        assert_eq!(
                            Quantity::from_prepared(mantissa, rule),
                            Quantity::from_sbe(mantissa, exponent, scale)
                        );
                    } else {
                        assert_eq!(
                            scale.prepare(exponent).err(),
                            reference(0, exponent, decimals).err()
                        );
                    }
                }
            }
        }
    }
}
