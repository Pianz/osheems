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
//use crate::settings::Setting;

#[cfg(feature = "native")]
pub struct DatabaseManager {
    pub main: MainDatabase,
    pub telemetry: Arc<Mutex<TelemetryDatabase>>,
}

#[cfg(feature = "native")]
impl DatabaseManager {
    pub fn new() -> Result<Self> {
        // 1. Détermination du chemin de stockage (Standard Snap ou Local)
        let db_dir = match env::var("SNAP_DATA") {
            Ok(val) => PathBuf::from(val),
            Err(_) => PathBuf::from("database"),
        };

        // 2. Initialisation des bases
        let main_path = db_dir.join("main.db");
        let main = MainDatabase::open(&main_path)?;
        let telemetry = TelemetryDatabase::new(&db_dir);

        let db_manager = Self {
            main,
            telemetry: Arc::new(Mutex::new(telemetry)),
        };

        // 3. Initialisation automatique des données de base (System + Users par défaut)
        db_manager.bootstrap()?;

        Ok(db_manager)
    }

    // --- ENTITIES ---

    pub fn create_entity(&self, entity_type: &str, name: &str, template_id: Option<&str>, config: &serde_json::Value) -> Result<i64> {
        self.main.create_entity(entity_type, name, template_id, config)
    }

    pub fn get_entity_by_id(&self, id: i64) -> Result<Entity> {
        self.main.get_entity_by_id(id)
    }

    pub fn get_entity_by_name(&self, name: &str) -> Result<Entity> {
        self.main.get_entity_by_name(name)
    }

    pub fn get_entities(&self, entity_type: Option<&str>) -> Result<Vec<Entity>> {
        self.main.get_entities(entity_type)
    }

    pub fn update_entity(&self, id: i64, label: Option<&str>, config: &serde_json::Value, is_enabled: bool) -> Result<()> {
        self.main.update_entity(id, label, config, is_enabled)
    }

    pub fn delete_entity(&self, id: i64) -> Result<()> {
        self.main.delete_entity(id)
    }

    // --- RELATIONS ---

    pub fn create_relation(&self, from_id: i64, predicate: &str, to_id: i64, metadata: &serde_json::Value) -> Result<()> {
        self.main.create_relation(from_id, predicate, to_id, metadata)
    }

    pub fn get_relations(&self, from_id: Option<i64>, predicate: Option<&str>, to_id: Option<i64>) -> Result<Vec<RelationRecord>> {
        self.main.get_relations(from_id, predicate, to_id)
    }

    pub fn get_relations_by_from_id(&self, from_id: i64) -> Result<Vec<RelationRecord>> {
        self.main.get_relations_by_from_id(from_id)
    }

    pub fn get_relations_by_to_id(&self, to_id: i64) -> Result<Vec<RelationRecord>> {
        self.main.get_relations_by_to_id(to_id)
    }

    pub fn get_relations_by_predicate(&self, predicate: &str) -> Result<Vec<RelationRecord>> {
        self.main.get_relations_by_predicate(predicate)
    }

    pub fn delete_relation(&self, from_id: i64, predicate: &str, to_id: i64) -> Result<()> {
        self.main.delete_relation(from_id, predicate, to_id)
    }

    // --- TELEMETRY ---

    pub fn log_data(&self, entity_id: i64, key: &str, value: f64) -> Result<()> {
        let mut tel = self.telemetry.lock().unwrap();
        tel.insert_data(entity_id, key, value)
    }

    pub fn get_current_month_telemetry(&self, entity_id: i64, key: &str) -> Result<Vec<(i64, f64)>> {
        let mut tel = self.telemetry.lock().unwrap();

        tel.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT timestamp, value FROM telemetry WHERE entity_id = ?1 AND key = ?2 ORDER BY timestamp ASC"
            )?;

            let rows = stmt.query_map(params![entity_id, key], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?;

            rows.collect::<Result<Vec<(i64, f64)>>>()
        })?
    }

    // --- SETTINGS ---

    pub fn set_setting(&self, entity_id: i64, key: &str, value: &serde_json::Value) -> Result<()> {
        self.main.set_setting(entity_id, key, value)
    }

    pub fn get_setting(&self, entity_id: i64, key: &str) -> Result<serde_json::Value> {
        self.main.get_setting(entity_id, key)
    }

    // --- USERS ---

    pub fn create_user(&self, username: &str, password_hash: &str, role: UserRole) -> Result<i64> {
        self.main.create_user(username, password_hash, &role.to_string())
    }

    pub fn delete_user(&self, user_entity_id: i64) -> Result<()> {
        self.main.delete_user(user_entity_id)
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
