use cosmwasm_schema::write_api;
use mars_swapper_duality::{config::DualityConfig, route::DualityRoute};
use mars_types::swapper::{ExecuteMsg, InstantiateMsg, QueryMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg<DualityRoute, DualityConfig>,
        query: QueryMsg,
    }
}
