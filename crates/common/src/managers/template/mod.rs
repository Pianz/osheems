pub mod manager;

pub use manager::TemplateManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// --- Modèles de données pour le Mapping (Technique) ---

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProtocolMapping {
    pub transport_config: serde_json::Value,
    pub points: HashMap<String, serde_json::Value>,
}

// --- Modèles de données pour le Template (Métier) ---

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DataPoint {
    pub id: String,
    pub r#trait: String,
    pub unit: Option<String>,
    pub r#type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TemplatePoints {
    pub states: Vec<DataPoint>,
    pub actions: Vec<DataPoint>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConfigField {
    pub r#type: String,
    pub default: serde_json::Value,
    pub description: String,
    pub only_for: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TemplateIdentity {
    pub brand: String,
    pub model: String,
    pub traits: Vec<String>,
    pub protocols: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EntityTemplate {
    pub template_id: String,
    pub version: String,
    pub entity_type: String,
    pub identity: TemplateIdentity,
    pub configuration: HashMap<String, ConfigField>,
    pub points: TemplatePoints,
    #[serde(default)]
    pub mappings: HashMap<String, ProtocolMapping>,
}
