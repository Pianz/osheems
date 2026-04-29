use std::sync::Arc;
use std::path::{PathBuf, Path};
use std::fs;
use serde_json::json;
use rhai::Map;

use osheems_common::managers::database::DatabaseManager;
use osheems_common::managers::template::TemplateManager;
use osheems_common::managers::driver::DriverManager;
use osheems_common::engines::execution::ExecutionEngine;

/// Nettoyage de la DB pour repartir sur un test propre
fn cleanup_database(db_path: &str) {
    let path = Path::new(db_path);
    if path.exists() {
        match fs::remove_file(path) {
            Ok(_) => println!("🧹 Database '{}' cleaned up successfully.", db_path),
            Err(e) => eprintln!("⚠️ Failed to delete database: {}. logic might fail due to UNIQUE constraints.", e),
        }
    }

    let _ = fs::remove_file(format!("{}-shm", db_path));
    let _ = fs::remove_file(format!("{}-wal", db_path));
}

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .build()
    .expect("Failed to build Tokio runtime");

    rt.block_on(async {
        env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

        println!("--- OSHEEMS Driver Execution Test (Multi-Gateway) ---");

        let main_db_path = "database/main.db";
        cleanup_database(main_db_path);

        let db_mgr = Arc::new(DatabaseManager::new().expect("Failed to init DB"));
        let tpl_mgr = Arc::new(TemplateManager::new(PathBuf::from("templates")));
        let driver_mgr = DriverManager::new(db_mgr.clone(), tpl_mgr.clone());

        // 1. BOOTSTRAP
        println!("🚀 Bootstrapping Core infrastructure...");
        db_mgr.main.bootstrap().expect("Bootstrap failed");

        // 2. CONFIGURATION DU TEST
        let remote_gw_id = "mqtt_gw";
        let target_mqtt_id = "shelly-pro3em-c82b96874564";

        println!("📝 Adding remote gateway and devices...");

        db_mgr.main.create_entity(
            remote_gw_id,
            "gateway",
            Some("gateways/mqtt"),
                                  Some("Remote MQTT Broker"),
                                  None,
                                  &json!({
                                      "broker_host": "192.168.5.11",
                                      "broker_port": 1883,
                                      "broker_topic": "osheems",
                                      "client_id": "osheems_core"
                                  }),
                                  &json!({}),
                                  false
        ).expect("Failed to create remote gateway");

        db_mgr.main.create_relation(
            remote_gw_id,
            "uses_interface",
            "net_eth0",
            &json!({ "priority": 10 }),
                                    true
        ).expect("Failed to link gateway to interface");

        db_mgr.main.create_entity(
            "shelly_1",
            "device",
            Some("devices/shelly/pro3em"),
                                  Some("Shelly Lounge"),
                                  None,
                                  &json!({}),
                                  &json!({}),
                                  false
        ).expect("Failed to create Shelly");

        db_mgr.main.create_relation(
            "shelly_1",
            "connected_to",
            remote_gw_id,
            &json!({ "mqtt_id": target_mqtt_id }),
                                    false
        ).expect("Failed to create relation");

        // 3. ORCHESTRATION & EXECUTION
        println!("⚙️ Orchestrating: {}", remote_gw_id);

        match driver_mgr.start_driver(remote_gw_id).await {
            Ok(active_driver) => {
                println!("✅ ActiveDriver for '{}' built.", remote_gw_id);
                let exec_engine = ExecutionEngine::new();

                match exec_engine.prepare(&active_driver) {
                    Ok(compiled) => {
                        // --- PARTIE 1 : SIMULATION RÉCEPTION (SENS DESCENDANT) ---
                        println!("\n📥 Simulating real Shelly message (Status EM)...");

                        let mut mqtt_payload = Map::new();
                        mqtt_payload.insert("topic".into(), format!("osheems/shellies/{}/status/em:0", target_mqtt_id).into());
                        mqtt_payload.insert("data".into(), r#"{"id":0,"a_current":0.029,"a_voltage":234.3,"a_act_power":0.2,"total_act_power":0.286}"#.into());

                        match exec_engine.route(&compiled, &active_driver, mqtt_payload.clone()) {
                            Ok(route_res) => {
                                let target_id = route_res.to_string();

                                if target_id.is_empty() || target_id == "null" {
                                    println!("⚠️ Gateway: No device matched for this topic.");
                                } else {
                                    println!("✨ Gateway routed to: {}", target_id);

                                    match exec_engine.process_device(&compiled, &active_driver, &target_id, mqtt_payload) {
                                        Ok(metrics) => println!("📊 Normalised Metrics: {:#?}", metrics),
                Err(e) => println!("❌ Device Parsing Error: {}", e),
                                    }
                                }
                            },
                Err(e) => println!("❌ Routing Error: {}", e),
                        }

                        // --- PARTIE 2 : SIMULATION COMMANDE (SENS MONTANT) ---
                        println!("\n📤 Simulating Command: Turn ON shelly_1...");

                        match exec_engine.send_to_device(&compiled, &active_driver, "shelly_1", "switch", true.into()) {
                            Ok(action) => {
                                println!("✨ Gateway translated command to network action:");
                                println!("   Protocol: {:?}", action.get("protocol").cloned().unwrap_or_default());
                                println!("   Topic:    {:?}", action.get("topic").cloned().unwrap_or_default());
                                println!("   Payload:  {:?}", action.get("payload").cloned().unwrap_or_default());
                            },
                Err(e) => println!("❌ Command Translation Error: {}", e),
                        }
                    },
                Err(e) => println!("❌ Compilation Error: {}", e),
                }
            },
            Err(e) => println!("❌ Orchestration Error: {}", e),
        }
    });
}
