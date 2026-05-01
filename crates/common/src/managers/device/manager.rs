use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use log::info; // On garde seulement info pour éviter le warning
use serde_json::{json, Value};

use crate::managers::database::DatabaseManager;
use crate::managers::template::TemplateManager;
use crate::managers::gateway::GatewayManager;
// Import manquant pour DeviceInstance
use super::DeviceInstance;

pub struct DeviceManager {
    db: Arc<DatabaseManager>,
    template_mgr: Arc<TemplateManager>,
    gateway_mgr: Arc<GatewayManager>,
    active_devices: RwLock<HashMap<String, DeviceInstance>>,
}

impl DeviceManager {
    pub fn new(
        db: Arc<DatabaseManager>,
        template_mgr: Arc<TemplateManager>,
        gateway_mgr: Arc<GatewayManager>
    ) -> Self {
        Self {
            db,
            template_mgr,
            gateway_mgr,
            active_devices: RwLock::new(HashMap::new()),
        }
    }

    pub async fn add_device(
        &self,
        id: &str,
        template_id: &str,
        gateway_id: &str,
        label: &str,
        conn_params: Value
    ) -> Result<(), String> {

        let template = self.template_mgr.get_template(template_id).await
        .ok_or_else(|| format!("Template '{}' not found", template_id))?;

        self.db.main.create_entity(
            id,
            "device",
            Some(template_id),
                                   Some(label),
                                   Some(&format!("Device {} based on {}", label, template.definition.identity.model)),
                                   &json!({}),
                                   &json!({}),
                                   false
        ).map_err(|e| e.to_string())?;

        self.db.main.create_relation(
            id,
            "connected_to",
            gateway_id,
            &conn_params,
            false
        ).map_err(|e| e.to_string())?;

        info!("[DEVICE] Device '{}' created successfully", id);

        Ok(())
    }

    pub async fn start(&self) -> Result<(), String> {
        let devices = self.db.main.get_all_entities(Some("device"))
        .map_err(|e| e.to_string())?;

        for d in devices {
            if d.is_enabled {
                info!("[DEVICE] Initializing device: {}", d.id);
            }
        }
        Ok(())
    }
}
