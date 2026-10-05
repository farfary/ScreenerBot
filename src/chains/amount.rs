// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Exact chain-neutral amounts represented in raw integer units.

use std::{fmt, num::TryFromIntError, str::FromStr};

pub(crate) mod math;

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, Value, ValueRef};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

/// A canonical decimal amount could not be parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AmountParseError {
    /// The input contained no digits.
    #[error("amount cannot be empty")]
    Empty,
    /// The input was not a canonical unsigned decimal integer.
    #[error("amount must be a canonical unsigned decimal integer")]
    InvalidFormat,
    /// The input exceeds the range of `u128`.
    #[error("amount exceeds the u128 range")]
    Overflow,
}

/// An exact unsigned amount in the smallest units defined by its asset.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RawAmount(u128);

impl RawAmount {
    /// The zero amount.
    pub const ZERO: Self = Self(0);

    /// The largest representable amount.
    pub const MAX: Self = Self(u128::MAX);

    /// Creates an amount from its raw integer value.
    pub const fn new(raw: u128) -> Self {
        Self(raw)
    }

    /// Returns the raw integer value.
    pub const fn raw(self) -> u128 {
        self.0
    }

    /// Adds two amounts if the result fits in `u128`.
    pub const fn checked_add(self, rhs: Self) -> Option<Self> {
        match self.0.checked_add(rhs.0) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Subtracts an amount if the result is nonnegative.
    pub const fn checked_sub(self, rhs: Self) -> Option<Self> {
        match self.0.checked_sub(rhs.0) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Multiplies two amounts if the result fits in `u128`.
    pub const fn checked_mul(self, rhs: Self) -> Option<Self> {
        match self.0.checked_mul(rhs.0) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Divides by a nonzero amount, truncating the quotient toward zero.
    pub const fn checked_div(self, rhs: Self) -> Option<Self> {
        match self.0.checked_div(rhs.0) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Multiplies, then divides with a floored quotient and a full-width product.
    ///
    /// Returns `None` for a zero divisor or a quotient outside `u128`.
    pub fn checked_mul_div(self, multiplier: Self, divisor: Self) -> Option<Self> {
        math::mul_div_floor(self.0, multiplier.0, divisor.0).map(Self)
    }
}

impl From<u64> for RawAmount {
    fn from(value: u64) -> Self {
        Self(u128::from(value))
    }
}

impl From<u128> for RawAmount {
    fn from(value: u128) -> Self {
        Self(value)
    }
}

impl TryFrom<RawAmount> for u64 {
    type Error = TryFromIntError;

    fn try_from(value: RawAmount) -> Result<Self, Self::Error> {
        u64::try_from(value.0)
    }
}

impl fmt::Display for RawAmount {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl FromStr for RawAmount {
    type Err = AmountParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty() {
            return Err(AmountParseError::Empty);
        }

        let bytes = value.as_bytes();
        if bytes == b"0" {
            return Ok(Self::ZERO);
        }
        if !matches!(bytes.first(), Some(b'1'..=b'9')) || !bytes.iter().all(u8::is_ascii_digit) {
            return Err(AmountParseError::InvalidFormat);
        }

        value
            .parse::<u128>()
            .map(Self)
            .map_err(|_| AmountParseError::Overflow)
    }
}

impl Serialize for RawAmount {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

struct RawAmountVisitor;

impl de::Visitor<'_> for RawAmountVisitor {
    type Value = RawAmount;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a canonical decimal string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        value.parse().map_err(E::custom)
    }
}

impl<'de> Deserialize<'de> for RawAmount {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(RawAmountVisitor)
    }
}

impl ToSql for RawAmount {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(Value::Text(self.to_string())))
    }
}

impl FromSql for RawAmount {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value {
            ValueRef::Text(bytes) => {
                let text = std::str::from_utf8(bytes)
                    .map_err(|error| FromSqlError::Other(Box::new(error)))?;
                text.parse()
                    .map_err(|error| FromSqlError::Other(Box::new(error)))
            }
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn raw_amount_constants_and_integer_conversions_preserve_values() {
        assert_eq!(RawAmount::ZERO.raw(), 0);
        assert_eq!(RawAmount::MAX.raw(), u128::MAX);
        assert_eq!(RawAmount::default(), RawAmount::ZERO);
        assert_eq!(RawAmount::from(u64::MAX).raw(), u128::from(u64::MAX));
        assert_eq!(RawAmount::from(u128::MAX).raw(), u128::MAX);
        assert_eq!(u64::try_from(RawAmount::from(u64::MAX)), Ok(u64::MAX));
        assert!(u64::try_from(RawAmount::from(u128::from(u64::MAX) + 1)).is_err());
    }

