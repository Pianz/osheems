use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use log::{info, warn, error};
use serde_json::Value;

use crate::managers::database::DatabaseManager;
use crate::managers::interface::InterfaceManager;

pub struct GatewayManager {
    db_manager: Arc<DatabaseManager>,
    interface_manager: Arc<InterfaceManager>,
    /// Registre des gateways actives (ID -> État/Driver)
    active_gateways: RwLock<HashMap<String, Value>>,
}

impl GatewayManager {
    pub fn new(db_manager: Arc<DatabaseManager>, interface_manager: Arc<InterfaceManager>) -> Self {
        Self {
            db_manager,
            interface_manager,
            active_gateways: RwLock::new(HashMap::new()),
        }
    }

    /// Démarre uniquement les entités de type "gateway"
    pub async fn start(&self) -> Result<(), String> {
        // Filtrage strict sur "gateway"
        let entities = self.db_manager.main.get_all_entities(Some("gateway"))
        .map_err(|e| e.to_string())?;

        for entity in entities {
            if entity.is_enabled {
                // On cherche l'interface physique liée dans les attributs
                let interface_id = entity.attributes.get("interface_id")
                .and_then(|v| v.as_str());

                if let Some(iface_id) = interface_id {
                    if let Err(e) = self.spawn_gateway(entity.id.clone(), iface_id, &entity.configuration).await {
                        error!("[GATEWAY] Failed to start gateway '{}': {}", entity.id, e);
                    }
                } else {
                    // Pour certaines gateways virtuelles, l'interface_id peut être Optionnelle
                    warn!("[GATEWAY] Gateway '{}' has no interface_id. Is it a virtual gateway?", entity.id);
                }
            }
        }
        Ok(())
    }

    pub async fn spawn_gateway(&self, id: String, interface_id: &str, config: &Value) -> Result<(), String> {
        // Vérification de l'interface via l'InterfaceManager
        let interface = self.interface_manager.get_interface(interface_id).await;

        if interface.is_none() {
            return Err(format!("Physical interface '{}' is not available", interface_id));
        }

        info!("[GATEWAY] Starting protocol handler for '{}' using interface '{}'", id, interface_id);

        // TODO: Initialiser ici le client MQTT ou le Master Modbus

        let mut registry = self.active_gateways.write().await;
        registry.insert(id, config.clone());

        Ok(())
    }
}
