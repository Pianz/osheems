#[cfg(feature = "native")]
use rusqlite::{params, Connection, Result};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::path::Path;

use crate::entities::Entity;
use crate::relations::RelationRecord;
use crate::users::{User, UserRole};

#[cfg(feature = "native")]
pub struct MainDatabase {
    conn: Arc<Mutex<Connection>>,
}

#[cfg(feature = "native")]
impl MainDatabase {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA foreign_keys = ON;
        ")?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init_tables()?;
        Ok(db)
    }

    fn init_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch("
        CREATE TABLE IF NOT EXISTS entities (
            id            TEXT PRIMARY KEY,
            entity_type   TEXT NOT NULL,
            template_id   TEXT,
            label         TEXT,
            description   TEXT,
            configuration TEXT NOT NULL DEFAULT '{}',
            attributes    TEXT NOT NULL DEFAULT '{}',
            is_enabled    INTEGER DEFAULT 1,
            is_system     INTEGER DEFAULT 0,
            created_at    DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at    DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS relations (
            from_id       TEXT NOT NULL,
            predicate     TEXT NOT NULL,
            to_id         TEXT NOT NULL,
            attributes    TEXT NOT NULL DEFAULT '{}',
            is_system     INTEGER DEFAULT 0, -- Ajouté
            created_at    DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at    DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(from_id) REFERENCES entities(id) ON DELETE CASCADE,
                           FOREIGN KEY(to_id)   REFERENCES entities(id) ON DELETE CASCADE,
                           PRIMARY KEY (from_id, predicate, to_id)
        );

        CREATE TABLE IF NOT EXISTS settings (
            entity_id  TEXT NOT NULL,
            key        TEXT NOT NULL,
            value      TEXT NOT NULL,
            is_system  INTEGER DEFAULT 0, -- Ajouté
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (entity_id, key),
                           FOREIGN KEY(entity_id) REFERENCES entities(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS users (
            entity_id      TEXT PRIMARY KEY,
            username       TEXT UNIQUE NOT NULL,
            password_hash  TEXT NOT NULL,
            role           TEXT NOT NULL,
            last_login     DATETIME,
            FOREIGN KEY(entity_id) REFERENCES entities(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(entity_type);
        CREATE INDEX IF NOT EXISTS idx_entities_template ON entities(template_id);
        CREATE INDEX IF NOT EXISTS idx_relations_from ON relations(from_id);
        CREATE INDEX IF NOT EXISTS idx_relations_to ON relations(to_id);
        CREATE INDEX IF NOT EXISTS idx_relations_predicate ON relations(predicate);
        CREATE INDEX IF NOT EXISTS idx_settings_entity ON settings(entity_id);
        ")?;
        Ok(())
    }

    // --- ENTITIES CRUD ---

    pub fn create_entity(&self, id: &str, e_type: &str, tpl: Option<&str>, lbl: Option<&str>, desc: Option<&str>, conf: &Value, attrs: &Value, sys: bool) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO entities (id, entity_type, template_id, label, description, configuration, attributes, is_system) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                     params![id, e_type, tpl, lbl, desc, conf.to_string(), attrs.to_string(), if sys { 1 } else { 0 }],
        ).map(|_| ())
    }

    pub fn get_entity(&self, id: &str) -> Result<Entity> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, entity_type, template_id, label, description, configuration, attributes, is_enabled, is_system FROM entities WHERE id = ?",
            [id],
            |row| self.map_row_to_entity(row)
        )
    }

    pub fn get_all_entities(&self, entity_type: Option<&str>) -> Result<Vec<Entity>> {
        let conn = self.conn.lock().unwrap();
        let mut query = String::from("SELECT id, entity_type, template_id, label, description, configuration, attributes, is_enabled, is_system FROM entities");
        let mut params_list: Vec<String> = Vec::new();

        if let Some(t) = entity_type {
            query.push_str(" WHERE entity_type = ?");
            params_list.push(t.to_string());
        }

        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params_list), |row| self.map_row_to_entity(row))?;
        rows.collect()
    }

    pub fn get_entities_by_template(&self, template_id: &str) -> Result<Vec<Entity>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, entity_type, template_id, label, description, configuration, attributes, is_enabled, is_system FROM entities WHERE template_id = ?")?;
        let rows = stmt.query_map([template_id], |row| self.map_row_to_entity(row))?;
        rows.collect()
    }

    pub fn update_entity(&self, id: &str, label: Option<&str>, description: Option<&str>, configuration: &Value, attributes: &Value, is_enabled: bool) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE entities SET label = ?1, description = ?2, configuration = ?3, attributes = ?4, is_enabled = ?5, updated_at = CURRENT_TIMESTAMP WHERE id = ?6",
            params![label, description, configuration.to_string(), attributes.to_string(), if is_enabled { 1 } else { 0 }, id],
        ).map(|_| ())
    }

    pub fn delete_entity(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM entities WHERE id = ?", [id]).map(|_| ())
    }

    // --- RELATIONS CRUD ---

    pub fn create_relation(&self, from_id: &str, predicate: &str, to_id: &str, attributes: &Value, is_system: bool) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO relations (from_id, predicate, to_id, attributes, is_system) VALUES (?1, ?2, ?3, ?4, ?5)",
                     params![from_id, predicate, to_id, attributes.to_string(), if is_system { 1 } else { 0 }],
        ).map(|_| ())
    }

    pub fn get_relation(&self, from_id: &str, predicate: &str, to_id: &str) -> Result<RelationRecord> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT from_id, predicate, to_id, attributes, is_system FROM relations WHERE from_id = ?1 AND predicate = ?2 AND to_id = ?3",
            params![from_id, predicate, to_id],
            |row| self.map_row_to_relation(row)
        )
    }

    pub fn get_all_relations(&self, from_id: Option<&str>, predicate: Option<&str>, to_id: Option<&str>) -> Result<Vec<RelationRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut query = String::from("SELECT from_id, predicate, to_id, attributes, is_system FROM relations WHERE 1=1");
        let mut params_list: Vec<String> = Vec::new();

        if let Some(f) = from_id { query.push_str(" AND from_id = ?"); params_list.push(f.to_string()); }
        if let Some(p) = predicate { query.push_str(" AND predicate = ?"); params_list.push(p.to_string()); }
        if let Some(t) = to_id { query.push_str(" AND to_id = ?"); params_list.push(t.to_string()); }

        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params_list), |row| self.map_row_to_relation(row))?;
        rows.collect()
    }

    pub fn get_related_entities(&self, from_id: &str, predicate: &str) -> Result<Vec<Entity>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT e.id, e.entity_type, e.template_id, e.label, e.description, e.configuration, e.attributes, e.is_enabled, e.is_system
            FROM entities e
            JOIN relations r ON e.id = r.to_id
            WHERE r.from_id = ?1 AND r.predicate = ?2"
        )?;
        let rows = stmt.query_map(params![from_id, predicate], |row| self.map_row_to_entity(row))?;
        rows.collect()
    }

    pub fn update_relation(&self, from_id: &str, predicate: &str, to_id: &str, attributes: &Value) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE relations SET attributes = ?1, updated_at = CURRENT_TIMESTAMP WHERE from_id = ?2 AND predicate = ?3 AND to_id = ?4",
            params![attributes.to_string(), from_id, predicate, to_id],
        ).map(|_| ())
    }

    pub fn delete_relation(&self, from_id: &str, predicate: &str, to_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM relations WHERE from_id = ?1 AND predicate = ?2 AND to_id = ?3", params![from_id, predicate, to_id]).map(|_| ())
    }

    // --- SETTINGS CRUD ---

    pub fn create_setting(&self, entity_id: &str, key: &str, value: &Value, is_system: bool) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO settings (entity_id, key, value, is_system) VALUES (?1, ?2, ?3, ?4)",
                     params![entity_id, key, value.to_string(), if is_system { 1 } else { 0 }]
        ).map(|_| ())
    }

    pub fn get_setting(&self, entity_id: &str, key: &str) -> Result<(Value, bool)> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT value, is_system FROM settings WHERE entity_id = ?1 AND key = ?2",
            params![entity_id, key],
            |row| {
                let val_str: String = row.get(0)?;
                let sys: i32 = row.get(1)?;
                Ok((serde_json::from_str(&val_str).unwrap_or(Value::Null), sys == 1))
            }
        )
    }

    pub fn get_all_settings_for_entity(&self, entity_id: &str) -> Result<Vec<(String, Value, bool)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT key, value, is_system FROM settings WHERE entity_id = ?")?;
        let rows = stmt.query_map([entity_id], |row| {
            let k: String = row.get(0)?;
            let v: String = row.get(1)?;
            let sys: i32 = row.get(2)?;
            Ok((k, serde_json::from_str(&v).unwrap_or(Value::Null), sys == 1))
        })?;
        rows.collect()
    }

    pub fn update_setting(&self, entity_id: &str, key: &str, value: &Value) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("UPDATE settings SET value = ?1, updated_at = CURRENT_TIMESTAMP WHERE entity_id = ?2 AND key = ?3", params![value.to_string(), entity_id, key]).map(|_| ())
    }

    pub fn delete_setting(&self, entity_id: &str, key: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM settings WHERE entity_id = ?1 AND key = ?2", params![entity_id, key]).map(|_| ())
    }

    // --- USERS CRUD ---

    pub fn create_user(&self, username: &str, password_hash: &str, role: &str, description: Option<&str>, is_system: bool) -> Result<String> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let entity_id = format!("user_{}", username);

        tx.execute("INSERT INTO entities (id, entity_type, label, description, is_system) VALUES (?1, 'user', ?2, ?3, ?4)",
                   params![entity_id, username, description, if is_system { 1 } else { 0 }])?;

                   tx.execute("INSERT INTO users (entity_id, username, password_hash, role) VALUES (?1, ?2, ?3, ?4)",
                              params![entity_id, username, password_hash, role])?;

                              tx.commit()?;
                              Ok(entity_id)
    }

    pub fn get_user(&self, username: &str) -> Result<User> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT entity_id, username, role, last_login FROM users WHERE username = ?",
            [username],
            |row| {
                let role_str: String = row.get(2)?;
                Ok(User {
                    entity_id: row.get(0)?,
                   username: row.get(1)?,
                   role: UserRole::from(role_str),
                   last_login: row.get(3)?,
                })
            }
        )
    }

    pub fn get_all_users(&self) -> Result<Vec<User>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT entity_id, username, role, last_login FROM users")?;
        let rows = stmt.query_map([], |row| {
            let role_str: String = row.get(2)?;
            Ok(User {
                entity_id: row.get(0)?,
               username: row.get(1)?,
               role: UserRole::from(role_str),
               last_login: row.get(3)?,
            })
        })?;
        rows.collect()
    }

    pub fn update_user_password(&self, username: &str, new_hash: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("UPDATE users SET password_hash = ?1 WHERE username = ?2", params![new_hash, username]).map(|_| ())
    }

    pub fn update_user_login_date(&self, username: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Local::now().timestamp();
        conn.execute("UPDATE users SET last_login = ?1 WHERE username = ?2", params![now, username]).map(|_| ())
    }

    pub fn delete_user(&self, user_entity_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM entities WHERE id = ? AND entity_type = 'user'", [user_entity_id]).map(|_| ())
    }

    // --- MAPPERS ---

    fn map_row_to_entity(&self, row: &rusqlite::Row) -> Result<Entity> {
        let config_str: String = row.get(5)?;
        let attr_str: String = row.get(6)?;
        Ok(Entity {
            id: row.get(0)?,
           entity_type: row.get(1)?,
           template_id: row.get(2)?,
           label: row.get(3)?,
           description: row.get(4)?,
           configuration: serde_json::from_str(&config_str).unwrap_or(Value::Null),
           attributes: serde_json::from_str(&attr_str).unwrap_or(Value::Null),
           is_enabled: row.get(7).unwrap_or(1) == 1,
           is_system: row.get(8).unwrap_or(0) == 1,
        })
    }

    fn map_row_to_relation(&self, row: &rusqlite::Row) -> Result<RelationRecord> {
        let attr_str: String = row.get(3)?;
        Ok(RelationRecord {
            from_id: row.get(0)?,
           predicate: row.get(1)?,
           to_id: row.get(2)?,
           attributes: serde_json::from_str(&attr_str).unwrap_or(Value::Null),
           is_system: row.get(4).unwrap_or(0) == 1, // Ajouté
        })
    }
}
