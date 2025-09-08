use cosmwasm_std::{Decimal, SignedDecimal};
use mars_rover_health_computer::Direction;

use crate::error::ContractResult;

pub trait Validator {
    fn validate_order_execution(
        &self,
        perp_funding_rate: SignedDecimal,
        net_spot_yield: SignedDecimal,
        spot_execution_price: SignedDecimal,
        perp_execution_price: SignedDecimal,
        perp_trading_fee_rate: Decimal,
        direction: Direction,
    ) -> ContractResult<bool>;
    
    fn validate(&self) -> ContractResult<()>;
}
