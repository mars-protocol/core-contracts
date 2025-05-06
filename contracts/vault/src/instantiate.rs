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
    let sent_base_token_amt =
        info.funds.iter().find(|c| c.denom == msg.base_token).map(|c| c.amount).unwrap_or_default();

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

    let config: credit_manager::ConfigResponse = deps
        .querier
        .query_wasm_smart(credit_manager.as_ref(), &credit_manager::QueryMsg::Config {})?;

    validate_base_token_value(&deps, &config, &msg.base_token, sent_base_token_amt)?;
    let rc_msg = prepare_rewards_collector_msg(&config, &msg.base_token, sent_base_token_amt)?;

    Ok(vault_token.instantiate()?.add_message(rc_msg))
}

/// Validates the base token value to be greater than the minimum creation amount in uusd
fn validate_base_token_value(
    deps: &DepsMut,
    config: &credit_manager::ConfigResponse,
    base_token: &str,
    sent_base_token_amt: Uint128,
) -> ContractResult<()> {
    let price: oracle::PriceResponse = deps.querier.query_wasm_smart(
        config.oracle.clone(),
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
    Ok(())
}

/// Prepares a message to send the base token to the rewards collector
fn prepare_rewards_collector_msg(
    config: &credit_manager::ConfigResponse,
    base_token: &str,
    sent_base_token_amt: Uint128,
) -> ContractResult<CosmosMsg> {
    // It should never happen, but we check for it anyway
    let Some(rewards_collector) = &config.rewards_collector else {
        // Return an error that rewards collector is not set
        return Err(ContractError::RewardsCollectorNotSet {});
    };

    let msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: rewards_collector.address.clone(),
        amount: coins(sent_base_token_amt.u128(), base_token),
    });
    Ok(msg)
}
