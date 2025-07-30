use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, StdResult};

#[cw_serde]
pub struct ActiveDeltaNeutralBase<T>(T);

impl<T> ActiveDeltaNeutralBase<T> {
    pub fn new(address: T) -> ActiveDeltaNeutralBase<T> {
        ActiveDeltaNeutralBase(address)
    }

    pub fn address(&self) -> &T {
        &self.0
    }
}

pub type ActiveDeltaNeutralUnchecked = ActiveDeltaNeutralBase<String>;
pub type ActiveDeltaNeutral = ActiveDeltaNeutralBase<Addr>;

impl From<ActiveDeltaNeutral> for ActiveDeltaNeutralUnchecked {
    fn from(active_delta_neutral: ActiveDeltaNeutral) -> Self {
        Self(active_delta_neutral.address().to_string())
    }
}

impl ActiveDeltaNeutralUnchecked {
    pub fn check(&self, api: &dyn Api) -> StdResult<ActiveDeltaNeutral> {
        Ok(ActiveDeltaNeutral::new(api.addr_validate(self.address())?))
    }
}
