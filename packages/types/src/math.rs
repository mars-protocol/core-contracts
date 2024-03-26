use std::{
    cmp::Ordering,
    fmt::{self, Write},
    str::FromStr,
};

use cosmwasm_std::{CheckedFromRatioError, Decimal, OverflowError, StdError, Uint128};
use schemars::JsonSchema;
use serde::{de, ser, Deserialize, Serialize};

use crate::signed_uint::SignedUint;

/// Inspired by Margined protocol's implementation:
/// https://github.com/margined-protocol/perpetuals/blob/main/packages/margined_common/src/integer.rs
///
/// This type is specifically adapted to our need (only methods that are
/// actually used by the contract are implemented) hence not suited for general
/// use.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, JsonSchema)]
pub struct SignedDecimal {
    pub negative: bool,
    pub abs: Decimal,
}

impl SignedDecimal {
    pub fn zero() -> Self {
        Self {
            negative: false,
            abs: Decimal::zero(),
        }
    }

    pub fn one() -> Self {
        Self {
            negative: false,
            abs: Decimal::one(),
        }
    }

    pub fn is_zero(&self) -> bool {
        self.abs.is_zero()
    }

    pub fn is_positive(&self) -> bool {
        !self.is_zero() && !self.negative
    }

    pub fn is_negative(&self) -> bool {
        !self.is_zero() && self.negative
    }

    /// Add to the signedDecimal by another SignedDecimal.
    pub fn checked_add(&self, other: SignedDecimal) -> Result<SignedDecimal, OverflowError> {
        match (self.negative, other.negative) {
            // positive + negative
            (false, true) => {
                if self.abs >= other.abs {
                    Ok(SignedDecimal {
                        negative: false,
                        abs: self.abs - other.abs,
                    })
                } else {
                    Ok(SignedDecimal {
                        negative: true,
                        abs: other.abs - self.abs,
                    })
                }
            }
            // negative + positive
            (true, false) => match self.abs.cmp(&other.abs) {
                Ordering::Greater => Ok(SignedDecimal {
                    negative: true,
                    abs: self.abs - other.abs,
                }),
                Ordering::Less => Ok(SignedDecimal {
                    negative: false,
                    abs: other.abs - self.abs,
                }),
                Ordering::Equal => Ok(SignedDecimal {
                    negative: false,
                    abs: Decimal::zero(),
                }),
            },
            // positive + positive
            (false, false) => Ok(SignedDecimal {
                negative: false,
                abs: self.abs.checked_add(other.abs)?,
            }),
            // negative + negative
            (true, true) => Ok(SignedDecimal {
                negative: true,
                abs: self.abs.checked_add(other.abs)?,
            }),
        }
    }

    /// Subtract the SignedDecimal by another SignedDecimal.
    pub fn checked_sub(&self, subtractor: SignedDecimal) -> Result<SignedDecimal, OverflowError> {
        match (self.negative, subtractor.negative) {
            // a positive number - a positive number
            (false, false) => {
                if self.abs >= subtractor.abs {
                    Ok(SignedDecimal {
                        negative: false,
                        abs: self.abs - subtractor.abs,
                    })
                } else {
                    Ok(SignedDecimal {
                        negative: true,
                        abs: subtractor.abs - self.abs,
                    })
                }
            }
            // a negative number - a negative number
            (true, true) => match self.abs.cmp(&subtractor.abs) {
                Ordering::Greater => Ok(SignedDecimal {
                    negative: true,
                    abs: self.abs - subtractor.abs,
                }),
                Ordering::Less => Ok(SignedDecimal {
                    negative: false,
                    abs: subtractor.abs - self.abs,
                }),
                Ordering::Equal => Ok(SignedDecimal {
                    negative: false,
                    abs: Decimal::zero(),
                }),
            },
            // a negative number - a positive number
            (true, false) => Ok(SignedDecimal {
                negative: true,
                abs: self.abs.checked_add(subtractor.abs)?,
            }),
            // a positive number - a negative number
            (false, true) => Ok(SignedDecimal {
                negative: false,
                abs: self.abs.checked_add(subtractor.abs)?,
            }),
        }
    }

