// crates/common/src/managers/gateway/manager.rs

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use log::{info, error};
use serde_json::{json, Value};

use crate::managers::database::DatabaseManager;
use crate::core_bus::manager::CoreBusManager;
use crate::core_bus::types::MqttSuffix;
use crate::entities::{Entity, EntityType};

pub struct GatewayManager {
    db_manager: Arc<DatabaseManager>,
    bus: Arc<CoreBusManager>,
    // Operational state registry (ID -> Status JSON)
    states: RwLock<HashMap<String, Value>>,
}

impl GatewayManager {
    pub fn new(db_manager: Arc<DatabaseManager>, bus: Arc<CoreBusManager>) -> Self {
        Self {
            db_manager,
            bus,
            states: RwLock::new(HashMap::new()),
        }
    }

    // Helper to publish logs to CoreBus (technical traces)
    async fn publish_log(&self, level: &str, id: &str, message: &str) {
        let payload_json = json!({
            "id": id,
            "level": level,
            "module": "gateway_manager",
            "msg": message,
            "ts": chrono::Utc::now().to_rfc3339()
        });

        if let Ok(payload) = serde_json::to_vec(&payload_json) {
            let sub_topic = format!("gateway/{}", level);
            // Updated to MqttSuffix::Logs per your enum definition
            self.bus.publish("osheems/core/system", MqttSuffix::Logs, &sub_topic, payload).await;
        }
    }

    // --- READ ---

    pub fn get_all(&self) -> Result<Vec<Entity>, String> {
        match self.db_manager.main.get_all_entities(Some("gateway")) {
            Ok(entities) => {
                info!("[GATEWAY] Retrieved {} gateway(s)", entities.len());
                Ok(entities)
            }
            Err(e) => {
                error!("[GATEWAY] Failed to retrieve gateways: {}", e);
                Err(e.to_string())
            }
        }
    }

    pub fn get_by_id(&self, id: &str) -> Result<Entity, String> {
        self.db_manager.main.get_entity(id).map_err(|e| e.to_string())
    }

    // --- CREATE / UPDATE ---

    pub async fn create(&self, entity: Entity) -> Result<(), String> {
        if entity.entity_type != EntityType::Gateway {
            let err_msg = "Type mismatch: expected Gateway";
            self.publish_log("error", &entity.id, err_msg).await;
            return Err(err_msg.to_string());
        }

        let id = entity.id.clone();
        let e_type_str = entity.entity_type.to_string();

        match self.db_manager.main.create_entity(
            &entity.id,
            &e_type_str,
            entity.template_id.as_deref(),
                                                 entity.label.as_deref(),
                                                 entity.description.as_deref(),
                                                 &entity.configuration,
                                                 &entity.attributes,
                                                 entity.is_system
        ) {
            Ok(_) => {
                info!("[GATEWAY] Created: {}", id);

                let evt_payload = serde_json::to_vec(&json!({ "id": id })).unwrap_or_default();
                self.bus.publish("osheems/core/system", MqttSuffix::Evt, "gateway/created", evt_payload).await;

                self.publish_log("info", &id, "Gateway successfully created in database").await;
                Ok(())
            }
            Err(e) => {
                let err_msg = e.to_string();
                self.publish_log("error", &id, &err_msg).await;
                Err(err_msg)
            }
        }
    }

    pub async fn update(&self, entity: Entity) -> Result<(), String> {
        let id = entity.id.clone();

        match self.db_manager.main.update_entity(
            &entity.id,
            entity.label.as_deref(),
                                                 entity.description.as_deref(),
                                                 &entity.configuration,
                                                 &entity.attributes,
                                                 entity.is_enabled
        ) {
            Ok(_) => {
                info!("[GATEWAY] Updated: {}", id);

                let evt_payload = serde_json::to_vec(&json!({ "id": id })).unwrap_or_default();
                self.bus.publish("osheems/core/system", MqttSuffix::Evt, "gateway/updated", evt_payload).await;

                self.publish_log("info", &id, "Gateway record updated in database").await;
                Ok(())
            }
            Err(e) => {
                let err_msg = e.to_string();
                self.publish_log("error", &id, &err_msg).await;
                Err(err_msg)
            }
        }
    }

    // --- DELETE ---

    pub async fn delete(&self, id: &str) -> Result<(), String> {
        match self.db_manager.main.delete_entity(id) {
            Ok(_) => {
                info!("[GATEWAY] Deleted: {}", id);

                let evt_payload = serde_json::to_vec(&json!({ "id": id })).unwrap_or_default();
                self.bus.publish("osheems/core/system", MqttSuffix::Evt, "gateway/deleted", evt_payload).await;

                self.publish_log("info", id, "Gateway successfully removed from database").await;

                if let Ok(mut registry) = self.states.try_write() {
                    registry.remove(id);
                }
                Ok(())
            }
            Err(e) => {
                let err_msg = e.to_string();
                self.publish_log("error", id, &err_msg).await;
                Err(err_msg)
            }
        }
    }

    // --- STATE MONITORING ---

    pub async fn update_state(&self, id: String, state: Value) {
        let mut registry = self.states.write().await;
        registry.insert(id, state);
    }

    pub async fn get_state(&self, id: &str) -> Value {
        let registry = self.states.read().await;
        registry.get(id).cloned().unwrap_or(json!({"status": "unknown"}))
    }
}
