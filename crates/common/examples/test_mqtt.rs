use osheems_common::mqtt::types::{MqttSuffix, OsheemsRole};
use osheems_common::mqtt::MqttManager;
use rumqttc::Event;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== OSHEEMS MQTT Integration Test ===");

    // Configuration - À adapter selon ton broker de test
    let broker_host = "192.168.5.11";
    let broker_port = 1883;
    let host_id = "test-station";
    let user = "oshems".to_string(); // Ton user
    let pass = "05H3M5".to_string(); // Ton pass

    // 1. Initialisation
    let (mqtt_manager, mut event_loop) = MqttManager::new(
        OsheemsRole::Core,
        host_id,
        broker_host,
        broker_port,
        Some((user, pass)),
    );

    // 2. Souscription aux commandes pour CET hôte spécifique
    // Topic: osheems/core/test-station/+/set
    let sub_topic = format!("osheems/{}/{}/+/set", OsheemsRole::Core, host_id);
    mqtt_manager.client.subscribe(&sub_topic, rumqttc::QoS::AtLeastOnce).await?;
    println!("SUBSCRIBE: {}", sub_topic);

    // 3. Gestion de la boucle d'événements en tâche de fond
    tokio::spawn(async move {
        loop {
            match event_loop.poll().await {
                Ok(notification) => {
                    if let Event::Incoming(rumqttc::Packet::Publish(p)) = notification {
                        let payload = String::from_utf8_lossy(&p.payload);
                        println!("\n[RECEPTION] Topic: {}", p.topic);
                        println!("[RECEPTION] Payload: {}", payload);
                    }
                }
                Err(e) => {
                    eprintln!("ERREUR MQTT: {:?}", e);
                    sleep(Duration::from_secs(5)).await;
                }
            }
        }
    });

    // 4. Test de publication (Log & Télémétrie)
    println!("Envoi des données de test...");

    // Log système
    mqtt_manager.publish("system", MqttSuffix::Logs, "Core online").await?;

    // Télémétrie fictive
    let data = serde_json::json!({
        "status": "connected",
        "load": 0.45,
        "uptime": 120
    });
    mqtt_manager.publish("internal_sensor", MqttSuffix::Get, data.to_string()).await?;

    println!("\nEn attente de messages externes (Ctrl+C pour quitter)...");
    println!("Essayez de publier sur: osheems/core/{}/any_device/set", host_id);

    // Maintien du programme en vie
    loop {
        sleep(Duration::from_secs(60)).await;
    }
}
