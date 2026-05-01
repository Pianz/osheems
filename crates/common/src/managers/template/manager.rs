// crates/common/src/managers/template/manager.rs

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tokio::sync::RwLock;
use walkdir::WalkDir;
use log::{info, error};

use crate::traits::TraitRegistry;
use super::{EntityTemplate, ProtocolMapping, TemplateDefinition};

pub struct TemplateManager {
    base_path: PathBuf,
    templates: RwLock<HashMap<String, EntityTemplate>>,
    traits: TraitRegistry,
}

impl TemplateManager {
    pub fn new(base_path: PathBuf) -> Self {
        let manager = Self {
            base_path,
            templates: RwLock::new(HashMap::new()),
            traits: TraitRegistry::build(),
        };

        manager.reload_sync();
        manager
    }

    fn reload_sync(&self) {
        info!("[TEMPLATE] Scanning templates in {:?}", self.base_path);
        let mut new_templates = HashMap::new();

        for entry in WalkDir::new(&self.base_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name() == "template.json")
            {
                let path = entry.path();

                // 1. Calcul de la clé relative
                let relative_key = if let Ok(rel_path) = path.parent().unwrap().strip_prefix(&self.base_path) {
                    // Normalisation : On remplace les antislashes Windows par des slashes et on enlève les bords
                    let s = rel_path.to_string_lossy().replace("\\", "/");
                    let cleaned = s.trim_matches('/');

                    if cleaned.is_empty() { "default".to_string() } else { cleaned.to_string() }
                } else {
                    "default".to_string()
                };

                match self.load_template(path) {
                    Ok(template) => {
                        // Cette ligne s'affichera avec RUST_LOG=info
                        info!("[TEMPLATE] Registered: '{}' (v{}) | Mappings: {} | Scripts: {}",
                              relative_key,
                              template.definition.version,
                              template.mappings.len(),
                              template.scripts.len()
                        );
                        new_templates.insert(relative_key, template);
                    }
                    Err(e) => {
                        error!("[TEMPLATE] Error at {:?}: {}", path, e);
                    }
                }
            }

            if let Ok(mut lock) = self.templates.try_write() {
                *lock = new_templates;
            }
            info!("[TEMPLATE] Scan complete. {} templates active.", self.count_sync());
    }

    fn load_template(&self, path: &Path) -> Result<EntityTemplate, String> {
        let parent_dir = path.parent().unwrap_or_else(|| Path::new("."));

        // 1. Chargement de la Definition (template.json)
        let def_content = fs::read_to_string(path)
        .map_err(|e| format!("Read error (template.json): {}", e))?;
        let definition: TemplateDefinition = serde_json::from_str(&def_content)
        .map_err(|e| format!("JSON error (template.json): {}", e))?;

        // 2. Chargement des Mappings (dossier /mappings)
        let mut mappings = HashMap::new();
        let mapping_dir = parent_dir.join("mappings");
        if mapping_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(mapping_dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let p = entry.path();
                    if p.extension().and_then(|s| s.to_str()) == Some("json") {
                        let name = p.file_stem().unwrap().to_str().unwrap().to_string();
                        let content = fs::read_to_string(&p).map_err(|e| e.to_string())?;
                        let mapping: ProtocolMapping = serde_json::from_str(&content).map_err(|e| e.to_string())?;
                        mappings.insert(name, mapping);
                    }
                }
            }
        }

        // 3. Chargement des Scripts (dossier /scripts)
        let mut scripts = HashMap::new();
        let scripts_dir = parent_dir.join("scripts");
        if scripts_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(scripts_dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let p = entry.path();
                    let ext = p.extension().and_then(|s| s.to_str());
                    if ext == Some("rhai") || ext == Some("js") {
                        let name = p.file_name().unwrap().to_str().unwrap().to_string();
                        let content = fs::read_to_string(&p).map_err(|e| e.to_string())?;
                        scripts.insert(name, content);
                    }
                }
            }
        }

        let template = EntityTemplate {
            definition,
            mappings,
            scripts,
        };

        // Validation par rapport au registre des traits
        self.validate_traits(&template)?;

        Ok(template)
    }

    fn validate_traits(&self, template: &EntityTemplate) -> Result<(), String> {
        for point in &template.definition.points.states {
            if !self.traits.exists(&point.r#trait) {
                return Err(format!("Unknown trait '{}' in states for template at {:?}", point.r#trait, template.definition.template_id));
            }
        }
        for point in &template.definition.points.actions {
            if !self.traits.exists(&point.r#trait) {
                return Err(format!("Unknown trait '{}' in actions for template at {:?}", point.r#trait, template.definition.template_id));
            }
        }
        Ok(())
    }

    // --- Accesseurs ---

    pub async fn get_template(&self, id: &str) -> Option<EntityTemplate> {
        let lock = self.templates.read().await;
        lock.get(id).cloned()
    }

    pub async fn list_templates(&self) -> Vec<EntityTemplate> {
        let lock = self.templates.read().await;
        lock.values().cloned().collect()
    }

    fn count_sync(&self) -> usize {
        if let Ok(lock) = self.templates.try_read() {
            lock.len()
        } else {
            0
        }
    }
}
