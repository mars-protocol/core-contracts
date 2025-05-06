use cosmwasm_std::{coins, BankMsg, CosmosMsg, DepsMut, Env, MessageInfo, Response, Uint128};
use mars_owner::OwnerInit;
use mars_types::{credit_manager, oracle};
use mars_utils::helpers::validate_native_denom;

use crate::{
    error::{ContractError, ContractResult},
    msg::InstantiateMsg,
    performance_fee::PerformanceFeeState,
    state::{
        BASE_TOKEN, COOLDOWN_PERIOD, CREDIT_MANAGER, DESCRIPTION, OWNER, PERFORMANCE_FEE_CONFIG,
        PERFORMANCE_FEE_STATE, SUBTITLE, TITLE, VAULT_TOKEN,
    },
    token_factory::TokenFactoryDenom,
    MIN_VAULT_FEE_CREATION_IN_UUSD,
};

pub fn init(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    let sent_base_token_amt = cw_utils::must_pay(&info, &msg.base_token)?;

    // initialize contract ownership info
    OWNER.initialize(
        deps.storage,
        deps.api,
        OwnerInit::SetInitialOwner {
            owner: info.sender.into(),
        },
    )?;

    // save credit manager address
    let credit_manager = deps.api.addr_validate(&msg.credit_manager)?;
    CREDIT_MANAGER.save(deps.storage, &credit_manager.to_string())?;

    // update contract metadata
    if let Some(title) = msg.title {
        TITLE.save(deps.storage, &title)?;
    }
    if let Some(subtitle) = msg.subtitle {
        SUBTITLE.save(deps.storage, &subtitle)?;
    }
    if let Some(desc) = msg.description {
        DESCRIPTION.save(deps.storage, &desc)?;
    }

    if msg.cooldown_period == 0 {
        return Err(ContractError::ZeroCooldownPeriod {});
    }

    COOLDOWN_PERIOD.save(deps.storage, &msg.cooldown_period)?;

    // initialize performance fee state
    msg.performance_fee_config.validate()?;
    PERFORMANCE_FEE_CONFIG.save(deps.storage, &msg.performance_fee_config)?;
    PERFORMANCE_FEE_STATE.save(deps.storage, &PerformanceFeeState::default())?;

    // initialize vault token
    let vault_token =
        TokenFactoryDenom::new(env.contract.address.to_string(), msg.vault_token_subdenom);
    VAULT_TOKEN.save(deps.storage, &vault_token)?;

    validate_native_denom(&msg.base_token)?;
    BASE_TOKEN.save(deps.storage, &msg.base_token)?;

    let msg = validate_base_token_value(
        &deps,
        credit_manager.as_ref(),
        &msg.base_token,
        sent_base_token_amt,
    )?;

    Ok(vault_token.instantiate()?.add_message(msg))
}

/// Validates the base token value to be greater than the minimum creation amount in uusd
fn validate_base_token_value(
    deps: &DepsMut,
    credit_manager: &str,
    base_token: &str,
    sent_base_token_amt: Uint128,
) -> ContractResult<CosmosMsg> {
    let config: credit_manager::ConfigResponse =
        deps.querier.query_wasm_smart(credit_manager, &credit_manager::QueryMsg::Config {})?;
    let price: oracle::PriceResponse = deps.querier.query_wasm_smart(
        config.oracle,
        &oracle::QueryMsg::Price {
            denom: base_token.to_string(),
            kind: None,
        },
    )?;
    let sent_base_token_value = sent_base_token_amt.checked_mul_floor(price.price)?;
    if sent_base_token_value < Uint128::from(MIN_VAULT_FEE_CREATION_IN_UUSD) {
        return Err(ContractError::MinAmountRequired {
            min_value: MIN_VAULT_FEE_CREATION_IN_UUSD,
            actual_value: sent_base_token_value.u128(),
            denom: base_token.to_string(),
        });
    }

    // It should never happen, but we check for it anyway
    let Some(rewards_collector) = config.rewards_collector else {
        // Return an error that rewards collector is not set
        return Err(ContractError::RewardsCollectorNotSet {});
    };

    let msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: rewards_collector.address,
        amount: coins(sent_base_token_amt.u128(), base_token),
    });
    Ok(msg)
}
