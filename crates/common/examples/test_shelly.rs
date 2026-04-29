use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::mpsc;
use rhai::{Map, Dynamic};
use log::{info, error};

use osheems_common::managers::database::DatabaseManager;
use osheems_common::managers::template::TemplateManager;
use osheems_common::managers::driver::DriverManager;
use osheems_common::engines::execution::ExecutionEngine;
use osheems_common::runners::mqtt::MqttRunner;
use osheems_common::core_bus::manager::CoreBusManager;
use osheems_common::core_bus::dispatcher::CoreBusDispatcher;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialisation du logger pour voir les scans de templates
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    println!("--- OSHEEMS Real-Time Shelly Test (Integrated Architecture) ---");

    // 1. INITIALISATION DES SERVICES CORE
    // La DB doit déjà être bootstrapée et contenir mqtt_local, mqtt_gw et tes shelly
    let db_mgr = Arc::new(DatabaseManager::new()?);

    // Le TemplateManager scanne maintenant le dossier 'templates/' au lancement
    let tpl_mgr = Arc::new(TemplateManager::new(PathBuf::from("templates")));

    let driver_mgr = DriverManager::new(db_mgr.clone(), tpl_mgr.clone());
    let exec_engine = Arc::new(ExecutionEngine::new());

    // 2. SETUP DU DISPATCHER (Sorties du CoreBus)
    let (tx_telemetry, mut rx_telemetry) = mpsc::channel(100);
    let (tx_driver, _rx_driver) = mpsc::channel(100);
    let (tx_logs, mut rx_logs) = mpsc::channel(100);
    let (tx_entities, _rx_entities) = mpsc::channel(100);

    let dispatcher = Arc::new(CoreBusDispatcher::new(
        tx_telemetry,
        tx_driver,
        tx_logs,
        tx_entities,
    ));

    // 3. PRÉPARATION DE L'INTERFACE (mqtt_local)
    let mqtt_local_entity = db_mgr.main.get_entity("mqtt_local")
    .map_err(|_| "Entity 'mqtt_local' missing from DB. Run bootstrap first.")?;

    // Conversion robuste pour Rhai (CoreBusManager en a besoin pour son propre client MQTT)
    let config_map: Map = rhai::serde::to_dynamic(mqtt_local_entity.configuration.clone())
    .unwrap_or(Dynamic::from(Map::new()))
    .cast::<Map>();

    let core_bus_ptr = CoreBusManager::new(
        "core".into(),
                                           "test-bench".into(),
                                           config_map,
                                           dispatcher.clone(),
    );

    // 4. ASSEMBLAGE DU DRIVER POUR LA GATEWAY
    let gateway_id = "mqtt_gw";
    info!("Assembling driver for: {}", gateway_id);

    // DriverManager va chercher le template, la config de la gateway et les devices reliés
    let active_driver = driver_mgr.start_driver(gateway_id).await?;
    let compiled = exec_engine.prepare(&active_driver)?;

    let active_driver = Arc::new(active_driver);
    let compiled = Arc::new(compiled);
    let (_tx_cmd, rx_cmd) = mpsc::channel::<Map>(32);

    // 5. LANCEMENT DU RUNNER (Connexion au réseau réel)
    let runner_driver = active_driver.clone();
    let runner_compiled = compiled.clone();
    let runner_engine = exec_engine.clone();
    let runner_bus = core_bus_ptr.clone();

    tokio::spawn(async move {
        info!("Launching MqttRunner for gateway '{}'...", gateway_id);
        if let Err(e) = MqttRunner::run(
            gateway_id.to_string(),
                                        runner_driver,
                                        runner_compiled,
                                        runner_engine,
                                        runner_bus,
                                        rx_cmd
        ).await {
            error!("🛑 [RUNNER CRASHED] {}", e);
        }
    });

    println!("🚀 OSHEEMS is live. Waiting for Shelly data...");

    // 6. SURVEILLANCE DES FLUX D'ÉVÉNEMENTS RÉELS
    loop {
        tokio::select! {
            Some(msg) = rx_telemetry.recv() => {
                // Ici, tu reçois les métriques normalisées par tes scripts Rhai/JS
                println!("📊 [METRIC] Device: {:?} | Data: {}",
                         msg.get("device_id").unwrap_or(&Dynamic::from("unknown")),
                         serde_json::to_string(msg.get("data").unwrap_or(&Dynamic::from("{}")))?
                );
            }
            Some(log) = rx_logs.recv() => {
                println!("📝 [CORE LOG] {:?}", log);
            }
            _ = tokio::signal::ctrl_c() => {
                println!("\n👋 Shutting down test bench...");
                break;
            }
        }
    }

    Ok(())
}
