#[cfg(feature = "native")]
use rusqlite::Result;
use crate::db::MainDatabase;
use crate::relations::predicates;
use crate::users::UserRole;
use serde_json::json;

#[cfg(feature = "native")]
impl MainDatabase {
    pub fn bootstrap(&self) -> Result<()> {
        let system_id = "system";
        let eth0_id = "net_eth0";
        let mqtt_id = "mqtt_local";

        // 1. Vérification de l'existence (évite les doublons et les UNIQUE constraint failures)
        if self.get_entity(system_id).is_ok() {
            return Ok(());
        }

        println!("[BOOTSTRAP] Initializing OSHEEMS standardized environment...");

        // 2. Création de l'entité système (Root)
        self.create_entity(
            system_id,
            "system",
            None,
            Some("OSHEEMS Core System"),
                           Some("Root entity for system settings"),
                           &json!({}),
                           &json!({ "version": "0.1.0" }),
                           true // is_system
        )?;

        // 3. Création de l'Interface Physique (Le lien avec le hardware)
        self.create_entity(
            eth0_id,
            "interface",
            None,
            Some("Ethernet Interface (eth0)"),
                           Some("Physical network connection for OSHEEMS"),
                           &json!({ "name": "eth0" }),
                           &json!({ "driver": "standard_network" }),
                           true // is_system
        )?;

        // 4. Création de la Gateway MQTT locale (Le bus de communication interne)
        self.create_entity(
            mqtt_id,
            "gateway",
            None, // Utilise le template par défaut si non spécifié
            Some("OSHEEMS MQTT Broker"),
                           Some("Primary communication bus for OSHEEMS"),
                           &json!({
                               "broker_host": "192.168.5.11",
                               "broker_port": 1883,
                               "broker_topic": "osheems",
                               "client_id": "osheems_core",
                               "username": "oshems",
                               "password": "05H3M5" // La virgule a été ajoutée ici pour corriger la macro
                           }),
                           &json!({ "protocol": "mqtt" }),
                           true // is_system
        )?;

        // 5. RELATIONS : On tisse la toile avec le flag is_system à true

        // Système -> Gateway
        self.create_relation(
            system_id,
            predicates::HAS_GATEWAY,
            mqtt_id,
            &json!({ "is_core": true }),
                             true // is_system: relation structurelle
        )?;

        // Gateway -> Interface (Lien entre le protocole et le hardware)
        self.create_relation(
            mqtt_id,
            predicates::USES_INTERFACE,
            eth0_id,
            &json!({
                "priority": 1,
                "connection_type": "persistent"
            }),
            true // is_system: relation structurelle
        )?;

        // 6. Création des utilisateurs système
        self.create_user(
            "osheems",
            "05H33M5_hash",
            &UserRole::SuperAdmin.to_string(),
                         Some("Default root administrator account"),
                         true // is_system: entité protégée
        )?;

        self.create_user(
            "operator",
            "operator_hash",
            &UserRole::Operator.to_string(),
                         Some("Standard system operator account"),
                         true // is_system: entité protégée
        )?;

        // 7. Configuration initiale (Settings)
        self.create_setting(
            system_id,
            "system.name",
            &json!("My OSHEEMS Installation"),
                            true // is_system: réglage protégé
        )?;

        println!("[BOOTSTRAP] System architecture (Interface -> Gateway) and users initialized.");
        Ok(())
    }
}
