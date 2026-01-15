use cosmwasm_std::{
    to_json_binary, Addr, CosmosMsg, DepsMut, Env, Order, Response, StdError, Uint128, WasmMsg,
};
use cw2::assert_contract_version;
use mars_types::{
    address_provider::{self, MarsAddressType},
    red_bank::{ExecuteMsg as RedBankExecuteMsg, PaginatedUserCollateralResponse, QueryMsg},
};

use crate::{
    contract::{CONTRACT_NAME, CONTRACT_VERSION},
    error::ContractError,
    state::{ACCOUNT_NFT, COIN_BALANCES, DEBT_SHARES, TOTAL_DEBT_SHARES},
};

// const FROM_CONTRACT_VERSION: &str = "2.4.1";

pub fn migrate(
    deps: DepsMut,
    env: Env,
    address_provider: Addr,
    bad_debt_owner: Addr,
    denom: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<Response, ContractError> {
    // assert_contract_version(
    //     deps.storage,
    //     &format!("crates.io:{CONTRACT_NAME}"),
    //     FROM_CONTRACT_VERSION,
    // )?;

    let account_nft = ACCOUNT_NFT.load(deps.storage)?;
    let tokens = account_nft.query_tokens(
        &deps.querier,
        bad_debt_owner.to_string(),
        start_after,
        limit,
    )?;
    let account_ids = tokens.tokens;

    let red_bank_addr = address_provider::helpers::query_contract_addr(
        deps.as_ref(),
        &address_provider,
        MarsAddressType::RedBank,
    )?;

    let mut response = Response::new()
        .add_attribute("action", "write_off_bad_debt")
        .add_attribute("address_provider", address_provider.to_string())
        .add_attribute("red_bank", red_bank_addr.to_string())
        .add_attribute("bad_debt_owner", bad_debt_owner.to_string())
        .add_attribute("denom", denom.clone())
        .add_attribute("accounts_processed", account_ids.len().to_string());

    if let Some(last_account_id) = account_ids.last() {
        response = response.add_attribute("last_account_id", last_account_id);
    }

    let mut bad_shares = Uint128::zero();
    let mut accounts_written_off = 0u32;
    let mut accounts_skipped = 0u32;

    for account_id in account_ids.iter() {
        if has_coin_balance(deps.storage, account_id)
            || has_red_bank_lend(deps.as_ref(), &red_bank_addr, &env.contract.address, account_id)?
        {
            accounts_skipped += 1;
            continue;
        }

        let debt_shares = DEBT_SHARES
            .may_load(deps.storage, (account_id.as_str(), denom.as_str()))?
            .unwrap_or_else(Uint128::zero);

        if debt_shares.is_zero() {
            continue;
        }

        accounts_written_off += 1;
        bad_shares = bad_shares.checked_add(debt_shares)?;
        DEBT_SHARES.remove(deps.storage, (account_id.as_str(), denom.as_str()));
    }

    response = response
        .add_attribute("accounts_written_off", accounts_written_off.to_string())
        .add_attribute("accounts_skipped", accounts_skipped.to_string());

    let total_shares = TOTAL_DEBT_SHARES
        .may_load(deps.storage, &denom)?
        .unwrap_or_else(Uint128::zero);
    if total_shares.is_zero() {
        return Err(StdError::generic_err(format!(
            "Total debt shares is zero for denom {denom}"
        ))
        .into());
    }

    let total_debt_amount = query_red_bank_total_debt(
        deps.as_ref(),
        &red_bank_addr,
        &env.contract.address,
        &denom,
    )?;

    let writeoff_amount = if total_debt_amount.is_zero() {
        Uint128::zero()
    } else {
        total_debt_amount.checked_mul_ceil((bad_shares, total_shares))?
    };

    let new_total_shares = total_shares.checked_sub(bad_shares)?;
    TOTAL_DEBT_SHARES.save(deps.storage, &denom, &new_total_shares)?;

    if !writeoff_amount.is_zero() {
        let msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: red_bank_addr.to_string(),
            msg: to_json_binary(&RedBankExecuteMsg::WriteOffBadDebt {
                denom: denom.clone(),
                amount: writeoff_amount,
            })?,
            funds: vec![],
        });
        response = response.add_message(msg);
    }

    response = response
        .add_attribute("writeoff_amount", writeoff_amount)
        .add_attribute("bad_shares", bad_shares)
        .add_attribute("total_shares", total_shares);

    Ok(response)
}

fn has_coin_balance(storage: &dyn cosmwasm_std::Storage, account_id: &str) -> bool {
    COIN_BALANCES
        .prefix(account_id)
        .range(storage, None, None, Order::Ascending)
        .next()
        .is_some()
}

fn has_red_bank_lend(
    deps: cosmwasm_std::Deps,
    red_bank_addr: &Addr,
    credit_manager_addr: &Addr,
    account_id: &str,
) -> Result<bool, ContractError> {
    let res: PaginatedUserCollateralResponse = deps.querier.query_wasm_smart(
        red_bank_addr.to_string(),
        &QueryMsg::UserCollateralsV2 {
            user: credit_manager_addr.to_string(),
            account_id: Some(account_id.to_string()),
            start_after: None,
            limit: Some(1),
        },
    )?;
    Ok(!res.data.is_empty())
}

fn query_red_bank_total_debt(
    deps: cosmwasm_std::Deps,
    red_bank_addr: &Addr,
    credit_manager_addr: &Addr,
    denom: &str,
) -> Result<Uint128, ContractError> {
    let res: mars_types::red_bank::UserDebtResponse = deps.querier.query_wasm_smart(
        red_bank_addr.to_string(),
        &QueryMsg::UserDebt {
            user: credit_manager_addr.to_string(),
            denom: denom.to_string(),
        },
    )?;
    Ok(res.amount)
}
