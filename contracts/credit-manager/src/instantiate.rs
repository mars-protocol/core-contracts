use cosmwasm_std::{DepsMut, Env};
use mars_owner::OwnerInit::SetInitialOwner;
use mars_types::credit_manager::InstantiateMsg;

use crate::{
    error::ContractResult,
    state::{
        DUALITY_SWAPPER, HEALTH_CONTRACT, INCENTIVES, KEEPER_FEE_CONFIG, MAX_SLIPPAGE,
        MAX_TRIGGER_ORDERS, MAX_UNLOCKING_POSITIONS, ORACLE, OWNER, PARAMS, PERPS_LB_RATIO,
        RED_BANK, SWAPPER, SWAP_FEE, ZAPPER,
    },
    utils::{assert_max_slippage, assert_perps_lb_ratio},
};

pub fn store_config(deps: DepsMut, env: Env, msg: &InstantiateMsg) -> ContractResult<()> {
    OWNER.initialize(
        deps.storage,
        deps.api,
        SetInitialOwner {
            owner: msg.owner.clone(),
        },
    )?;

    RED_BANK.save(deps.storage, &msg.red_bank.check(deps.api, env.contract.address.clone())?)?;
    ORACLE.save(deps.storage, &msg.oracle.check(deps.api)?)?;
    SWAPPER.save(deps.storage, &msg.swapper.check(deps.api)?)?;
    DUALITY_SWAPPER.save(deps.storage, &msg.duality_swapper.check(deps.api)?)?;
    ZAPPER.save(deps.storage, &msg.zapper.check(deps.api)?)?;
    MAX_TRIGGER_ORDERS.save(deps.storage, &msg.max_trigger_orders)?;
    MAX_UNLOCKING_POSITIONS.save(deps.storage, &msg.max_unlocking_positions)?;

    assert_max_slippage(msg.max_slippage)?;
    MAX_SLIPPAGE.save(deps.storage, &msg.max_slippage)?;

    assert_perps_lb_ratio(msg.perps_liquidation_bonus_ratio)?;
    PERPS_LB_RATIO.save(deps.storage, &msg.perps_liquidation_bonus_ratio)?;

    HEALTH_CONTRACT.save(deps.storage, &msg.health_contract.check(deps.api)?)?;
    PARAMS.save(deps.storage, &msg.params.check(deps.api)?)?;
    INCENTIVES.save(deps.storage, &msg.incentives.check(deps.api, env.contract.address)?)?;
    KEEPER_FEE_CONFIG.save(deps.storage, &msg.keeper_fee_config)?;
    SWAP_FEE.save(deps.storage, &msg.swap_fee)?;

    Ok(())
}
