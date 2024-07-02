use std::cmp::{max, min};
#[allow(unused_imports)] // FromStr is used in the tests
use std::{cmp::Ordering, str::FromStr};

use cosmwasm_std::{
    CheckedFromRatioError, Decimal, Decimal256, Fraction, StdError, Uint128, Uint256,
};
use schemars::JsonSchema;

use crate::{math::SignedDecimal, signed_uint::SignedUint};

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, JsonSchema)]
pub struct SignedRational {
    pub numerator: Uint256,
    pub denominator: Uint256,
    pub negative: bool,
}

impl SignedRational {
    pub fn new(numerator: Uint128, denominator: Uint128, negative: bool) -> Self {
        SignedRational {
            numerator: numerator.into(),
            denominator: denominator.into(),
            negative,
        }
    }

    pub fn one() -> SignedRational {
        SignedRational {
            numerator: Uint256::one(),
            denominator: Uint256::one(),
            negative: false,
        }
    }

    pub fn neg(&self) -> Self {
        SignedRational {
            numerator: self.numerator,
            denominator: self.denominator,
            negative: !self.negative,
        }
    }

    pub fn to_signed_uint(&self) -> Result<SignedUint, StdError> {
        let result = self.numerator.checked_div(self.denominator)?;
        let raw = Uint128::try_from(result)?;
        Ok(SignedUint {
            negative: self.negative,
            abs: raw,
        })
    }

    pub fn to_decimal_256(&self) -> Result<Decimal256, CheckedFromRatioError> {
        Decimal256::checked_from_ratio(self.numerator, self.denominator)
    }

    fn gcd(&self, mut n: Uint256, mut m: Uint256) -> Uint256 {
        while !m.is_zero() {
            match n.cmp(&m) {
                Ordering::Less => {
                    std::mem::swap(&mut n, &mut m);
                }
                _ => {
                    n %= m;
                }
            }
        }
        n
    }

    fn normalise(&self) -> Result<Self, StdError> {
        let gcd = self.gcd(self.numerator, self.denominator);

        Ok(SignedRational {
            numerator: self.numerator.checked_div(gcd)?,
            denominator: self.denominator.checked_div(gcd)?,
            negative: self.negative,
        })
    }

    pub fn mul_rational(&self, other: SignedRational) -> Result<SignedRational, StdError> {
        let numerator = self.numerator.checked_mul(other.numerator)?;
        let denominator = self.denominator.checked_mul(other.denominator)?;
        let negative = if numerator.is_zero() || denominator.is_zero() {
            false
        } else {
            // use the XOR bitwise operator
            self.negative ^ other.negative
        };
        SignedRational {
            numerator,
            denominator,
            negative,
        }
        .normalise()
    }

    pub fn div_rational(&self, other: SignedRational) -> Result<SignedRational, StdError> {
        let numerator = self.numerator.checked_mul(other.denominator)?;
        let denominator = self.denominator.checked_mul(other.numerator)?;
        let negative = if numerator.is_zero() || denominator.is_zero() {
            false
        } else {
            // use the XOR bitwise operator
            self.negative ^ other.negative
        };

        SignedRational {
            numerator,
            denominator,
            negative,
        }
        .normalise()
    }

    pub fn add_rational(self, other: SignedRational) -> Result<SignedRational, StdError> {
        let lh_numerator = self.numerator.checked_mul(other.denominator)?;
        let rh_numerator = other.numerator.checked_mul(self.denominator)?;
        let new_denominator = self.denominator.checked_mul(other.denominator)?;

        let result = if self.negative == other.negative {
            SignedRational {
                numerator: lh_numerator.checked_add(rh_numerator)?,
                denominator: new_denominator,
                negative: self.negative,
            }
        } else {
            if lh_numerator == rh_numerator {
                return Ok(SignedRational {
                    numerator: Uint256::zero(),
                    denominator: new_denominator,
                    negative: false,
                });
            }
            let max = max(lh_numerator, rh_numerator);
            let min = min(lh_numerator, rh_numerator);

            let negative = if lh_numerator == max {
                self.negative
            } else {
                other.negative
            };
            SignedRational {
                numerator: max.checked_sub(min)?,
                denominator: new_denominator,
                negative,
            }
        };

        result.normalise()
    }

