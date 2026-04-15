pub mod main_db;
pub mod telemetry_db;
pub mod bootstrap;

#[cfg(feature = "native")]
use rusqlite::{params, Result};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::env;

pub use main_db::MainDatabase;
pub use telemetry_db::TelemetryDatabase;
use crate::entities::Entity;
use crate::relations::RelationRecord;
use crate::users::{User, UserRole};

#[cfg(feature = "native")]
pub struct DatabaseManager {
    pub main: MainDatabase,
    pub telemetry: Arc<Mutex<TelemetryDatabase>>,
}

#[cfg(feature = "native")]
impl DatabaseManager {
    pub fn new() -> Result<Self> {
        let db_dir = match env::var("SNAP_DATA") {
            Ok(val) => PathBuf::from(val),
            Err(_) => PathBuf::from("database"),
        };

        let main_path = db_dir.join("main.db");
        let main = MainDatabase::open(&main_path)?;
        let telemetry = TelemetryDatabase::new(&db_dir);

        let db_manager = Self {
            main,
            telemetry: Arc::new(Mutex::new(telemetry)),
        };

        db_manager.bootstrap()?;

        Ok(db_manager)
    }

    // --- ENTITIES ---

    pub fn create_entity(
        &self,
        id: &str,
        entity_type: &str,
        template_id: Option<&str>,
        label: Option<&str>,
        description: Option<&str>,
        config: &serde_json::Value,
        is_system: bool
    ) -> Result<()> {
        self.main.create_entity(id, entity_type, template_id, label, description, config, is_system)
    }

    pub fn get_entity_by_id(&self, id: &str) -> Result<Entity> {
        self.main.get_entity_by_id(id)
    }

    pub fn get_entities(&self, entity_type: Option<&str>) -> Result<Vec<Entity>> {
        self.main.get_entities(entity_type)
    }

    pub fn update_entity(&self, id: &str, label: Option<&str>, config: &serde_json::Value, is_enabled: bool) -> Result<()> {
        self.main.update_entity(id, label, config, is_enabled)
    }

    pub fn delete_entity(&self, id: &str) -> Result<()> {
        self.main.delete_entity(id)
    }

    // --- RELATIONS ---

    pub fn create_relation(&self, from_id: &str, predicate: &str, to_id: &str, metadata: &serde_json::Value) -> Result<()> {
        self.main.create_relation(from_id, predicate, to_id, metadata)
    }

    pub fn get_relations(&self, from_id: Option<&str>, predicate: Option<&str>, to_id: Option<&str>) -> Result<Vec<RelationRecord>> {
        self.main.get_relations(from_id, predicate, to_id)
    }

    pub fn delete_relation(&self, from_id: &str, predicate: &str, to_id: &str) -> Result<()> {
        self.main.delete_relation(from_id, predicate, to_id)
    }

    // --- SETTINGS ---

    pub fn set_setting(&self, entity_id: &str, key: &str, value: &serde_json::Value) -> Result<()> {
        self.main.set_setting(entity_id, key, value)
    }

    pub fn get_setting(&self, entity_id: &str, key: &str) -> Result<serde_json::Value> {
        self.main.get_setting(entity_id, key)
    }

    // --- USERS ---

    pub fn create_user(&self, username: &str, password_hash: &str, role: UserRole, description: Option<&str>, is_system: bool) -> Result<String> {
        self.main.create_user(username, password_hash, &role.to_string(), description, is_system)
    }

    pub fn get_user_by_username(&self, username: &str) -> Result<User> {
        self.main.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT entity_id, username, role, last_login FROM users WHERE username = ?"
            )?;

            stmt.query_row([username], |row| {
                let role_str: String = row.get(2)?;
                Ok(User {
                    entity_id: row.get(0)?,
                   username: row.get(1)?,
                   role: UserRole::from(role_str),
                   last_login: row.get(3)?,
                })
            })
        })
    }
}
