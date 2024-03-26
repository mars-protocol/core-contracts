use std::{
    cmp::Ordering,
    fmt::{self, Write},
    str::FromStr,
};

use cosmwasm_std::{
    CheckedMultiplyFractionError, DivideByZeroError, OverflowError, StdError, Uint128,
};
use schemars::JsonSchema;
use serde::{de, ser, Deserialize, Serialize};

use crate::math::SignedDecimal;

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, JsonSchema)]
pub struct SignedUint {
    pub negative: bool,
    pub abs: Uint128,
}

impl SignedUint {
    pub fn zero() -> Self {
        Self {
            negative: false,
            abs: Uint128::zero(),
        }
    }

    pub fn one() -> Self {
        Self {
            negative: false,
            abs: Uint128::one(),
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

    /// Add to the SignedUint by another SignedUint.
    pub fn checked_add(&self, other: SignedUint) -> Result<SignedUint, OverflowError> {
        match (self.negative, other.negative) {
            // positive + negative
            (false, true) => {
                if self.abs >= other.abs {
                    Ok(SignedUint {
                        negative: false,
                        abs: self.abs - other.abs,
                    })
                } else {
                    Ok(SignedUint {
                        negative: true,
                        abs: other.abs - self.abs,
                    })
                }
            }
            // negative + positive
            (true, false) => match self.abs.cmp(&other.abs) {
                Ordering::Greater => Ok(SignedUint {
                    negative: true,
                    abs: self.abs - other.abs,
                }),
                Ordering::Less => Ok(SignedUint {
                    negative: false,
                    abs: other.abs - self.abs,
                }),
                Ordering::Equal => Ok(SignedUint {
                    negative: false,
                    abs: Uint128::zero(),
                }),
            },
            // positive + positive
            (false, false) => Ok(SignedUint {
                negative: false,
                abs: self.abs.checked_add(other.abs)?,
            }),
            // negative + negative
            (true, true) => Ok(SignedUint {
                negative: true,
                abs: self.abs.checked_add(other.abs)?,
            }),
        }
    }

    /// Subtract the SignedUint by another SignedUint.
    pub fn checked_sub(&self, subtractor: SignedUint) -> Result<SignedUint, OverflowError> {
        match (self.negative, subtractor.negative) {
            // a positive number - a positive number
            (false, false) => {
                if self.abs >= subtractor.abs {
                    Ok(SignedUint {
                        negative: false,
                        abs: self.abs - subtractor.abs,
                    })
                } else {
                    Ok(SignedUint {
                        negative: true,
                        abs: subtractor.abs - self.abs,
                    })
                }
            }
            // a negative number - a negative number
            (true, true) => match self.abs.cmp(&subtractor.abs) {
                Ordering::Greater => Ok(SignedUint {
                    negative: true,
                    abs: self.abs - subtractor.abs,
                }),
                Ordering::Less => Ok(SignedUint {
                    negative: false,
                    abs: subtractor.abs - self.abs,
                }),
                Ordering::Equal => Ok(SignedUint {
                    negative: false,
                    abs: Uint128::zero(),
                }),
            },
            // a negative number - a positive number
            (true, false) => Ok(SignedUint {
                negative: true,
                abs: self.abs.checked_add(subtractor.abs)?,
            }),
            // a positive number - a negative number
            (false, true) => Ok(SignedUint {
                negative: false,
                abs: self.abs.checked_add(subtractor.abs)?,
            }),
        }
    }

    /// Multiple the SignedUint by another SignedUint.
    pub fn checked_mul(&self, multiplier: SignedUint) -> Result<SignedUint, OverflowError> {
        let abs = self.abs.checked_mul(multiplier.abs)?;
        let negative = if abs.is_zero() {
            false
        } else {
            // use the XOR bitwise operator
            self.negative ^ multiplier.negative
        };
        Ok(SignedUint {
            negative,
            abs,
        })
    }

    pub fn checked_mul_floor(
        &self,
        multiplier: SignedDecimal,
    ) -> Result<SignedUint, CheckedMultiplyFractionError> {
        let negative = if self.abs.is_zero() || multiplier.abs.is_zero() {
            false
        } else {
            // use the XOR bitwise operator
            self.negative ^ multiplier.negative
        };

        let res = if negative {
            SignedUint {
                negative,
                abs: self.abs.checked_mul_ceil(multiplier.abs)?,
            }
        } else {
            SignedUint {
                negative,
                abs: self.abs.checked_mul_floor(multiplier.abs)?,
            }
        };
        Ok(res)
    }

    pub fn checked_mul_ceil(
        &self,
        multiplier: SignedDecimal,
    ) -> Result<SignedUint, CheckedMultiplyFractionError> {
        let negative = if self.abs.is_zero() || multiplier.abs.is_zero() {
            false
        } else {
            // use the XOR bitwise operator
            self.negative ^ multiplier.negative
        };

        let res = if negative {
            SignedUint {
                negative,
                abs: self.abs.checked_mul_floor(multiplier.abs)?,
            }
        } else {
            SignedUint {
                negative,
                abs: self.abs.checked_mul_ceil(multiplier.abs)?,
            }
        };
        Ok(res)
    }

    /// Divide the SignedUint by another SignedUint.
    pub fn checked_div(&self, divisor: SignedUint) -> Result<SignedUint, DivideByZeroError> {
        let abs = self.abs.checked_div(divisor.abs)?;
        let negative = if abs.is_zero() {
            false
        } else {
            // the divisor is always non-negative, so sign doesn't change
            self.negative ^ divisor.negative
        };
        Ok(SignedUint {
            negative,
            abs,
        })
    }

    pub fn checked_div_floor(
        &self,
        divisor: SignedDecimal,
    ) -> Result<SignedUint, CheckedMultiplyFractionError> {
        let negative = if self.abs.is_zero() || divisor.abs.is_zero() {
            false
        } else {
            // use the XOR bitwise operator
            self.negative ^ divisor.negative
        };

        let res = if negative {
            SignedUint {
                negative,
                abs: self.abs.checked_div_ceil(divisor.abs)?,
            }
        } else {
            SignedUint {
                negative,
                abs: self.abs.checked_div_floor(divisor.abs)?,
            }
        };
        Ok(res)
    }

    pub fn checked_div_ceil(
        &self,
        divisor: SignedDecimal,
    ) -> Result<SignedUint, CheckedMultiplyFractionError> {
        let negative = if self.abs.is_zero() || divisor.abs.is_zero() {
            false
        } else {
            // use the XOR bitwise operator
            self.negative ^ divisor.negative
        };

        let res = if negative {
            SignedUint {
                negative,
                abs: self.abs.checked_div_floor(divisor.abs)?,
            }
        } else {
            SignedUint {
                negative,
                abs: self.abs.checked_div_ceil(divisor.abs)?,
            }
        };
        Ok(res)
    }
}

impl From<Uint128> for SignedUint {
    fn from(abs: Uint128) -> Self {
        SignedUint {
            negative: false,
            abs,
        }
    }
}

impl PartialOrd for SignedUint {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SignedUint {
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

impl std::fmt::Display for SignedUint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.is_negative() {
            f.write_char('-')?;
        }

        f.write_str(&self.abs.to_string())
    }
}

impl FromStr for SignedUint {
    type Err = StdError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match &input[..1] {
            "-" => Ok(SignedUint {
                negative: true,
                abs: Uint128::from_str(&input[1..])?,
            }),
            _ => Ok(SignedUint {
                negative: false,
                abs: Uint128::from_str(input)?,
            }),
        }
    }
}

impl Serialize for SignedUint {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for SignedUint {
    fn deserialize<D>(deserializer: D) -> Result<SignedUint, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_str(Visitor)
    }
}

struct Visitor;

impl<'de> de::Visitor<'de> for Visitor {
    type Value = SignedUint;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a signed integer")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        SignedUint::from_str(v).map_err(E::custom)
    }
}
