#[cfg(feature = "native")]
use rusqlite::Result;
use crate::db::DatabaseManager;
use crate::users::UserRole;
use serde_json::json;

impl DatabaseManager {
    pub fn bootstrap(&self) -> Result<()> {
        let system_id = "system";

        // 1. Vérification de l'existence
        if self.get_entity_by_id(system_id).is_ok() {
            return Ok(());
        }

        println!("[BOOTSTRAP] Initializing OSHEEMS default environment...");

        // 2. Création de l'entité système complète
        self.create_entity(
            system_id,
            "system",
            None,
            Some("OSHEEMS Core System"),
            Some("Root entity for system settings"),
            &json!({}),
            true
        )?;

        // 3. Création des utilisateurs système avec descriptions
        self.create_user(
            "osheems",
            "05H33M5",
            UserRole::SuperAdmin,
            Some("Default root administrator account"),
            true
        )?;

        self.create_user(
            "operator",
            "1234",
            UserRole::Operator,
            Some("Standard system operator account"),
            true
        )?;

        // 4. Configuration initiale
        self.set_setting(system_id, "system.version", &json!("0.1.0"))?;
        self.set_setting(system_id, "system.name", &json!("My OSHEEMS Installation"))?;

        println!("[BOOTSTRAP] Default environment and users created successfully.");
        Ok(())
    }
}
