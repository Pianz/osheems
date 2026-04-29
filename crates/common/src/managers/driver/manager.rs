use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use log::{info, warn, error};
use serde_json::{json, Value};

use crate::managers::database::DatabaseManager;
use crate::managers::template::TemplateManager;
use super::{ActiveDriver, ResourceBundle, EngineType};

pub struct DriverManager {
    db: Arc<DatabaseManager>,
    template_mgr: Arc<TemplateManager>,
    active_drivers: RwLock<HashMap<String, ActiveDriver>>,
}

impl DriverManager {
    pub fn new(db: Arc<DatabaseManager>, template_mgr: Arc<TemplateManager>) -> Self {
        Self {
            db,
            template_mgr,
            active_drivers: RwLock::new(HashMap::new()),
        }
    }

    /// Démarre l'orchestration complète pour une Gateway et ses périphériques connectés
    pub async fn start_driver(&self, gateway_id: &str) -> Result<ActiveDriver, String> {
        info!("[DRIVER] Building full hierarchical context for gateway: {}", gateway_id);

        // 1. Récupération de l'entité Gateway
        let gateway_entity = self.db.main.get_entity(gateway_id)
        .map_err(|e| format!("Gateway entity not found: {}", e))?;

        // 2. Chargement des trois piliers de ressources
        let gateway_res = self.collect_resource_bundle(&gateway_entity).await?;
        let interface_res = self.collect_interface_for_gateway(gateway_id).await?;
        let devices_res = self.collect_devices_for_gateway(gateway_id).await?;

        // 3. Assemblage de l'ActiveDriver
        let active_driver = ActiveDriver {
            gateway_id: gateway_id.to_string(),
            main_engine: gateway_res.engine_type, // On se base sur l'engine de la gateway
            entity: gateway_res.clone(),         // La gateway est aussi une entité
            gateway: gateway_res,
            interface: interface_res,
            devices_resources: devices_res,
        };

        let driver_to_return = active_driver.clone();

        let mut registry = self.active_drivers.write().await;
        registry.insert(gateway_id.to_string(), active_driver);

        info!("[DRIVER] Context for '{}' is fully loaded with templates and relations", gateway_id);

        Ok(driver_to_return)
    }

    /// Charge le triptyque (template + mappings + scripts) pour une entité donnée
    async fn collect_resource_bundle(&self, entity: &crate::entities::Entity) -> Result<ResourceBundle, String> {
        let template_id = entity.template_id.as_deref().unwrap_or("default");
        let mut base_path = PathBuf::from("templates").join(template_id);

        // Fallback sur default si le dossier n'existe pas
        if !base_path.exists() {
            warn!("[DRIVER] Template '{}' not found, using 'default'", template_id);
            base_path = PathBuf::from("templates").join("default");
            if !base_path.exists() {
                return Err(format!("Critical: Template directory missing: {:?}", template_id));
            }
        }

        // --- A. Template.json ---
        let template_path = base_path.join("template.json");
        let template_val = if template_path.exists() {
            let content = std::fs::read_to_string(&template_path).map_err(|e| e.to_string())?;
            serde_json::from_str(&content).map_err(|e| e.to_string())?
        } else {
            json!({})
        };

        // --- B. Mappings ---
        let mut mappings = json!({});
        let mapping_dir = base_path.join("mappings");
        if mapping_dir.exists() {
            for entry in std::fs::read_dir(mapping_dir).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                    let name = entry.path().file_stem().unwrap().to_str().unwrap().to_string();
                    let content = std::fs::read_to_string(entry.path()).map_err(|e| e.to_string())?;
                    let val: Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
                    if let Some(obj) = mappings.as_object_mut() {
                        obj.insert(name, val);
                    }
                }
            }
        }

        // --- C. Scripts ---
        let mut scripts = HashMap::new();
        let mut engine_type = EngineType::Rhai;
        let script_dir = base_path.join("scripts");
        if script_dir.exists() {
            for entry in std::fs::read_dir(script_dir).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                let path = entry.path();
                if path.is_file() {
                    let ext = path.extension().and_then(|s| s.to_str());
                    if ext == Some("js") { engine_type = EngineType::JavaScript; }
                    let filename = path.file_name().unwrap().to_str().unwrap().to_string();
                    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
                    scripts.insert(filename, content);
                }
            }
        }

        Ok(ResourceBundle {
            template_id: template_id.to_string(),
           template: template_val,
           engine_type,
           scripts,
           mappings,
           configuration: entity.configuration.clone(),
        })
    }

    /// Récupère l'interface liée à la gateway (relation uses_interface)
    async fn collect_interface_for_gateway(&self, gateway_id: &str) -> Result<ResourceBundle, String> {
        let relations = self.db.main.get_all_relations(Some(gateway_id), Some("uses_interface"), None)
        .map_err(|e| e.to_string())?;

        if let Some(rel) = relations.first() {
            let entity = self.db.main.get_entity(&rel.to_id).map_err(|e| e.to_string())?;
            self.collect_resource_bundle(&entity).await
        } else {
            warn!("[DRIVER] No interface relation for {}, using virtual", gateway_id);
            Ok(self.virtual_bundle("virtual_interface"))
        }
    }

    /// Récupère tous les devices connectés à cette gateway
    async fn collect_devices_for_gateway(&self, gateway_id: &str) -> Result<HashMap<String, ResourceBundle>, String> {
        let mut devices = HashMap::new();
        let relations = self.db.main.get_all_relations(None, Some("connected_to"), Some(gateway_id))
        .map_err(|e| e.to_string())?;

        for rel in relations {
            let entity = self.db.main.get_entity(&rel.from_id).map_err(|e| e.to_string())?;

            // Injection des attributs de relation dans la config pour que le driver y ait accès
            let mut bundle = self.collect_resource_bundle(&entity).await?;
            if let Some(obj) = bundle.configuration.as_object_mut() {
                obj.insert("relation_attributes".to_string(), rel.attributes.clone());
            }

            devices.insert(rel.from_id, bundle);
        }
        Ok(devices)
    }

    fn virtual_bundle(&self, id: &str) -> ResourceBundle {
        ResourceBundle {
            template_id: id.to_string(),
            template: json!({}),
            engine_type: EngineType::Rhai,
            scripts: HashMap::new(),
            mappings: json!({}),
            configuration: json!({}),
        }
    }
}
