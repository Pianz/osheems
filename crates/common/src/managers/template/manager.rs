use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use walkdir::WalkDir;
use log::{info, error};

use crate::traits::TraitRegistry;
use super::super::template::{EntityTemplate, ProtocolMapping}; // Ajuste les imports selon ta structure de traits

pub struct TemplateManager {
    base_path: PathBuf,
    templates: RwLock<HashMap<String, EntityTemplate>>,
    traits: TraitRegistry,
}

impl TemplateManager {
    /// Initialise le gestionnaire et charge les templates depuis le disque
    pub fn new(base_path: PathBuf) -> Self {
        let manager = Self {
            base_path,
            templates: RwLock::new(HashMap::new()),
            traits: TraitRegistry::build(),
        };

        // Note: On ne peut pas appeler de méthode async ici si le constructeur est synchrone.
        // On fera un reload initial synchrone pour bloquer le démarrage jusqu'au chargement.
        manager.reload_sync();
        manager
    }

    /// Scan synchrone utilisé lors de l'initialisation
    fn reload_sync(&self) {
        info!("[TEMPLATE] Scanning templates in {:?}", self.base_path);
        let mut new_templates = HashMap::new();

        for entry in WalkDir::new(&self.base_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name() == "template.json")
            {
                match self.load_template(entry.path()) {
                    Ok(template) => {
                        info!("[TEMPLATE] Loaded: {} (v{}) with {} mappings",
                              template.template_id,
                              template.version,
                              template.mappings.len()
                        );
                        new_templates.insert(template.template_id.clone(), template);
                    }
                    Err(e) => {
                        error!("[TEMPLATE] Failed to load template at {:?}: {}", entry.path(), e);
                    }
                }
            }

            // On bloque juste le temps de l'écriture initiale
            if let Ok(mut lock) = self.templates.try_write() {
                *lock = new_templates;
            }
            info!("[TEMPLATE] Initial scan complete. {} templates active.", self.count_sync());
    }

    /// Version asynchrone pour recharger les templates à chaud (Hot Reload)
    pub async fn reload(&self) {
        // Logique similaire à reload_sync mais avec .write().await
        // ... (implémentation asynchrone pour plus tard si besoin)
    }

    fn load_template(&self, path: &Path) -> Result<EntityTemplate, String> {
        let content = fs::read_to_string(path)
        .map_err(|e| format!("Read error: {}", e))?;

        let mut template: EntityTemplate = serde_json::from_str(&content)
        .map_err(|e| format!("JSON parse error: {}", e))?;

        let parent_dir = path.parent().unwrap_or_else(|| Path::new("."));
        let mapping_dir = parent_dir.join("mappings");

        if mapping_dir.is_dir() {
            let mut mappings = HashMap::new();
            if let Ok(entries) = fs::read_dir(mapping_dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let p = entry.path();
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

        self.validate_traits(&template)?;
        Ok(template)
    }

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
