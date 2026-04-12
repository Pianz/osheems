#[cfg(feature = "native")]
use rusqlite::Result;
use crate::db::DatabaseManager;
use crate::users::UserRole;
use serde_json::json;

impl DatabaseManager {
    pub fn bootstrap(&self) -> Result<()> {
        // On aide le compilateur en spécifiant le type de retour de la closure : Result<bool>
        let system_exists = self.main.with_conn(|conn| -> Result<bool> {
            let mut stmt = conn.prepare("SELECT 1 FROM entities WHERE id = 1")?;
            Ok(stmt.exists([])?)
        })?;

        if system_exists {
            return Ok(());
        }

        println!("[BOOTSTRAP] Initializing OSHEEMS default environment...");

        // On crée l'entité système (ID 1)
        self.main.with_conn(|conn| -> Result<()> {
            conn.execute(
                "INSERT INTO entities (id, entity_type, name, label, is_system)
            VALUES (1, 'system', 'osheems_core', 'OSHEEMS Core System', 1)",
                         [],
            )?;
            Ok(())
        })?;

        // Création des utilisateurs via la façade (déjà en Result<i64>)
        self.create_user("osheems", "05H33M5", UserRole::SuperAdmin)?;
        self.create_user("operator", "1234", UserRole::Operator)?;

        // Configuration par défaut via la façade
        self.set_setting(1, "system.version", &json!("0.1.0"))?;
        self.set_setting(1, "system.name", &json!("My OSHEEMS Installation"))?;

        println!("[BOOTSTRAP] Default users created successfully.");
        Ok(())
    }
}