    #[test]
    fn decimal_display_and_parsing_round_trip_full_precision_values() {
        for raw in [0, 1, u128::from(u64::MAX), u128::MAX] {
            let amount = RawAmount::new(raw);
            assert_eq!(amount.to_string().parse::<RawAmount>(), Ok(amount));
        }

        let eighteen_decimals = RawAmount::new(1_234_567_890_123_456_789);
        assert_eq!(eighteen_decimals.to_string(), "1234567890123456789");
        assert_eq!(
            eighteen_decimals.to_string().parse::<RawAmount>(),
            Ok(eighteen_decimals)
        );
    }

    #[test]
    fn decimal_parsing_rejects_noncanonical_and_out_of_range_inputs() {
        for value in [
            "", "-1", "+1", " 1", "1 ", "01", "00", "١", "１", "1.0", "1e2", "0x10",
        ] {
            assert_eq!(
                value.parse::<RawAmount>(),
                Err(if value.is_empty() {
                    AmountParseError::Empty
                } else {
                    AmountParseError::InvalidFormat
                }),
                "input: {value:?}"
            );
        }

        assert_eq!(
            "340282366920938463463374607431768211456".parse::<RawAmount>(),
            Err(AmountParseError::Overflow)
        );
    }

    #[test]
    fn serde_uses_canonical_decimal_strings_only() {
        let amount = RawAmount::MAX;
        let encoded = serde_json::to_string(&amount).unwrap();
        assert_eq!(encoded, format!("\"{}\"", u128::MAX));
        assert_eq!(serde_json::from_str::<RawAmount>(&encoded).unwrap(), amount);

        for value in [
            "0", "1", "1.0", "1e2", "-1", "\"01\"", "null", "true", "[]", "{}",
        ] {
            assert!(
                serde_json::from_str::<RawAmount>(value).is_err(),
                "input: {value}"
            );
        }
    }

    #[test]
    fn sqlite_stores_and_reads_amounts_as_canonical_text() {
        let connection = Connection::open_in_memory().unwrap();
        let amount = RawAmount::MAX;
        let (storage_type, stored, decoded) = connection
            .query_row("SELECT typeof(?1), ?1, ?1", [amount], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, RawAmount>(2)?,
                ))
            })
            .unwrap();

        assert_eq!(storage_type, "text");
        assert_eq!(stored, u128::MAX.to_string());
        assert_eq!(decoded, amount);
    }

    #[test]
    fn sqlite_rejects_non_text_and_noncanonical_values() {
        let connection = Connection::open_in_memory().unwrap();
        for sql in [
            "SELECT 1",
            "SELECT 1.5",
            "SELECT X'31'",
            "SELECT NULL",
            "SELECT '01'",
            "SELECT '+1'",
            "SELECT '340282366920938463463374607431768211456'",
        ] {
            let result = connection.query_row(sql, [], |row| row.get::<_, RawAmount>(0));
            assert!(result.is_err(), "query: {sql}");
        }
    }

    #[test]
    fn checked_arithmetic_reports_overflow_underflow_zero_and_rounding() {
        let seven = RawAmount::new(7);
        let three = RawAmount::new(3);

        assert_eq!(seven.checked_add(three), Some(RawAmount::new(10)));
        assert_eq!(RawAmount::MAX.checked_add(RawAmount::new(1)), None);
        assert_eq!(three.checked_sub(seven), None);
        assert_eq!(seven.checked_mul(three), Some(RawAmount::new(21)));
        assert_eq!(RawAmount::MAX.checked_mul(RawAmount::new(2)), None);
        assert_eq!(seven.checked_div(three), Some(RawAmount::new(2)));
        assert_eq!(seven.checked_div(RawAmount::ZERO), None);
        assert_eq!(
            seven.checked_mul_div(three, RawAmount::new(2)),
            Some(RawAmount::new(10))
        );
        assert_eq!(seven.checked_mul_div(three, RawAmount::ZERO), None);
    }

    #[test]
    fn checked_mul_div_handles_full_width_products_and_quotient_overflow() {
        assert_eq!(
            RawAmount::MAX.checked_mul_div(RawAmount::new(2), RawAmount::new(2)),
            Some(RawAmount::MAX)
        );
        assert_eq!(
            RawAmount::MAX.checked_mul_div(RawAmount::MAX, RawAmount::MAX),
            Some(RawAmount::MAX)
        );
        assert_eq!(
            RawAmount::MAX.checked_mul_div(RawAmount::MAX, RawAmount::new(1)),
            None
        );
        assert_eq!(
            RawAmount::ZERO.checked_mul_div(RawAmount::MAX, RawAmount::new(1)),
            Some(RawAmount::ZERO)
        );
        assert_eq!(
            RawAmount::MAX.checked_mul_div(RawAmount::ZERO, RawAmount::MAX),
            Some(RawAmount::ZERO)
        );
        assert_eq!(
            RawAmount::ZERO.checked_mul_div(RawAmount::MAX, RawAmount::ZERO),
            None
        );
        assert_eq!(
            RawAmount::MAX.checked_mul_div(RawAmount::new(1), RawAmount::MAX),
            Some(RawAmount::new(1))
        );
    }
}
