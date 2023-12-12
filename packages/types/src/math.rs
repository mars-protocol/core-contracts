use std::{
    cmp::Ordering,
    fmt::{self, Write},
    str::FromStr,
};

use cosmwasm_std::{CheckedFromRatioError, Decimal, OverflowError, StdError, Uint128};
use schemars::JsonSchema;
use serde::{de, ser, Deserialize, Serialize};

/// Inspired by Margined protocol's implementation:
/// https://github.com/margined-protocol/perpetuals/blob/main/packages/margined_common/src/integer.rs
///
/// This type is specifically adapted to our need (only methods that are
/// actually used by the contract are implemented) hence not suited for general
/// use.
//
// TODO: we should manually implement PartialEq such that +0 and -0 are
// considered equal
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

    pub fn floor(&self) -> Self {
        Self {
            negative: self.negative,
            abs: self.abs.floor(),
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
            (true, false) => {
                if self.abs >= other.abs {
                    Ok(SignedDecimal {
                        negative: true,
                        abs: self.abs - other.abs,
                    })
                } else {
                    Ok(SignedDecimal {
                        negative: false,
                        abs: other.abs - self.abs,
                    })
                }
            }
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
            (true, true) => {
                if self.abs >= subtractor.abs {
                    Ok(SignedDecimal {
                        negative: true,
                        abs: self.abs - subtractor.abs,
                    })
                } else {
                    Ok(SignedDecimal {
                        negative: false,
                        abs: subtractor.abs - self.abs,
                    })
                }
            }
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
        Ok(SignedDecimal {
            // use the XOR bitwise operator
            negative: self.negative ^ multiplier.negative,
            abs: self.abs.checked_mul(multiplier.abs)?,
        })
    }

    /// Divide the SignedDecimal by another SignedDecimal.
    pub fn checked_div(
        &self,
        divisor: SignedDecimal,
    ) -> Result<SignedDecimal, CheckedFromRatioError> {
        Ok(SignedDecimal {
            // the divisor is always non-negative, so sign doesn't change
            negative: self.negative ^ divisor.negative,
            abs: self.abs.checked_div(divisor.abs)?,
        })
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
        if self.is_negative() && other.is_positive() {
            Some(Ordering::Less)
        } else if self.is_positive() && other.is_negative() {
            Some(Ordering::Greater)
        } else if self.is_positive() {
            self.abs.partial_cmp(&other.abs)
        } else {
            other.abs.partial_cmp(&self.abs)
        }
    }
}

impl Ord for SignedDecimal {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.is_negative() && other.is_positive() {
            Ordering::Less
        } else if self.is_positive() && other.is_negative() {
            Ordering::Greater
        } else if self.is_positive() {
            self.abs.cmp(&other.abs)
        } else {
            other.abs.cmp(&self.abs)
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
