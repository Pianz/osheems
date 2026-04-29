use tokio::sync::mpsc;
use std::sync::Arc;
use std::convert::TryFrom;
use rhai::Map;
use serde::{Serialize, Deserialize};

use crate::core_bus::runner::CoreBusRunner;
use crate::core_bus::types::MqttSuffix;
use crate::core_bus::dispatcher::CoreBusDispatcher;

/// Structure normalisée pour les données de télémétrie sur le CoreBus.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CorePayload {
    pub value: serde_json::Value,
    pub unit: String,
    pub label: String,
    pub timestamp: u64,
}

pub struct CoreBusManager {
    sender: mpsc::Sender<(String, Vec<u8>)>,
    role: String, // "core" ou "edge"
}

impl CoreBusManager {
    /// Initialise le CoreBus avec son Runner et son Dispatcher.
    pub fn new(
        role: String,
        name: String,
        configuration: Map,
        dispatcher: Arc<CoreBusDispatcher>
    ) -> Arc<Self> {
        let (tx_out, rx_out) = mpsc::channel(100);
        let (tx_in, mut rx_in) = mpsc::channel(100);

        let manager = Arc::new(Self {
            sender: tx_out,
            role,
        });

        // 1. Lancement du Runner (Gestion MQTT)
        let conf = configuration.clone();
        let runner_name = name.clone();
        tokio::spawn(async move {
            if let Err(e) = CoreBusRunner::run(runner_name, conf, rx_out, tx_in).await {
                eprintln!("❌ [CORE BUS FATAL] Runner exited: {}", e);
            }
        });

        // 2. Boucle de Dispatching (MQTT -> Logic)
        let disp = dispatcher.clone();
        tokio::spawn(async move {
            while let Some((topic, payload)) = rx_in.recv().await {
                let parts: Vec<&str> = topic.split('/').collect();

                // On gère les topics à 5 segments pour inclure le point_topic
                if (parts.len() == 4 || parts.len() == 5) && parts[0] == "osheems" {
                    let r = parts[1];
                    let d_topic = parts[2]; // Correspond à {device_topic}
                    let s_str = parts[3];   // get|set|evt|conf|logs

                    if let Ok(suffix) = MqttSuffix::try_from(s_str) {
                        // On dispatch en utilisant le device_topic comme identifiant
                        if let Err(e) = disp.dispatch(r, d_topic, suffix, payload).await {
                            eprintln!("⚠️ [DISPATCH ERROR] {}", e);
                        }
                    }
                }
            }
        });

        manager
    }

    /// Publie une mesure de télémétrie structurée sur le bus.
    /// Utilise automatiquement le suffixe 'evt'.
    pub async fn publish_telemetry(
        &self,
        device_topic: &str,
        point_topic: &str,
        data: CorePayload
    ) -> Result<(), String> {
        let payload_bytes = serde_json::to_vec(&data)
        .map_err(|e| format!("Serialization error: {}", e))?;

        self.publish(device_topic, MqttSuffix::Evt, point_topic, payload_bytes).await
    }

    /// Publie un message brut sur le bus.
    /// Format : osheems/{role}/{device_topic}/{suffix}/{point_topic}
    pub async fn publish(
        &self,
        device_topic: &str,
        suffix: MqttSuffix,
        point_topic: &str,
        payload: Vec<u8>
    ) -> Result<(), String> {

        let topic = format!(
            "osheems/{}/{}/{}/{}",
            self.role,
            device_topic,
            suffix.to_string().to_lowercase(),
                            point_topic
        );

        self.sender.send((topic, payload)).await
        .map_err(|e| format!("Failed to send to CoreBusRunner: {}", e))
    }
}
