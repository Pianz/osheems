// crates/common/src/managers/device/manager.rs

use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use log::{info, error, warn};
use serde_json::{json, Value};

use crate::managers::database::DatabaseManager;
use crate::managers::template::TemplateManager;
use crate::managers::gateway::GatewayManager;
use crate::core_bus::manager::CoreBusManager;
use crate::core_bus::types::MqttSuffix;
use crate::entities::EntityType;

use super::DeviceInstance;

pub struct DeviceManager {
    db: Arc<DatabaseManager>,
    template_mgr: Arc<TemplateManager>,
    gateway_mgr: Arc<GatewayManager>,
    bus: Arc<CoreBusManager>,
    active_devices: RwLock<HashMap<String, DeviceInstance>>,
}

impl DeviceManager {
    pub fn new(
        db: Arc<DatabaseManager>,
        template_mgr: Arc<TemplateManager>,
        gateway_mgr: Arc<GatewayManager>,
        bus: Arc<CoreBusManager>,
    ) -> Self {
        Self {
            db,
            template_mgr,
            gateway_mgr,
            bus,
            active_devices: RwLock::new(HashMap::new()),
        }
    }

    /// Internal helper to push technical logs to the CoreBus
    async fn publish_log(&self, level: &str, id: &str, message: &str) {
        let payload_json = json!({
            "id": id,
            "level": level,
            "module": "device_manager",
            "msg": message,
            "ts": chrono::Utc::now().to_rfc3339()
        });

        if let Ok(payload) = serde_json::to_vec(&payload_json) {
            let sub_topic = format!("device/{}", level);
            self.bus.publish("osheems/core/system", MqttSuffix::Logs, &sub_topic, payload).await;
        }
    }

    /// Adds a device and establishes the "connected_to" relation with a gateway
    pub async fn add_device(
        &self,
        id: &str,
        template_id: &str,
        gateway_id: &str,
        label: &str,
        conn_params: Value
    ) -> Result<(), String> {
        // 1. Template validation
        let template = self.template_mgr.get_template(template_id).await
        .ok_or_else(|| format!("Template '{}' not found", template_id))?;

        // 2. Persist the Device entity
        let e_type_str = EntityType::Device.to_string();
        self.db.main.create_entity(
            id,
            &e_type_str,
            Some(template_id),
                                   Some(label),
                                   Some(&format!("Device {} based on {}", label, template.definition.identity.model)),
                                   &json!({}),
                                   &json!({}),
                                   false
        ).map_err(|e| e.to_string())?;

        // 3. Create the relation: Device -> [connected_to] -> Gateway
        // This is where we store specific connection IDs or addresses
        self.db.main.create_relation(
            id,
            "connected_to",
            gateway_id,
            &conn_params,
            false
        ).map_err(|e| e.to_string())?;

        info!("[DEVICE] Device '{}' created successfully", id);

        // 4. Notify the system via CoreBus
        let evt_payload = serde_json::to_vec(&json!({
            "id": id,
            "template_id": template_id,
            "gateway_id": gateway_id
        })).unwrap_or_default();

        self.bus.publish("osheems/core/system", MqttSuffix::Evt, "device/created", evt_payload).await;
        self.publish_log("info", id, "Device added and linked to gateway").await;

        Ok(())
    }

    /// Synchronizes with DB and starts all enabled devices
    pub async fn start(&self) -> Result<(), String> {
        info!("[DEVICE] Starting devices initialization...");

        let devices = self.db.main.get_all_entities(Some("device"))
        .map_err(|e| e.to_string())?;

        for d in devices {
            if d.is_enabled {
                info!("[DEVICE] Initializing device: {}", d.id);

                // Future logic:
                // 1. Fetch 'connected_to' relation to get gateway context
                // 2. Instantiate DeviceInstance with the correct parser
                // 3. Start data polling or subscription

                self.publish_log("info", &d.id, "Device enabled and initialized").await;
            } else {
                warn!("[DEVICE] Device {} is disabled, skipping.", d.id);
            }
        }
        Ok(())
    }
}
