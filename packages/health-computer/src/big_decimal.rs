use std::str::FromStr;

use bigdecimal::BigDecimal;
use cosmwasm_std::{Decimal, Int128, Uint128};

/// Safely unwraps, as any number already represented by types like `Decimal` or `Int128`
/// will also be compatible with `BigDecimal`.

pub trait ToBigDecimal: ToString {
    fn bd(&self) -> BigDecimal;
}

impl ToBigDecimal for Uint128 {
    fn bd(&self) -> BigDecimal {
        BigDecimal::from_str(&self.to_string()).unwrap()
    }
}

impl ToBigDecimal for Int128 {
    fn bd(&self) -> BigDecimal {
        BigDecimal::from_str(&self.to_string()).unwrap()
    }
}

impl ToBigDecimal for Decimal {
    fn bd(&self) -> BigDecimal {
        BigDecimal::from_str(&self.to_string()).unwrap()
    }
}
