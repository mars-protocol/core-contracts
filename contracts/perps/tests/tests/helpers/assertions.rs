#![allow(dead_code)] // TODO: remove once functions are used
use anyhow::Result as AnyResult;
use cw_multi_test::AppResponse;
use mars_perps::error::ContractError;

pub fn assert_err(res: AnyResult<AppResponse>, err: ContractError) {
    match res {
        Ok(_) => panic!("Result was not an error"),
        Err(generic_err) => {
            let contract_err: ContractError = generic_err.downcast().unwrap();
            assert_eq!(contract_err, err);
        }
    }
}
