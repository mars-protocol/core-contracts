use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;
use cw721_base::InstantiateMsg as ParentInstantiateMsg;

#[cw_serde]
pub struct InstantiateMsg {
    //--------------------------------------------------------------------------------------------------
    // Extended and overridden messages
    //--------------------------------------------------------------------------------------------------
    /// The maximum value of Debts + Collaterals (denominated in base token) for an account
    /// before burns are disallowed for the NFT. Meant to prevent accidental account deletions
    pub max_value_for_burn: Uint128,
    /// Used to query contract addresses to validate the account state in other contracts
    pub address_provider_contract: String,

    //--------------------------------------------------------------------------------------------------
    // Base cw721 messages
    //--------------------------------------------------------------------------------------------------
    /// Name of the NFT contract
    pub name: String,
    /// Symbol of the NFT contract
    pub symbol: String,
    /// The minter is the only one who can create new NFTs.
    /// Initially this likely will be the contract deployer. However, this role should be transferred
    /// through a config update to the Credit Manager. It is separate because some blockchains
    /// are permissioned and contracts go through governance and are instantiated separately.
    pub minter: String,
}

impl From<InstantiateMsg> for ParentInstantiateMsg {
    fn from(msg: InstantiateMsg) -> Self {
        Self {
            name: msg.name,
            symbol: msg.symbol,
            minter: msg.minter,
        }
    }
}
