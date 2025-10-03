use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, Decimal, QuerierWrapper, StdResult};

use crate::credit_manager::{AccountTierAndDiscountResponse, QueryMsg};

#[cw_serde]
pub struct CreditManagerBase<T>(T);

impl<T> CreditManagerBase<T> {
    pub fn new(address: T) -> CreditManagerBase<T> {
        CreditManagerBase(address)
    }

    pub fn address(&self) -> &T {
        &self.0
    }
}

pub type CreditManagerUnchecked = CreditManagerBase<String>;
pub type CreditManager = CreditManagerBase<Addr>;

impl From<CreditManager> for CreditManagerUnchecked {
    fn from(cm: CreditManager) -> Self {
        Self(cm.address().to_string())
    }
}

impl CreditManagerUnchecked {
    pub fn check(&self, api: &dyn Api) -> StdResult<CreditManager> {
        Ok(CreditManagerBase::new(api.addr_validate(self.address())?))
    }
}

impl CreditManager {
    pub fn query_account_tier_and_discount(
        &self,
        querier: &QuerierWrapper,
        account_id: &str,
    ) -> StdResult<AccountTierAndDiscountResponse> {
        let res: AccountTierAndDiscountResponse = querier.query_wasm_smart(
            self.address(),
            &QueryMsg::GetAccountTierAndDiscount {
                account_id: account_id.to_string(),
            },
        )?;
        Ok(res)
    }

    pub fn query_discount_pct(
        &self,
        querier: &QuerierWrapper,
        account_id: &str,
    ) -> StdResult<Decimal> {
        Ok(self.query_account_tier_and_discount(querier, account_id)?.discount_pct)
    }
}
