use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Trading direction for a single-sided position
///
/// Represents the direction of a trade in a single market (long or short)
/// This differs from Side which represents the combined direction in a delta-neutral position.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum Direction {
    /// Long position - buying the asset
    Long,

    /// Short position - selling the asset
    Short,
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Direction::Long => write!(f, "Long"),
            Direction::Short => write!(f, "Short"),
        }
    }
}
