// crates/common/src/managers/template/mod.rs

pub mod manager;

pub use manager::TemplateManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// --- 1. Miroir du dossier 'mappings/' (Technique) ---

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProtocolMapping {
    pub transport_config: serde_json::Value,
    pub points: HashMap<String, serde_json::Value>,
}

// --- 2. Miroir du fichier 'template.json' (Métier / Définition) ---

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

/// Représente le contenu strict du fichier template.json
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TemplateDefinition {
    pub template_id: String,
    pub version: String,
    pub entity_type: String,
    pub identity: TemplateIdentity,
    pub configuration: HashMap<String, ConfigField>,
    pub points: TemplatePoints,
}

// --- 3. L'Objet Global : EntityTemplate (Le Dossier) ---

/// Conteneur racine représentant un dossier de template complet
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EntityTemplate {
    /// Le contenu du fichier template.json
    pub definition: TemplateDefinition,

    /// Le contenu du dossier mappings/ (Clé = nom du protocole, ex: "mqtt")
    #[serde(default)]
    pub mappings: HashMap<String, ProtocolMapping>,

    /// Le contenu du dossier scripts/ (Clé = nom du fichier, ex: "main.rhai")
    #[serde(default)]
    pub scripts: HashMap<String, String>,
}
