use crate::managers::database::DatabaseManager;
use crate::managers::interface::AsyncInterface;
use crate::managers::interface::uart::UartInterface;
use crate::managers::interface::network::NetworkInterface;
use crate::managers::interface::i2c::I2cInterface;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json::{json, Value};

pub struct InterfaceManager {
    db_manager: Arc<DatabaseManager>,
    /// Registry of instantiated interfaces (ID -> Instance)
    interfaces: RwLock<HashMap<String, Arc<dyn AsyncInterface>>>,
}

impl InterfaceManager {
    pub fn new(db_manager: Arc<DatabaseManager>) -> Self {
        Self {
            db_manager,
            interfaces: RwLock::new(HashMap::new()),
        }
    }

    /// Main entry point: synchronizes DB with hardware and launches enabled interfaces
    pub async fn start(&self) -> Result<(), String> {
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
            // --- 1. UART SCAN (PnP with Hardware IDs) ---
            if let Ok(ports) = serialport::available_ports() {
                for p in ports {
                    if let serialport::SerialPortType::UsbPort(info) = p.port_type {
                        // Generate a unique Hardware ID (VID:PID:Serial)
                        let hw_id = format!("{:04x}:{:04x}:{}",
                                            info.vid,
                                            info.pid,
                                            info.serial_number.as_deref().unwrap_or("no_serial")
                        );

                        let port_id = format!("usb_{}", hw_id.replace(":", "_"));
                        discovered_paths.insert(port_id.clone(), p.port_name.clone());

                        // Register in DB if it's the first time we see this device
                        if self.db_manager.main.get_entity(&port_id).is_err() {
                            // User modifiable config
                            let config = json!({
                                "baud_rate": 9600
                            });

                            // System detected attributes
                            let attributes = json!({
                                "driver": "uart",
                                "hw_id": hw_id,
                                "original_path": p.port_name
                            });

                            self.db_manager.main.create_entity(
                                &port_id,
                                "interface",
                                None,
                                Some(&format!("USB Device ({})", p.port_name)),
                                                               Some("Plug'N'Play hardware interface"),
                                                               &config,
                                                               &attributes, // 7th argument
                                                               false        // 8th argument
                            ).ok();
                            println!("[PnP] New UART hardware registered: {}", port_id);
                        }
                    }
                }
            }

            // --- 2. NETWORK SCAN ---
            if let Ok(entries) = std::fs::read_dir("/sys/class/net/") {
                for entry in entries.flatten() {
                    let iface_name = entry.file_name().into_string().unwrap_or_default();
                    if iface_name == "lo" || iface_name.starts_with("veth") || iface_name.starts_with("br-") || iface_name.starts_with("docker") {
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

                        self.db_manager.main.create_entity(
                            &port_id,
                            "interface",
                            None,
                            Some(&format!("Network Interface: {}", iface_name)),
                                                           Some("Automatically detected network card"),
                                                           &config,
                                                           &attributes, // 7th argument
                                                           true         // 8th argument (System)
                        ).ok();
                        println!("[PnP] New Network hardware registered: {}", port_id);
                    }
                }
            }
        }
        Ok(discovered_paths)
    }

    /// Iterates through the DB to instantiate and open enabled interfaces
    async fn load_enabled_interfaces(&self, discovered_paths: HashMap<String, String>) -> Result<(), String> {
        let entities = self.db_manager.main.get_all_entities(Some("interface"))
        .map_err(|e| e.to_string())?;

        for entity in entities {
            if entity.is_enabled {
                // We extract the driver from attributes
                let driver = entity.attributes.get("driver").and_then(|v| v.as_str()).unwrap_or("");
                let mut runtime_config = entity.configuration.clone();

                // Inject the driver for the spawn_interface logic
                if let Value::Object(ref mut map) = runtime_config {
                    map.insert("driver".to_string(), json!(driver));
                }

                // PnP Resolution: If it's a UART/USB device, we update the path with the one currently found
                if let Some(current_path) = discovered_paths.get(&entity.id) {
                    if driver == "uart" {
                        runtime_config["path"] = json!(current_path);
                    } else if driver == "network" {
                        runtime_config["interface_name"] = json!(current_path);
                    }

                    if let Err(e) = self.spawn_interface(entity.id.clone(), &runtime_config).await {
                        eprintln!("[INTERFACE] Error starting {}: {}", entity.id, e);
                    }
                } else {
                    eprintln!("[INTERFACE] Hardware {} is enabled but not physically present. Skipping.", entity.id);
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

        println!("[INTERFACE] Interface '{}' started successfully.", id);
        Ok(())
    }

    pub async fn get_interface(&self, id: &str) -> Option<Arc<dyn AsyncInterface>> {
        let registry = self.interfaces.read().await;
        registry.get(id).cloned()
    }
}
