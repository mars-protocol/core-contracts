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
pub enum GovernanceQueryMsg {
    VotingPowerAtHeight(VotingPowerAtHeightQuery),
}

#[cw_serde]
pub struct GovernanceBase<T>(T);

impl<T> GovernanceBase<T> {
    pub fn new(address: T) -> GovernanceBase<T> {
        GovernanceBase(address)
    }

    pub fn address(&self) -> &T {
        &self.0
    }
}

pub type GovernanceUnchecked = GovernanceBase<String>;
pub type Governance = GovernanceBase<Addr>;

impl From<Governance> for GovernanceUnchecked {
    fn from(governance: Governance) -> Self {
        Self(governance.address().to_string())
    }
}

impl GovernanceUnchecked {
    pub fn check(&self, api: &dyn Api) -> StdResult<Governance> {
        Ok(GovernanceBase::new(api.addr_validate(self.address())?))
    }
}

impl Governance {
    pub fn query_voting_power_at_height(
        &self,
        querier: &QuerierWrapper,
        address: &str,
    ) -> StdResult<VotingPowerAtHeightResponse> {
        let query_msg = GovernanceQueryMsg::VotingPowerAtHeight(VotingPowerAtHeightQuery {
            address: address.to_string(),
        });

        querier.query_wasm_smart(self.address().to_string(), &query_msg)
    }
}
