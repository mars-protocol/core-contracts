use cosmwasm_std::{Reply, StdError, StdResult, SubMsgResult};

pub const INSTANTIATE_CREDIT_ACCOUNT_REPLY_ID: u64 = 1;

pub trait AttrParse {
    fn parse_create_credit_account_event(self) -> StdResult<String>;
}

impl AttrParse for Reply {
    fn parse_create_credit_account_event(self) -> StdResult<String> {
        match self.result {
            SubMsgResult::Err(err) => Err(StdError::generic_err(err)),
            SubMsgResult::Ok(response) => {
                let mut action = None;
                let mut token_id = None;
                for event in &response.events {
                    if event.ty == "wasm" {
                        for attr in &event.attributes {
                            if attr.key == "action" {
                                action = Some(attr.value.as_str());
                            }
                            if attr.key == "token_id" {
                                token_id = Some(attr.value.clone());
                            }
                        }
                    }
                    if action == Some("mint") {
                        return token_id.ok_or_else(|| {
                            StdError::generic_err("Missing token_id in mint event")
                        });
                    }
                }

                Ok(token_id.ok_or_else(|| StdError::generic_err("Missing token_id in mint event"))?)
            }
        }
    }
}
