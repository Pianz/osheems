pub mod manager;
pub use manager::DriverManager;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum EngineType {
    Rhai,
    JavaScript,
}

#[derive(Debug, Clone)]
pub struct ResourceBundle {
    pub template_id: String,
    pub template: Value,
    pub engine_type: EngineType,
    pub scripts: HashMap<String, String>,
    pub mappings: Value,
    pub configuration: Value,
}

#[derive(Debug, Clone)]
pub struct ActiveDriver {
    pub gateway_id: String,
    pub main_engine: EngineType,
    pub entity: ResourceBundle,
    pub gateway: ResourceBundle,
    pub interface: ResourceBundle,
    pub devices_resources: HashMap<String, ResourceBundle>,
}
