use rumqttc::{AsyncClient, MqttOptions, QoS, Event, Packet};
use tokio::sync::mpsc;
use std::time::Duration;
use rhai::Map;

pub struct CoreBusRunner;

impl CoreBusRunner {
    pub async fn run(
        name: String,
        configuration: Map,
        mut rx_publish: mpsc::Receiver<(String, Vec<u8>)>,
                     tx_incoming: mpsc::Sender<(String, Vec<u8>)>,
    ) -> Result<(), String> {

        // --- 1. CONFIGURATION (Extraction simplifiée de Rhai) ---
        let host = configuration.get("broker_host")
        .and_then(|v| {
            // On essaie de récupérer une String. Rhai peut renvoyer Dynamic(String)
            if v.is::<String>() { Some(v.clone().cast::<String>()) } else { None }
        })
        .ok_or("CoreBus: Missing host")?;

        let port = configuration.get("broker_port")
        .and_then(|v| v.as_int().ok())
        .unwrap_or(1883) as u16;

        let mut mqttoptions = MqttOptions::new(format!("core-{}", name), host, port);
        mqttoptions.set_keep_alive(Duration::from_secs(5));

        // Extraction Username / Password
        let username = configuration.get("username").and_then(|v| {
            if v.is::<String>() { Some(v.clone().cast::<String>()) } else { None }
        });
        let password = configuration.get("password").and_then(|v| {
            if v.is::<String>() { Some(v.clone().cast::<String>()) } else { None }
        });

        if let (Some(u), Some(p)) = (username, password) {
            mqttoptions.set_credentials(u, p);
        }

        let (client, mut eventloop) = AsyncClient::new(mqttoptions, 50);
        client.subscribe("osheems/#", QoS::AtLeastOnce).await.ok();

        println!("💎 [CORE BUS] Connected to broker for {}", name);

        // --- 2. MAIN LOOP ---
        loop {
            tokio::select! {
                notification = eventloop.poll() => {
                    match notification {
                        Ok(Event::Incoming(Packet::Publish(publish))) => {
                            let topic = publish.topic.clone();
                            let payload = publish.payload.to_vec();
                            if let Err(e) = tx_incoming.send((topic, payload)).await {
                                eprintln!("❌ [CORE BUS] Dispatcher channel closed: {}", e);
                            }
                        }
                        Err(e) => {
                            eprintln!("❌ [CORE BUS MQTT ERROR] {}", e);
                            tokio::time::sleep(Duration::from_secs(1)).await;
                        }
                        _ => {}
                    }
                }

                Some((topic, payload)) = rx_publish.recv() => {
                    if let Err(e) = client.publish(topic, QoS::AtLeastOnce, false, payload).await {
                        eprintln!("❌ [CORE BUS PUB ERROR] {}", e);
                    }
                }
            }
        }
    }
}
