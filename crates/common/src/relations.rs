use serde::{Serialize, Deserialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct RelationRecord {
    /// The source entity ID (String/Slug)
    pub from_id: String,

    /// The nature of the link (see predicates module)
    pub predicate: String,

    /// The target entity ID (String/Slug)
    pub to_id: String,

    /// Additional context (e.g., calculation coefficients)
    pub metadata: Value,
}

pub mod predicates {
    /// Used for Virtual Devices (e.g., Summing two meters)
    pub const CALCULATES_FOR: &str = "calculates_for";

    /// Used for topology (e.g., Device inside a Room)
    pub const IS_CHILD_OF: &str = "is_child_of";

    /// Used for automation/event chains
    pub const TRIGGERS: &str = "triggers";
}
