use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use log::{info, error, warn};
use walkdir::WalkDir;

use crate::traits::TraitRegistry;

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
    pub r#type: Option<String>, // Principalement pour les actions (bool, number, etc.)
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
    pub config: HashMap<String, ConfigField>,
    pub points: TemplatePoints,

    // Mappings chargés dynamiquement depuis le sous-dossier mapping/
    #[serde(default)]
    pub mappings: HashMap<String, ProtocolMapping>,
}

// --- Le Gestionnaire de Templates ---

pub struct TemplateManager {
    base_path: PathBuf,
    templates: HashMap<String, EntityTemplate>,
    traits: TraitRegistry,
}

impl TemplateManager {
    /// Initializes the manager and loads templates from disk
    pub fn new(base_path: PathBuf) -> Self {
        let mut manager = Self {
            base_path,
            templates: HashMap::new(),
            traits: TraitRegistry::build(),
        };

        manager.reload_all();
        manager
    }

    /// Recursively scans the directory to load template.json files and their mappings
    pub fn reload_all(&mut self) {
        info!("OSHEEMS: Scanning templates in {:?}", self.base_path);
        let mut new_templates = HashMap::new();

        for entry in WalkDir::new(&self.base_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name() == "template.json")
            {
                match self.load_template(entry.path()) {
                    Ok(template) => {
                        info!("Loaded template: {} (v{}) with {} mappings",
                              template.template_id,
                              template.version,
                              template.mappings.len()
                        );
                        new_templates.insert(template.template_id.clone(), template);
                    }
                    Err(e) => {
                        error!("Failed to load template at {:?}: {}", entry.path(), e);
                    }
                }
            }

            self.templates = new_templates;
            info!("OSHEEMS: {} templates currently active", self.templates.len());
    }

    /// Loads, deserializes, and validates a single template along with its mappings
    fn load_template(&self, path: &Path) -> Result<EntityTemplate, String> {
        let content = fs::read_to_string(path)
        .map_err(|e| format!("Read error: {}", e))?;

        let mut template: EntityTemplate = serde_json::from_str(&content)
        .map_err(|e| format!("JSON parse error: {}", e))?;

        // --- Automatic Mapping Discovery ---
        let parent_dir = path.parent().unwrap_or_else(|| Path::new("."));
        let mapping_dir = parent_dir.join("mappings");

        if mapping_dir.is_dir() {
            let mut mappings = HashMap::new();

            if let Ok(entries) = fs::read_dir(mapping_dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let p = entry.path();

                    // Only process .json files in the mapping folder
                    if p.extension().and_then(|s| s.to_str()) == Some("json") {
                        let protocol_name = p.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();

                        let map_content = fs::read_to_string(&p)
                        .map_err(|e| format!("Mapping read error ({}): {}", protocol_name, e))?;

                        let mapping: ProtocolMapping = serde_json::from_str(&map_content)
                        .map_err(|e| format!("Mapping JSON error in {}: {}", protocol_name, e))?;

                        mappings.insert(protocol_name, mapping);
                    }
                }
            }
            template.mappings = mappings;
        }

        // Semantic Validation
        self.validate_traits(&template)?;

        Ok(template)
    }

    /// Ensures all traits used in the template exist in the OSHEEMS TraitRegistry
    fn validate_traits(&self, template: &EntityTemplate) -> Result<(), String> {
        for point in &template.points.states {
            if !self.traits.exists(&point.r#trait) {
                return Err(format!("Unknown trait '{}' in states of {}", point.r#trait, template.template_id));
            }
        }

        for point in &template.points.actions {
            if !self.traits.exists(&point.r#trait) {
                return Err(format!("Unknown trait '{}' in actions of {}", point.r#trait, template.template_id));
            }
        }

        Ok(())
    }

    // --- Accessors ---

    pub fn get_template(&self, id: &str) -> Option<&EntityTemplate> {
        self.templates.get(id)
    }

    pub fn list_templates(&self) -> Vec<&EntityTemplate> {
        self.templates.values().collect()
    }
}
