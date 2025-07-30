use crate::error::ContractResult;

// TODO validate profitabity correctly here
pub fn validate_entry() -> ContractResult<()> {
    // let new_balance = positions
    //     .deposits
    //     .iter()
    //     .find(|deposit| deposit.asset == config.spot_denom)
    //     .unwrap_or_default()
    //     .amount;

    // // Calculate the signed balance difference: positive for buy, negative for sell, using checked math
    // let spot_size = if new_balance >= previous_balance {
    //     Int128::from(new_balance.checked_sub(previous_balance)?)
    // } else {
    //     Int128::from(previous_balance.checked_sub(new_balance)?).checked_neg()?
    // };

    // let spot_price_impact = spot_size.checked_div(perp_size)?;

    // // We should probably enter based on the market interest rate.
    // // If the rate is higher we should be entering with worse execution, if the rate is lower we should be entering with better execution.
    // if spot_size > config.acceptable_entry_delta {
    //     return Err(ContractError::ProfitabilityValidationFailed {});
    // };

    Ok(())
}