    /// Multiple the SignedDecimal by another SignedDecimal.
    pub fn checked_mul(&self, multiplier: SignedDecimal) -> Result<SignedDecimal, OverflowError> {
        let abs = self.abs.checked_mul(multiplier.abs)?;
        let negative = if abs.is_zero() {
            false
        } else {
            // use the XOR bitwise operator
            self.negative ^ multiplier.negative
        };
        Ok(SignedDecimal {
            negative,
            abs,
        })
    }

    /// Divide the SignedDecimal by another SignedDecimal.
    pub fn checked_div(
        &self,
        divisor: SignedDecimal,
    ) -> Result<SignedDecimal, CheckedFromRatioError> {
        let abs = self.abs.checked_div(divisor.abs)?;
        let negative = if abs.is_zero() {
            false
        } else {
            // the divisor is always non-negative, so sign doesn't change
            self.negative ^ divisor.negative
        };
        Ok(SignedDecimal {
            negative,
            abs,
        })
    }

    pub fn checked_from_ratio(
        numerator: SignedUint,
        denominator: SignedUint,
    ) -> Result<SignedDecimal, CheckedFromRatioError> {
        let abs = Decimal::checked_from_ratio(numerator.abs, denominator.abs)?;
        let negative = if abs.is_zero() {
            false
        } else {
            // use the XOR bitwise operator
            numerator.negative ^ denominator.negative
        };
        Ok(SignedDecimal {
            negative,
            abs,
        })
    }

    pub fn to_signed_uint_floor(self) -> SignedUint {
        if self.is_negative() {
            SignedUint {
                negative: true,
                abs: self.abs.to_uint_ceil(),
            }
        } else {
            SignedUint {
                negative: false,
                abs: self.abs.to_uint_floor(),
            }
        }
    }
}

impl From<Decimal> for SignedDecimal {
    fn from(abs: Decimal) -> Self {
        SignedDecimal {
            negative: false,
            abs,
        }
    }
}

impl From<Uint128> for SignedDecimal {
    fn from(abs: Uint128) -> Self {
        SignedDecimal {
            negative: false,
            abs: Decimal::from_atomics(abs, 0).unwrap(),
        }
    }
}

impl PartialOrd for SignedDecimal {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SignedDecimal {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.is_negative(), other.is_negative()) {
            (true, true) => other.abs.cmp(&self.abs),
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => {
                let self_abs = self.abs;
                let other_abs = other.abs;
                match (self_abs.is_zero(), other_abs.is_zero()) {
                    (true, true) => Ordering::Equal,
                    (true, false) if other.is_positive() => Ordering::Less,
                    (true, false) => Ordering::Greater,
                    (false, true) if self.is_positive() => Ordering::Greater,
                    (false, true) => Ordering::Less,
                    (false, false) => self_abs.cmp(&other_abs),
                }
            }
        }
    }
}

impl std::fmt::Display for SignedDecimal {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.is_negative() {
            f.write_char('-')?;
        }

        f.write_str(&self.abs.to_string())
    }
}

impl FromStr for SignedDecimal {
    type Err = StdError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match &input[..1] {
            "-" => Ok(SignedDecimal {
                negative: true,
                abs: Decimal::from_str(&input[1..])?,
            }),
            _ => Ok(SignedDecimal {
                negative: false,
                abs: Decimal::from_str(input)?,
            }),
        }
    }
}

impl Serialize for SignedDecimal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for SignedDecimal {
    fn deserialize<D>(deserializer: D) -> Result<SignedDecimal, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_str(Visitor)
    }
}

struct Visitor;

impl<'de> de::Visitor<'de> for Visitor {
    type Value = SignedDecimal;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a signed integer")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        SignedDecimal::from_str(v).map_err(E::custom)
    }
}

#[cfg(test)]
mod tests {
    use std::{cmp, str::FromStr};

    use super::*;

