use cw_storage_plus::{Item, Map};
use mars_delta_neutral_position::types::Position;
use mars_owner::Owner;
use mars_types::active_delta_neutral::query::{Config, MarketConfig};

// Market configuration denom always the index
// below are the states for each item we need for the markets
pub const OWNER: Owner = Owner::new("owner");
pub const CONFIG: Item<Config> = Item::new("config");
pub const MARKET_CONFIG: Map<&str, MarketConfig> = Map::new("market_config");
pub const POSITION: Map<&str, Position> = Map::new("position");
