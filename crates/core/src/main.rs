// crates/core/src/main.rs

use std::sync::Arc;
use std::path::PathBuf;
use log::{info, error};
use tokio::sync::mpsc;
use rhai::Map;

// On importe les managers et types nécessaires
use osheems_common::managers::database::DatabaseManager;
use osheems_common::managers::template::TemplateManager;
use osheems_common::managers::driver::DriverManager;
use osheems_common::engines::execution::ExecutionEngine;
use osheems_common::core_bus::manager::CoreBusManager;
use osheems_common::core_bus::dispatcher::CoreBusDispatcher;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialisation du logging (en anglais)
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    info!("Starting OSHEEMS Core (Open Source High Efficiency Energies Management)...");

    // 2. Configuration des chemins (Snap ou Local fallback géré par le DatabaseManager)
    let base_path = PathBuf::from("./templates");

    // 3. Initialisation du Database Manager
    // Il gère 'main.db' pour la config et les fichiers mensuels pour la télémétrie
    let db_manager = Arc::new(DatabaseManager::new()?);
    info!("Database manager initialized. Fetching CoreBus configuration...");

    // 4. Extraction de la configuration mqtt_local depuis la table des entités
    // On accède à db_manager.main pour utiliser les méthodes de MainDatabase
    let bus_config = match db_manager.main.get_all_entities(Some("gateway")) {
        Ok(entities) => entities
        .iter()
        .find(|gw| gw.id == "mqtt_local") // On cherche l'ID spécifique à ton MQTT local
        .and_then(|gw| {
            // On convertit la configuration JSON en Map Rhai pour le bus
            serde_json::from_value::<Map>(gw.configuration.clone()).ok()
        })
        .unwrap_or_else(|| {
            error!("Gateway 'mqtt_local' not found in database. Using empty defaults.");
            Map::new()
        }),
        Err(e) => {
            error!("Failed to query gateways from database: {}", e);
            Map::new()
        }
    };

    // 5. Initialisation de l'infrastructure Core Bus
    let (tx_telemetry, _rx_telemetry) = mpsc::channel(100);
    let (tx_events, _rx_events) = mpsc::channel(100);
    let (tx_logs, _rx_logs) = mpsc::channel(100);
    let (tx_stats, _rx_stats) = mpsc::channel(100);

    let dispatcher = Arc::new(CoreBusDispatcher::new(
        tx_telemetry,
        tx_events,
        tx_logs,
        tx_stats,
    ));

    // Le CoreBusManager::new renvoie déjà un Arc<Self>
    let core_bus = CoreBusManager::new(
        "core".to_string(),
                                       "osheems_core".to_string(),
                                       bus_config,
                                       dispatcher,
    );

    // 6. Initialisation du Template Manager et de l'Execution Engine (Rhai)
    let template_manager = Arc::new(TemplateManager::new(base_path));
    let exec_engine = Arc::new(ExecutionEngine::new());

    // 7. Initialisation du Driver Manager
    let driver_manager = DriverManager::new(
        db_manager.clone(),
                                            template_manager.clone()
    );

    // 8. Démarrage des passerelles et drivers (Shelly, Modbus, etc.)
    // Le bus est maintenant correctement configuré via la DB avant ce lancement
    if let Err(e) = driver_manager.initialize_all_from_db(exec_engine, core_bus).await {
        error!("Failed to initialize drivers from database: {}", e);
    }

    info!("OSHEEMS Core is running. Monitoring energy efficiency...");

    // 9. Arrêt propre sur Ctrl+C
    tokio::signal::ctrl_c().await?;
    info!("Shutting down OSHEEMS gracefully...");

    Ok(())
}
