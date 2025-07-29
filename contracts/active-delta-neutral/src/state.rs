use cw_storage_plus::Map;
use mars_delta_neutral_position::types::Position;
use mars_types::active_delta_neutral::query::Config;

// Market configuration denom always the index
// below are the states for each item we need for the markets
pub const CONFIG: Map<&str, Config> = Map::new("config");
pub const POSITION: Map<&str, Position> = Map::new("position");