    pub fn sub_rational(self, other: SignedRational) -> Result<SignedRational, StdError> {
        let lh_numerator = self.numerator.checked_mul(other.denominator)?;
        let rh_numerator = other.numerator.checked_mul(self.denominator)?;
        let new_denominator = self.denominator.checked_mul(other.denominator)?;

        let result = if self.negative == other.negative {
            if lh_numerator == rh_numerator {
                return Ok(SignedRational {
                    numerator: Uint256::zero(),
                    denominator: new_denominator,
                    negative: false,
                });
            }

            let max = max(lh_numerator, rh_numerator);
            let min = min(lh_numerator, rh_numerator);

            let negative = if lh_numerator == max {
                self.negative
            } else {
                !other.negative
            };

            SignedRational {
                numerator: max.checked_sub(min)?,
                denominator: new_denominator,
                negative,
            }
        } else {
            SignedRational {
                numerator: lh_numerator.checked_add(rh_numerator)?,
                denominator: new_denominator,
                negative: self.negative,
            }
        };

        result.normalise()
    }

    pub fn abs_sqrt(&self) -> Self {
        let abs_sqrt = Decimal256::from_ratio(self.numerator, self.denominator).sqrt();
        SignedRational {
            numerator: abs_sqrt.numerator(),
            denominator: abs_sqrt.denominator(),
            negative: false,
        }
    }
}

impl From<SignedDecimal> for SignedRational {
    fn from(signed_decimal: SignedDecimal) -> Self {
        SignedRational {
            negative: signed_decimal.negative,
            numerator: signed_decimal.abs.numerator().into(),
            denominator: signed_decimal.abs.denominator().into(),
        }
    }
}

impl From<SignedUint> for SignedRational {
    fn from(signed_uint: SignedUint) -> Self {
        SignedRational {
            negative: signed_uint.negative,
            numerator: signed_uint.abs.into(),
            denominator: Uint256::one(),
        }
    }
}

impl From<Uint128> for SignedRational {
    fn from(number: Uint128) -> Self {
        SignedRational {
            negative: false,
            numerator: number.into(),
            denominator: Uint256::one(),
        }
    }
}

impl From<Decimal> for SignedRational {
    fn from(decimal: Decimal) -> Self {
        SignedRational {
            negative: false,
            numerator: decimal.numerator().into(),
            denominator: decimal.denominator().into(),
        }
    }
}

impl From<Decimal256> for SignedRational {
    fn from(decimal: Decimal256) -> Self {
        SignedRational {
            negative: false,
            numerator: decimal.numerator(),
            denominator: decimal.denominator(),
        }
    }
}

#[test]
fn subtract() {
    // 1/2
    let half = SignedRational {
        numerator: Uint256::from_str("1000000000").unwrap(),
        denominator: Uint256::from_str("2000000000").unwrap(),
        negative: false,
    };

    // -1/2
    let negative_half = SignedRational {
        numerator: Uint256::from_str("1000000000").unwrap(),
        denominator: Uint256::from_str("2000000000").unwrap(),
        negative: true,
    };

    // 2/1
    let two = SignedRational {
        numerator: Uint256::from_str("2000000000").unwrap(),
        denominator: Uint256::from_str("1000000000").unwrap(),
        negative: false,
    };

    // 1/2 - 1/2 = 0
    let result = half.sub_rational(half).unwrap();
    assert_eq!(result.to_signed_uint(), Ok(SignedUint::zero()));

    // 1/2 - -1/2 = 1
    let result = half.sub_rational(negative_half).unwrap();
    assert_eq!(result.to_signed_uint(), Ok(SignedUint::one()));

    // 2 - 1/2 = 1
    let result = two.sub_rational(half).unwrap();
    assert_eq!(result.to_signed_uint(), Ok(SignedUint::from_str("1").unwrap()));
}
#[test]

