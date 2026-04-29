use std::sync::Arc;
use tokio::sync::mpsc;
use crate::core_bus::types::MqttSuffix;
use rhai::Map;

// On importe les traits ou managers nécessaires pour le dispatch
// Note : Ces imports seront à ajuster selon la structure exacte de tes autres managers
// use crate::managers::telemetry::TelemetryManager;
// use crate::managers::driver::DriverManager;

pub struct CoreBusDispatcher {
    // Canaux vers les autres composants du système
    tx_telemetry: mpsc::Sender<Map>,
    tx_driver: mpsc::Sender<Map>,
    tx_logs: mpsc::Sender<Map>,
    tx_entities: mpsc::Sender<Map>,
}

impl CoreBusDispatcher {
    pub fn new(
        tx_telemetry: mpsc::Sender<Map>,
        tx_driver: mpsc::Sender<Map>,
        tx_logs: mpsc::Sender<Map>,
        tx_entities: mpsc::Sender<Map>,
    ) -> Self {
        Self {
            tx_telemetry,
            tx_driver,
            tx_logs,
            tx_entities,
        }
    }

    /// La fonction centrale d'aiguillage.
    /// Elle prend un topic décomposé et un payload, puis décide qui doit traiter l'info.
    pub async fn dispatch(
        &self,
        role: &str,
        device_id: &str,
        suffix: MqttSuffix,
        payload: Vec<u8>,
    ) -> Result<(), String> {

        // Préparation du message pour les managers (souvent en JSON/Map pour Rhai)
        let mut message = Map::new();
        message.insert("role".into(), role.into());
        message.insert("device_id".into(), device_id.into());

        // Conversion du payload pour traitement
        let data: serde_json::Value = serde_json::from_slice(&payload)
        .unwrap_or(serde_json::Value::String(String::from_utf8_lossy(&payload).to_string()));

        message.insert("data".into(), format!("{:?}", data).into());

        // --- LOGIQUE DE ROUTAGE OSHEEMS ---
        match suffix {
            // Événements de mesure -> Vers le TelemetryManager (Stockage SQLite)
            MqttSuffix::Evt => {
                self.tx_telemetry.send(message).await
                .map_err(|e| format!("Dispatcher: Telemetry drop: {}", e))?;
            },

            // Commandes de contrôle -> Vers le DriverManager (Traduction vers Runner)
            MqttSuffix::Set | MqttSuffix::Get => {
                self.tx_driver.send(message).await
                .map_err(|e| format!("Dispatcher: Driver drop: {}", e))?;
            },

            // Logs système ou équipement -> Vers le service de logging
            MqttSuffix::Logs => {
                self.tx_logs.send(message).await
                .map_err(|e| format!("Dispatcher: Log drop: {}", e))?;
            },

            // Mises à jour de configuration -> Vers l'EntityManager
            MqttSuffix::Conf => {
                self.tx_entities.send(message).await
                .map_err(|e| format!("Dispatcher: Entity drop: {}", e))?;
            },
        }

        Ok(())
    }
}
