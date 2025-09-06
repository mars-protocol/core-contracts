use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, QuerierWrapper, StdResult, Uint128};

#[cw_serde]
pub struct VotingPowerAtHeightQuery {
    pub address: String,
}

#[cw_serde]
pub struct VotingPowerAtHeightResponse {
    pub power: Uint128,
    pub height: u64,
}

#[cw_serde]
pub enum DaoStakingQueryMsg {
    VotingPowerAtHeight(VotingPowerAtHeightQuery),
}

#[cw_serde]
pub struct DaoStakingBase<T>(T);

impl<T> DaoStakingBase<T> {
    pub fn new(address: T) -> DaoStakingBase<T> {
        DaoStakingBase(address)
    }

    pub fn address(&self) -> &T {
        &self.0
    }
}

pub type DaoStakingUnchecked = DaoStakingBase<String>;
pub type DaoStaking = DaoStakingBase<Addr>;

impl From<DaoStaking> for DaoStakingUnchecked {
    fn from(dao_staking: DaoStaking) -> Self {
        Self(dao_staking.address().to_string())
    }
}

impl DaoStakingUnchecked {
    pub fn check(&self, api: &dyn Api) -> StdResult<DaoStaking> {
        Ok(DaoStakingBase::new(api.addr_validate(self.address())?))
    }
}

impl DaoStaking {
    pub fn query_voting_power_at_height(
        &self,
        querier: &QuerierWrapper,
        address: &str,
    ) -> StdResult<VotingPowerAtHeightResponse> {
        let query_msg = DaoStakingQueryMsg::VotingPowerAtHeight(VotingPowerAtHeightQuery {
            address: address.to_string(),
        });

        querier.query_wasm_smart(self.address().to_string(), &query_msg)
    }
}
