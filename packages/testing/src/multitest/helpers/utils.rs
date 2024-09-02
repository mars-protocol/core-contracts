use cosmwasm_std::{Coin, Uint128};
use mars_types::credit_manager::DebtAmount;

pub fn get_coin(denom: &str, coins: &[Coin]) -> Coin {
    coins.iter().find(|cv| cv.denom == denom).unwrap().clone()
}

pub fn get_debt(denom: &str, coins: &[DebtAmount]) -> DebtAmount {
    coins
        .iter()
        .find(|coin| coin.denom.as_str() == denom)
        .unwrap_or(&DebtAmount {
            denom: denom.to_string(),
            shares: Uint128::zero(),
            amount: Uint128::zero(),
        })
        .clone()
}
