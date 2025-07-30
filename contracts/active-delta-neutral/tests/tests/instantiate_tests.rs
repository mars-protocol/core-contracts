// #[cfg(test)]
// mod tests {
//     use cosmwasm_std::{Addr, Empty, StdError};
//     use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};
//     use crate::contract::{instantiate, execute, query};
//     use crate::msg::InstantiateMsg;

//     fn mock_app() -> App {
//         AppBuilder::new().build(|router, _, storage| {
//             // Initialize any necessary state or contracts here
//         })
//     }

//     fn contract_template() -> Box<dyn Contract<Empty>> {
//         let contract = ContractWrapper::new(execute, instantiate, query);
//         Box::new(contract)
//     }

//     #[test]
//     fn test_instantiate() {
//         let mut app = mock_app();

//         // Store the contract code
//         let code_id = app.store_code(contract_template());

//         // Define test cases
//         let test_cases = vec![
//             ("valid_address_provider", "valid_credit_manager_address", true),
//             ("invalid_address_provider", "valid_credit_manager_address", false),
//             ("valid_address_provider", "invalid_credit_manager_address", false),
//         ];

//         for (address_provider, credit_manager_address, should_succeed) in test_cases {
//             // Instantiate the contract
//             let msg = InstantiateMsg {
//                 address_provider: address_provider.to_string(),
//             };

//             let contract_addr = app
//                 .instantiate_contract(
//                     code_id,
//                     Addr::unchecked("creator"),
//                     &msg,
//                     &[],
//                     "Test Contract",
//                     None,
//                 )
//                 .unwrap();
//             // Mock the query response for the address provider
//             app.wrap().with_wasm_handler(|query| {
//                 match query {
//                     cosmwasm_std::QueryRequest::Wasm(cosmwasm_std::WasmQuery::Smart { contract_addr, .. }) => {
//                         if contract_addr == address_provider {
//                             Ok(cosmwasm_std::to_json_binary(&credit_manager_address)?)
//                         } else {
//                             Err(StdError::generic_err("Address provider not found"))
//                         }
//                     }
//                     _ => Err(StdError::generic_err("Unsupported query")),
//                 }
//             });

//             // Assert the result based on the expected outcome
//             if should_succeed {
//                 let response = app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
//                 assert_eq!(response.attributes.len(), 3);
//                 assert_eq!(response.attributes[1].value, address_provider);
//                 assert_eq!(response.attributes[2].value, credit_manager_address);
//             } else {
//                 let result = app.wrap().query_wasm_smart(contract_addr, &msg);
//                 assert!(result.is_err(), "Expected error but got success");
//             }
//         }
//     }
// }
