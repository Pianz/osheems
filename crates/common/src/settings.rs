use serde::{Serialize, Deserialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Setting {
    pub entity_id: i64,
    pub key: String,
    pub value: Value,
    pub updated_at: Option<String>,
}

pub mod keys {
    // System
    pub const SYSTEM_NAME: &str = "system.name";
    pub const RETENTION_POLICY_DAYS: &str = "system.retention_days";

    // User
    pub const UI_THEME: &str = "ui.theme";
    pub const UI_LANGUAGE: &str = "ui.language";
}
