// crates/common/src/managers/interface/manager.rs

use crate::managers::database::DatabaseManager;
use crate::managers::interface::AsyncInterface;
use crate::managers::interface::uart::UartInterface;
use crate::managers::interface::network::NetworkInterface;
use crate::managers::interface::i2c::I2cInterface;
use crate::core_bus::manager::CoreBusManager;
use crate::core_bus::types::MqttSuffix;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use log::{info, error, warn};
use serde_json::{json, Value};

pub struct InterfaceManager {
    db_manager: Arc<DatabaseManager>,
    bus: Arc<CoreBusManager>,
    /// Registry of instantiated interfaces (ID -> Instance)
    interfaces: RwLock<HashMap<String, Arc<dyn AsyncInterface>>>,
}

impl InterfaceManager {
    pub fn new(db_manager: Arc<DatabaseManager>, bus: Arc<CoreBusManager>) -> Self {
        Self {
            db_manager,
            bus,
            interfaces: RwLock::new(HashMap::new()),
        }
    }

    /// Helper to publish logs to CoreBus
    async fn publish_log(&self, level: &str, id: &str, message: &str) {
        let payload_json = json!({
            "id": id,
            "level": level,
            "module": "interface_manager",
            "msg": message,
            "ts": chrono::Utc::now().to_rfc3339()
        });

        if let Ok(payload) = serde_json::to_vec(&payload_json) {
            let sub_topic = format!("interface/{}", level);
            self.bus.publish("osheems/core/system", MqttSuffix::Logs, &sub_topic, payload).await;
        }
    }

    /// Main entry point: synchronizes DB with hardware and launches enabled interfaces
    pub async fn start(&self) -> Result<(), String> {
        info!("[INTERFACE] Starting hardware scan...");

        // 1. Detect hardware and resolve dynamic paths (PnP)
        let discovered_paths = self.scan_hardware().await?;

        // 2. Load and open interfaces marked as "is_enabled"
        self.load_enabled_interfaces(discovered_paths).await?;

        Ok(())
    }

    /// Scans physical resources and returns a map of HardwareID -> CurrentPath
    async fn scan_hardware(&self) -> Result<HashMap<String, String>, String> {
        let mut discovered_paths = HashMap::new();

        #[cfg(target_os = "linux")]
        {
            // --- 1. UART SCAN ---
            if let Ok(ports) = serialport::available_ports() {
                for p in ports {
                    if let serialport::SerialPortType::UsbPort(info) = p.port_type {
                        let hw_id = format!("{:04x}:{:04x}:{}",
                                            info.vid,
                                            info.pid,
                                            info.serial_number.as_deref().unwrap_or("no_serial")
                        );

                        let port_id = format!("usb_{}", hw_id.replace(":", "_"));
                        discovered_paths.insert(port_id.clone(), p.port_name.clone());

                        if self.db_manager.main.get_entity(&port_id).is_err() {
                            let config = json!({ "baud_rate": 9600 });
                            let attributes = json!({
                                "driver": "uart",
                                "hw_id": hw_id,
                                "original_path": p.port_name
                            });

                            let _ = self.db_manager.main.create_entity(
                                &port_id,
                                "interface",
                                None,
                                Some(&format!("USB Device ({})", p.port_name)),
                                                                       Some("Plug'N'Play hardware interface"),
                                                                       &config,
                                                                       &attributes,
                                                                       false
                            );

                            self.publish_log("info", &port_id, "New UART hardware discovered and registered").await;
                        }
                    }
                }
            }

            // --- 2. NETWORK SCAN ---
            if let Ok(entries) = std::fs::read_dir("/sys/class/net/") {
                for entry in entries.flatten() {
                    let iface_name = entry.file_name().into_string().unwrap_or_default();
                    if ["lo", "veth", "br-", "docker"].iter().any(|&pre| iface_name.starts_with(pre)) {
                        continue;
                    }

                    let port_id = format!("net_{}", iface_name);
                    discovered_paths.insert(port_id.clone(), iface_name.clone());

                    if self.db_manager.main.get_entity(&port_id).is_err() {
                        let config = json!({});
                        let attributes = json!({
                            "driver": "network",
                            "interface_name": iface_name
                        });

                        let _ = self.db_manager.main.create_entity(
                            &port_id,
                            "interface",
                            None,
                            Some(&format!("Network Interface: {}", iface_name)),
                                                                   Some("Automatically detected network card"),
                                                                   &config,
                                                                   &attributes,
                                                                   true
                        );

                        self.publish_log("info", &port_id, "New network interface registered").await;
                    }
                }
            }
        }
        Ok(discovered_paths)
    }

    async fn load_enabled_interfaces(&self, discovered_paths: HashMap<String, String>) -> Result<(), String> {
        let entities = self.db_manager.main.get_all_entities(Some("interface"))
        .map_err(|e| e.to_string())?;

        for entity in entities {
            if entity.is_enabled {
                let driver = entity.attributes.get("driver").and_then(|v| v.as_str()).unwrap_or("");
                let mut runtime_config = entity.configuration.clone();

                if let Value::Object(ref mut map) = runtime_config {
                    map.insert("driver".to_string(), json!(driver));
                }

                if let Some(current_path) = discovered_paths.get(&entity.id) {
                    match driver {
                        "uart" => { runtime_config["path"] = json!(current_path); },
                        "network" => { runtime_config["interface_name"] = json!(current_path); },
                        _ => {}
                    }

                    if let Err(e) = self.spawn_interface(entity.id.clone(), &runtime_config).await {
                        let err_msg = format!("Failed to spawn interface: {}", e);
                        error!("[INTERFACE] {}", err_msg);
                        self.publish_log("error", &entity.id, &err_msg).await;
                    }
                } else {
                    let warn_msg = "Hardware enabled but not physically present. Skipping.";
                    warn!("[INTERFACE] {} ({})", entity.id, warn_msg);
                    self.publish_log("warn", &entity.id, warn_msg).await;
                }
            }
        }
        Ok(())
    }

    pub async fn spawn_interface(&self, id: String, config: &Value) -> Result<(), String> {
        let driver_type = config.get("driver").and_then(|v| v.as_str()).unwrap_or("");

        let mut interface: Box<dyn AsyncInterface> = match driver_type {
            "uart" => Box::new(UartInterface::from_config(config).ok_or("Invalid UART config")?),
            "network" => Box::new(NetworkInterface::from_config(config).ok_or("Invalid Network config")?),
            "i2c" => Box::new(I2cInterface::from_config(config).ok_or("Invalid I2C config")?),
            _ => return Err(format!("Unknown driver: {}", driver_type)),
        };

        interface.open().await.map_err(|e| e.to_string())?;

        let mut registry = self.interfaces.write().await;
        registry.insert(id.clone(), Arc::from(interface));

        info!("[INTERFACE] Interface '{}' started successfully.", id);

        let evt_payload = serde_json::to_vec(&json!({ "id": id, "status": "online" })).unwrap_or_default();
        self.bus.publish("osheems/core/system", MqttSuffix::Evt, "interface/started", evt_payload).await;

        self.publish_log("info", &id, "Interface opened and registered in runtime").await;
        Ok(())
    }

    pub async fn get_interface(&self, id: &str) -> Option<Arc<dyn AsyncInterface>> {
        let registry = self.interfaces.read().await;
        registry.get(id).cloned()
    }
}
