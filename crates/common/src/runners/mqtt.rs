use rumqttc::{AsyncClient, MqttOptions, QoS, Event, Packet};
use tokio::sync::mpsc;
use std::sync::Arc;
use rhai::Map;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::engines::execution::{ExecutionEngine, CompiledDriver};
use crate::managers::driver::ActiveDriver;
use crate::core_bus::manager::{CoreBusManager, CorePayload};

pub struct MqttRunner;

impl MqttRunner {
    pub async fn run(
        gateway_id: String,
        active_driver: Arc<ActiveDriver>,
        compiled: Arc<CompiledDriver>,
        engine: Arc<ExecutionEngine>,
        core_bus: Arc<CoreBusManager>,
        mut rx_cmd: mpsc::Receiver<Map>,
    ) -> Result<(), String> {

        // --- 1. CONFIGURATION DU CLIENT PHYSIQUE ---
        // MODIFICATION : On utilise la gateway pour les infos de connexion au broker.
        // L'interface représente le protocole, mais la gateway contient l'instance config.
        let config = &active_driver.gateway.configuration;

        let host = config["host"].as_str()
        .or_else(|| config["broker_host"].as_str())
        .ok_or_else(|| format!("Gateway '{}': Missing host/broker_host in configuration", gateway_id))?;

        let port = config["port"].as_u64()
        .or_else(|| config["broker_port"].as_u64())
        .unwrap_or(1883) as u16;

        let mut mqttoptions = MqttOptions::new(format!("gw-{}", gateway_id), host, port);
        mqttoptions.set_keep_alive(std::time::Duration::from_secs(5));

        // Authentification optionnelle
        if let (Some(u), Some(p)) = (config["username"].as_str(), config["password"].as_str()) {
            mqttoptions.set_credentials(u, p);
        }

        let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

        // --- SOUSCRIPTION DYNAMIQUE ---
        for (id, dev_res) in &active_driver.devices_resources {
            if let Some(device_topic) = dev_res.configuration.get("device_topic").and_then(|v| v.as_str()) {
                let sub_filter = format!("{}/#", device_topic);
                client.subscribe(&sub_filter, QoS::AtMostOnce).await.ok();
                println!("📡 [GW:{}] Subscribed to physical topic for {}: {}", gateway_id, id, sub_filter);
            }
        }

        println!("📡 [GW:{}] Connected to external broker at {}:{}", gateway_id, host, port);

        // --- 2. BOUCLE PRINCIPALE ---
        loop {
            tokio::select! {
                notification = eventloop.poll() => {
                    match notification {
                        Ok(Event::Incoming(Packet::Publish(publish))) => {
                            let topic = publish.topic.clone();
                            let payload_str = String::from_utf8_lossy(&publish.payload).to_string();

                            let mut msg = Map::new();
                            msg.insert("topic".into(), topic.clone().into());
                            msg.insert("data".into(), payload_str.into());

                            // A. Routage via le moteur Rhai
                            match engine.route(&compiled, &active_driver, msg.clone()) {
                                Ok(device_id_dyn) => {
                                    let device_id = device_id_dyn.to_string();

                                    if !device_id.is_empty() && device_id != "null" {
                                        // B. Traitement des données du device
                                        match engine.process_device(&compiled, &active_driver, &device_id, msg) {
                                            Ok(metrics_dynamic) => {
                                                if let Some(metrics) = metrics_dynamic.try_cast::<Map>() {
                                                    let bus = core_bus.clone();
                                                    let d_id = device_id.clone();

                                                    if let Some(dev_res) = active_driver.devices_resources.get(&d_id) {
                                                        // Récupération du mapping MQTT
                                                        let mqtt_points_opt = dev_res.mappings.get("mqtt")
                                                        .and_then(|m| m.get("points"))
                                                        .and_then(|p| p.as_object());

                                                        if let Some(mqtt_points) = mqtt_points_opt {
                                                            for (p_id, point_def) in mqtt_points {
                                                                if let Some(val) = metrics.get(p_id.as_str()) {

                                                                    // 1. Détermination du Label
                                                                    let user_label = dev_res.configuration.get("labels")
                                                                    .and_then(|l| l.get(p_id.as_str()))
                                                                    .and_then(|v| v.as_str());
                                                                    let default_label = point_def.get("label").and_then(|v| v.as_str());
                                                                    let final_label = user_label.or(default_label).unwrap_or(p_id.as_str());

                                                                    // 2. Récupération de l'unité depuis le template
                                                                    let unit = dev_res.template["points"]["states"]
                                                                    .as_array()
                                                                    .and_then(|states| {
                                                                        states.iter().find(|s| s["id"].as_str() == Some(p_id.as_str()))
                                                                    })
                                                                    .and_then(|s| s["unit"].as_str())
                                                                    .unwrap_or("");

                                                                    // 3. Horodatage
                                                                    let ts = SystemTime::now()
                                                                    .duration_since(UNIX_EPOCH)
                                                                    .unwrap_or_default()
                                                                    .as_millis() as u64;

                                                                    // 4. Publication sur le Core Bus
                                                                    let core_data = CorePayload {
                                                                        value: serde_json::to_value(val).unwrap_or(serde_json::Value::Null),
                                                                        unit: unit.to_string(),
                                                                        label: final_label.to_string(),
                                                                        timestamp: ts,
                                                                    };

                                                                    let p_topic = p_id.to_string();
                                                                    let bus_inner = bus.clone();
                                                                    let d_id_inner = d_id.clone();

                                                                    tokio::spawn(async move {
                                                                        if let Err(e) = bus_inner.publish_telemetry(&d_id_inner, &p_topic, core_data).await {
                                                                            eprintln!("❌ [CORE BUS ERROR] {} for {}/{}", e, d_id_inner, p_topic);
                                                                        }
                                                                    });
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            },
                                            Err(e) => eprintln!("❌ [ENGINE ERROR] process_device failed for {}: {}", device_id, e),
                                        }
                                    }
                                },
                                Err(e) => eprintln!("❌ [ROUTING ERROR] Engine failed to route topic {}: {}", topic, e),
                            }
                        },
                        Err(e) => {
                            eprintln!("⚠️ [GW:{}] Eventloop error: {}", gateway_id, e);
                            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        }
                        _ => {}
                    }
                }

                // Réception d'une commande depuis le Core Bus vers le monde physique
                Some(action) = rx_cmd.recv() => {
                    let topic = action.get("topic").map(|t| t.to_string()).unwrap_or_default();
                    let payload = action.get("payload").map(|p| p.to_string()).unwrap_or_default();

                    if !topic.is_empty() {
                        if let Err(e) = client.publish(&topic, QoS::AtMostOnce, false, payload).await {
                            eprintln!("❌ [GW:{}] Failed to send command: {}", gateway_id, e);
                        }
                    }
                }
            }
        }
    }
}
