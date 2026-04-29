// crates/common/src/managers/driver/manager.rs

use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use log::{info, warn};
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

    pub async fn start_driver(&self, gateway_id: &str) -> Result<ActiveDriver, String> {
        info!("[DRIVER] Assembling context for gateway: {}", gateway_id);

        let gateway_entity = self.db.main.get_entity(gateway_id)
        .map_err(|e| format!("Gateway entity not found: {}", e))?;

        let gateway_res = self.collect_resource_bundle(&gateway_entity).await?;
        let interface_res = self.collect_interface_for_gateway(gateway_id).await?;
        let devices_res = self.collect_devices_for_gateway(gateway_id).await?;

        let active_driver = ActiveDriver {
            gateway_id: gateway_id.to_string(),
            main_engine: gateway_res.engine_type,
            entity: gateway_res.clone(),
            gateway: gateway_res,
            interface: interface_res,
            devices_resources: devices_res,
        };

        let driver_to_return = active_driver.clone();
        let mut registry = self.active_drivers.write().await;
        registry.insert(gateway_id.to_string(), active_driver);

        info!("[DRIVER] '{}' is ready.", gateway_id);
        Ok(driver_to_return)
    }

    async fn collect_resource_bundle(&self, entity: &crate::entities::Entity) -> Result<ResourceBundle, String> {
        let template_id = entity.template_id.as_deref().unwrap_or("default");

        if let Some(template_full) = self.template_mgr.get_template(template_id).await {
            let mut final_config = serde_json::to_value(&template_full.definition.configuration).unwrap_or(json!({}));

            if let Some(entity_config_obj) = entity.configuration.as_object() {
                if let Some(final_obj) = final_config.as_object_mut() {
                    for (k, v) in entity_config_obj {
                        final_obj.insert(k.clone(), v.clone());
                    }
                }
            }

            let engine_type = if template_full.scripts.keys().any(|k| k.ends_with(".js")) {
                EngineType::JavaScript
            } else {
                EngineType::Rhai
            };

            Ok(ResourceBundle {
                template_id: template_id.to_string(),
               template: serde_json::to_value(&template_full.definition).unwrap_or(json!({})),
               engine_type,
               scripts: template_full.scripts.clone(),
               mappings: serde_json::to_value(&template_full.mappings).unwrap_or(json!({})),
               configuration: final_config,
            })
        } else {
            warn!("[DRIVER] Template '{}' not found, using virtual bundle.", template_id);
            let mut bundle = self.virtual_bundle(template_id);
            bundle.configuration = entity.configuration.clone();
            Ok(bundle)
        }
    }

    async fn collect_interface_for_gateway(&self, gateway_id: &str) -> Result<ResourceBundle, String> {
        let relations = self.db.main.get_all_relations(Some(gateway_id), Some("uses_interface"), None)
        .map_err(|e| e.to_string())?;

        if let Some(rel) = relations.first() {
            let entity = self.db.main.get_entity(&rel.to_id).map_err(|e| e.to_string())?;
            self.collect_resource_bundle(&entity).await
        } else {
            Ok(self.virtual_bundle("virtual_interface"))
        }
    }

    async fn collect_devices_for_gateway(&self, gateway_id: &str) -> Result<HashMap<String, ResourceBundle>, String> {
        let mut devices = HashMap::new();
        let relations = self.db.main.get_all_relations(None, Some("connected_to"), Some(gateway_id))
        .map_err(|e| e.to_string())?;

        for rel in relations {
            let entity = self.db.main.get_entity(&rel.from_id).map_err(|e| e.to_string())?;
            let mut bundle = self.collect_resource_bundle(&entity).await?;

            if let Some(obj) = bundle.configuration.as_object_mut() {
                // --- RÉTABLISSEMENT DE LA STRUCTURE RELATION_ATTRIBUTES ---
                // On injecte les attributs dans la clé attendue par le script
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