fn multiply() {
    // 1/2
    let half = SignedRational {
        numerator: Uint256::from_str("1000000000").unwrap(),
        denominator: Uint256::from_str("2000000000").unwrap(),
        negative: false,
    };

    // -1/2
    let negative_half = SignedRational {
        numerator: Uint256::from_str("1000000000").unwrap(),
        denominator: Uint256::from_str("2000000000").unwrap(),
        negative: true,
    };

    // 2/1
    let two = SignedRational {
        numerator: Uint256::from_str("2000000000").unwrap(),
        denominator: Uint256::from_str("1000000000").unwrap(),
        negative: false,
    };

    // 10
    let ten = SignedRational {
        numerator: Uint256::from_str("10000000000").unwrap(),
        denominator: Uint256::from_str("1000000000").unwrap(),
        negative: false,
    };

    // 2 * 0.5 = 1
    let result = half.mul_rational(two).unwrap();
    assert_eq!(SignedUint::one(), result.to_signed_uint().unwrap());

    // 10 * 2 = 20
    let result = ten.mul_rational(two).unwrap();
    assert_eq!(result.to_signed_uint().unwrap(), SignedUint::from_str("20").unwrap());

    //-0.5 * -0.5 = 0.25
    let result = negative_half.mul_rational(negative_half).unwrap();
    assert_eq!(result.to_decimal_256().unwrap(), Decimal256::from_str("0.25").unwrap());
}

#[test]
fn divide() {
    // 1/2
    let half = SignedRational {
        numerator: Uint256::from_str("1000000000").unwrap(),
        denominator: Uint256::from_str("2000000000").unwrap(),
        negative: false,
    };

    // 2/1
    let two = SignedRational {
        numerator: Uint256::from_str("2000000000").unwrap(),
        denominator: Uint256::from_str("1000000000").unwrap(),
        negative: false,
    };

    let four = SignedRational {
        numerator: Uint256::from_str("4000000000").unwrap(),
        denominator: Uint256::from_str("1000000000").unwrap(),
        negative: false,
    };

    // -2/1
    let negative_two = SignedRational {
        numerator: Uint256::from_str("2000000000").unwrap(),
        denominator: Uint256::from_str("1000000000").unwrap(),
        negative: true,
    };

    // 2 / 0.5 = 4
    let result = two.div_rational(half).unwrap();
    assert_eq!(result.to_signed_uint().unwrap(), SignedUint::from_str("4").unwrap());

    // -2 / 2 = -1
    assert_eq!(
        negative_two.div_rational(two).unwrap().to_signed_uint().unwrap(),
        SignedUint::from_str("-1").unwrap()
    );

    // 2 / 4 = 0.5
    assert_eq!(
        two.div_rational(four).unwrap().to_decimal_256().unwrap(),
        Decimal256::from_str("0.5").unwrap()
    );
}

#[test]
fn add() {
    // 1/2
    let half = SignedRational {
        numerator: Uint256::from_str("1000000000").unwrap(),
        denominator: Uint256::from_str("2000000000").unwrap(),
        negative: false,
    };

    // 2/1
    let two = SignedRational {
        numerator: Uint256::from_str("2000000000").unwrap(),
        denominator: Uint256::from_str("1000000000").unwrap(),
        negative: false,
    };

    // 4/1
    let four = SignedRational {
        numerator: Uint256::from_str("4000000000").unwrap(),
        denominator: Uint256::from_str("1000000000").unwrap(),
        negative: false,
    };

    // -4/1
    let negative_four = SignedRational {
        numerator: Uint256::from_str("4000000000").unwrap(),
        denominator: Uint256::from_str("1000000000").unwrap(),
        negative: true,
    };

    // -2/1
    let negative_two = SignedRational {
        numerator: Uint256::from_str("2000000000").unwrap(),
        denominator: Uint256::from_str("1000000000").unwrap(),
        negative: true,
    };

    // 2 + 1/2 = 2.5
    let result = two.add_rational(half).unwrap();
    assert_eq!(result.to_decimal_256().unwrap(), Decimal256::from_str("2.5").unwrap());

    // 2 + -2 = 0
    assert_eq!(
        two.add_rational(negative_two).unwrap().to_signed_uint().unwrap(),
        SignedUint::zero()
    );

    // 2 + 4 = 6
    assert_eq!(
        two.add_rational(four).unwrap().to_signed_uint().unwrap(),
        SignedUint::from_str("6").unwrap()
    );

    // 2 + -4 = -2
    assert_eq!(
        two.add_rational(negative_four).unwrap().to_signed_uint().unwrap(),
        SignedUint::from_str("-2").unwrap()
    );
}
