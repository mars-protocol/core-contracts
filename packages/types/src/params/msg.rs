use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal, Uint128};
use mars_owner::OwnerUpdate;

use super::{asset::AssetParamsUnchecked, vault::VaultConfigUnchecked, PerpParams};

#[cw_serde]
pub struct InstantiateMsg {
    /// Contract's owner
    pub owner: String,
    /// Contracts optional risk manager
    pub risk_manager: Option<String>,
    /// Address of the address provider contract
    pub address_provider: String,
    /// Maximum number of perps that can be created
    pub max_perp_params: u8,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateOwner(OwnerUpdate),
    UpdateRiskManager(OwnerUpdate),
    ResetRiskManager(),
    UpdateConfig {
        address_provider: Option<String>,
        max_perp_params: Option<u8>,
    },
    UpdateAssetParams(AssetParamsUpdate),
    UpdateVaultConfig(VaultConfigUpdate),
    UpdatePerpParams(PerpParamsUpdate),
    EmergencyUpdate(EmergencyUpdate),
    UpdateManagedVaultConfig(ManagedVaultUpdate),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(mars_owner::OwnerResponse)]
    Owner {},

    #[returns(mars_owner::OwnerResponse)]
    RiskManager {},

    #[returns(super::msg::ConfigResponse)]
    Config {},

    #[returns(super::msg::ManagedVaultConfigResponse)]
    ManagedVaultConfig {},

    #[returns(Option<super::asset::AssetParams>)]
    AssetParams {
        denom: String,
    },

    #[returns(Vec<super::asset::AssetParams>)]
    AllAssetParams {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    #[returns(cw_paginate::PaginationResponse<super::asset::AssetParams>)]
    AllAssetParamsV2 {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    #[returns(super::vault::VaultConfig)]
    VaultConfig {
        /// Address of vault
        address: String,
    },

    #[returns(Vec<super::vault::VaultConfig>)]
    AllVaultConfigs {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    #[returns(cw_paginate::PaginationResponse<super::vault::VaultConfig>)]
    AllVaultConfigsV2 {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    #[returns(super::perp::PerpParams)]
    PerpParams {
        denom: String,
    },

    #[returns(Vec<super::perp::PerpParams>)]
    AllPerpParams {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    #[returns(cw_paginate::PaginationResponse<super::perp::PerpParams>)]
    AllPerpParamsV2 {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    /// Compute the total amount deposited of the given asset across Red Bank
    /// and Credit Manager.
    #[returns(TotalDepositResponse)]
    TotalDeposit {
        denom: String,
    },

    /// Compute the total amount deposited for paginated assets across Red Bank
    /// and Credit Manager.
    #[returns(cw_paginate::PaginationResponse<TotalDepositResponse>)]
    AllTotalDepositsV2 {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct ConfigResponse {
    /// Address provider returns addresses for all protocol contracts
    pub address_provider: String,
    /// Maximum number of perps that can be created
    pub max_perp_params: u8,
}

#[cw_serde]
pub struct ManagedVaultConfigResponse {
    /// Minimum creation fee in uusd for managed vaults
    pub min_creation_fee_in_uusd: u128,
    /// List of code ids for managed vaults
    pub code_ids: Vec<u32>,
}

#[cw_serde]
pub struct TotalDepositResponse {
    pub denom: String,
    pub cap: Uint128,
    pub amount: Uint128,
}

#[cw_serde]
pub enum AssetParamsUpdate {
    AddOrUpdate {
        params: AssetParamsUnchecked,
    },
}

#[cw_serde]
pub enum VaultConfigUpdate {
    AddOrUpdate {
        config: VaultConfigUnchecked,
    },
}

#[cw_serde]
pub enum PerpParamsUpdate {
    AddOrUpdate {
        params: PerpParams,
    },
}

#[cw_serde]
pub enum CmEmergencyUpdate {
    SetZeroMaxLtvOnVault(String),
    SetZeroDepositCapOnVault(String),
    DisallowCoin(String),
    DisableWithdraw(String),
}

#[cw_serde]
pub enum RedBankEmergencyUpdate {
    DisableBorrowing(String),
    DisableWithdraw(String),
}

#[cw_serde]
pub enum PerpsEmergencyUpdate {
    DisableTrading(String),
    DisableDeleverage(),
    DisableCounterpartyVaultWithdraw(),
}

#[cw_serde]
pub enum EmergencyUpdate {
    CreditManager(CmEmergencyUpdate),
    RedBank(RedBankEmergencyUpdate),
    Perps(PerpsEmergencyUpdate),
}

#[cw_serde]
pub enum ManagedVaultUpdate {
    AddCodeId(u32),
    RemoveCodeId(u32),
    SetMinCreationFeeInUusd(u128),
}

#[cw_serde]
pub struct MigrateMsg {
    pub close_factor: Decimal,
}
