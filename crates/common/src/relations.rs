use serde::{Serialize, Deserialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct RelationRecord {
    pub from_id: i64,
    pub predicate: String,
    pub to_id: i64,
    pub metadata: Value,
}

pub mod predicates {
    pub const CALCULATES_FOR: &str = "calculates_for";
    pub const IS_CHILD_OF: &str = "is_child_of";
    pub const TRIGGERS: &str = "triggers";
}
