use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::mpsc;
use rhai::{Map, Dynamic};
use osheems_common::managers::database::DatabaseManager;
use osheems_common::managers::template::TemplateManager;
use osheems_common::managers::driver::DriverManager;
use osheems_common::engines::execution::ExecutionEngine;
use osheems_common::runners::mqtt::MqttRunner;
use osheems_common::core_bus::manager::CoreBusManager;
use osheems_common::core_bus::dispatcher::CoreBusDispatcher;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- OSHEEMS Real-Time Shelly Test (v2 Architecture) ---");

    // 1. Setup Managers de base
    let db_mgr = Arc::new(DatabaseManager::new()?);
    let tpl_mgr = Arc::new(TemplateManager::new(PathBuf::from("templates")));
    let driver_mgr = DriverManager::new(db_mgr.clone(), tpl_mgr.clone());
    let exec_engine = Arc::new(ExecutionEngine::new());

    // 2. Setup du CoreBusDispatcher
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

    // 3. Setup du CoreBusManager
    // Note: get_entity est synchrone dans ton implémentation MainDatabase
    let mqtt_local_entity = db_mgr.main.get_entity("mqtt_local")
    .map_err(|_| "Entity 'mqtt_local' not found in database")?;

    // Conversion de serde_json::Value vers rhai::Map
    let mut config_map = Map::new();
    if let Some(obj) = mqtt_local_entity.configuration.as_object() {
        for (k, v) in obj {
            // Conversion simple des types JSON vers Rhai Dynamic
            let dynamic_val = if v.is_string() {
                Dynamic::from(v.as_str().unwrap().to_string())
            } else if v.is_number() {
                Dynamic::from(v.as_i64().unwrap_or(0))
            } else {
                Dynamic::UNIT
            };
            config_map.insert(k.clone().into(), dynamic_val);
        }
    }

    let core_bus_ptr = CoreBusManager::new(
        "core".into(),           // role
                                           "test-bench".into(),     // name
                                           config_map,              // Map convertie
                                           dispatcher.clone(),
    );

    // 4. Orchestration du Driver
    let gateway_id = "mqtt_gw";
    let active_driver = driver_mgr.start_driver(gateway_id).await?;
    let compiled = exec_engine.prepare(&active_driver)?;

    println!("\n--- 🔗 Topic Mapping Map ---");
    for (device_id, _dev_res) in &active_driver.devices_resources {
        println!("📝 Device: {}", device_id);
    }
    println!("-----------------------------\n");

    let active_driver = Arc::new(active_driver);
    let compiled = Arc::new(compiled);
    let (_tx_cmd, rx_cmd) = mpsc::channel::<Map>(32);

    // 5. Launch Runner (Monde Physique)
    let runner_driver = active_driver.clone();
    let runner_compiled = compiled.clone();
    let runner_engine = exec_engine.clone();
    let runner_bus = core_bus_ptr.clone();

    tokio::spawn(async move {
        if let Err(e) = MqttRunner::run(
            gateway_id.to_string(),
                                        runner_driver,
                                        runner_compiled,
                                        runner_engine,
                                        runner_bus,
                                        rx_cmd
        ).await {
            eprintln!("🛑 [RUNNER ERROR] {}", e);
        }
    });

    println!("👀 Monitoring OSHEEMS Channels...");

    // 6. Boucle de test
    loop {
        tokio::select! {
            Some(msg) = rx_telemetry.recv() => {
                println!("✅ [DISPATCHED EVT] Device: {:?} Data: {:?}",
                         msg.get("device_id"),
                         msg.get("data")
                );
            }
            Some(log) = rx_logs.recv() => {
                println!("📝 [DISPATCHED LOG] {:?}", log);
            }
        }
    }
}