    #[test]
    fn max() {
        let val = cmp::max(SignedDecimal::zero(), SignedDecimal::from_str("-1").unwrap());
        assert_eq!(val, SignedDecimal::zero());

        let val = cmp::max(SignedDecimal::zero(), SignedDecimal::zero());
        assert_eq!(val, SignedDecimal::zero());

        let val = cmp::max(SignedDecimal::zero(), SignedDecimal::from_str("1").unwrap());
        assert_eq!(val, SignedDecimal::from_str("1").unwrap());

        let val = cmp::max(SignedDecimal::from_str("1").unwrap(), SignedDecimal::zero());
        assert_eq!(val, SignedDecimal::from_str("1").unwrap());

        let val =
            cmp::max(SignedDecimal::from_str("1").unwrap(), SignedDecimal::from_str("1").unwrap());
        assert_eq!(val, SignedDecimal::from_str("1").unwrap());

        let val =
            cmp::max(SignedDecimal::from_str("1").unwrap(), SignedDecimal::from_str("2").unwrap());
        assert_eq!(val, SignedDecimal::from_str("2").unwrap());

        let val =
            cmp::max(SignedDecimal::from_str("2").unwrap(), SignedDecimal::from_str("1").unwrap());
        assert_eq!(val, SignedDecimal::from_str("2").unwrap());

        let val = cmp::max(SignedDecimal::from_str("-1").unwrap(), SignedDecimal::zero());
        assert_eq!(val, SignedDecimal::zero());

        let val = cmp::max(
            SignedDecimal::from_str("-1").unwrap(),
            SignedDecimal::from_str("-1").unwrap(),
        );
        assert_eq!(val, SignedDecimal::from_str("-1").unwrap());

        let val = cmp::max(
            SignedDecimal::from_str("-1").unwrap(),
            SignedDecimal::from_str("-2").unwrap(),
        );
        assert_eq!(val, SignedDecimal::from_str("-1").unwrap());

        let val = cmp::max(
            SignedDecimal::from_str("-2").unwrap(),
            SignedDecimal::from_str("-1").unwrap(),
        );
        assert_eq!(val, SignedDecimal::from_str("-1").unwrap());
    }

    #[test]
    fn min() {
        let val = cmp::min(SignedDecimal::zero(), SignedDecimal::from_str("-1").unwrap());
        assert_eq!(val, SignedDecimal::from_str("-1").unwrap());

        let val = cmp::min(SignedDecimal::zero(), SignedDecimal::zero());
        assert_eq!(val, SignedDecimal::zero());

        let val = cmp::min(SignedDecimal::zero(), SignedDecimal::from_str("1").unwrap());
        assert_eq!(val, SignedDecimal::zero());

        let val = cmp::min(SignedDecimal::from_str("1").unwrap(), SignedDecimal::zero());
        assert_eq!(val, SignedDecimal::zero());

        let val =
            cmp::min(SignedDecimal::from_str("1").unwrap(), SignedDecimal::from_str("1").unwrap());
        assert_eq!(val, SignedDecimal::from_str("1").unwrap());

        let val =
            cmp::min(SignedDecimal::from_str("1").unwrap(), SignedDecimal::from_str("2").unwrap());
        assert_eq!(val, SignedDecimal::from_str("1").unwrap());

        let val =
            cmp::min(SignedDecimal::from_str("2").unwrap(), SignedDecimal::from_str("1").unwrap());
        assert_eq!(val, SignedDecimal::from_str("1").unwrap());

        let val = cmp::min(SignedDecimal::from_str("-1").unwrap(), SignedDecimal::zero());
        assert_eq!(val, SignedDecimal::from_str("-1").unwrap());

        let val = cmp::min(
            SignedDecimal::from_str("-1").unwrap(),
            SignedDecimal::from_str("-1").unwrap(),
        );
        assert_eq!(val, SignedDecimal::from_str("-1").unwrap());

        let val = cmp::min(
            SignedDecimal::from_str("-1").unwrap(),
            SignedDecimal::from_str("-2").unwrap(),
        );
        assert_eq!(val, SignedDecimal::from_str("-2").unwrap());

        let val = cmp::min(
            SignedDecimal::from_str("-2").unwrap(),
            SignedDecimal::from_str("-1").unwrap(),
        );
        assert_eq!(val, SignedDecimal::from_str("-2").unwrap());
    }
}
