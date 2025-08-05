use cosmwasm_std::{DepsMut, Reply, Response, StdError};
use mars_types::active_delta_neutral::{
    query::Config,
    reply::{AttrParse, INSTANTIATE_CREDIT_ACCOUNT_REPLY_ID},
};

use crate::{
    error::{ContractError, ContractResult},
    state::CONFIG,
};

pub fn reply(deps: DepsMut, reply: Reply) -> ContractResult<Response> {
    match reply.id {
        INSTANTIATE_CREDIT_ACCOUNT_REPLY_ID => {
            let token_id = reply.parse_create_credit_account_event()?;
            CONFIG.update(deps.storage, |mut config: Config| {
                config.credit_account_id = Some(token_id);
                Ok::<Config, ContractError>(config)
            })?;
            Ok(Response::new().add_attribute("action", "active_delta_neutral/reply"))
        }
        id => Err(ContractError::Std(StdError::generic_err(format!("Unknown reply ID: {}", id)))),
    }
}
