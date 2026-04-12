#[cfg(feature = "native")]
use rusqlite::{params, Connection, Result};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::path::Path;

use crate::entities::Entity;
use crate::relations::RelationRecord;

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

    pub fn with_conn<F, T>(&self, f: F) -> T
    where
    F: FnOnce(&Connection) -> T
    {
        let conn = self.conn.lock().unwrap();
        f(&conn)
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
            is_enabled    INTEGER DEFAULT 1,
            is_system     INTEGER DEFAULT 0,
            created_at    DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at    DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS relations (
            from_id        TEXT NOT NULL,
            predicate      TEXT NOT NULL,
            to_id          TEXT NOT NULL,
            metadata       TEXT DEFAULT '{}',
            FOREIGN KEY(from_id) REFERENCES entities(id) ON DELETE CASCADE,
                           FOREIGN KEY(to_id)   REFERENCES entities(id) ON DELETE CASCADE,
                           PRIMARY KEY (from_id, predicate, to_id)
        );

        CREATE TABLE IF NOT EXISTS settings (
            entity_id  TEXT NOT NULL,
            key        TEXT NOT NULL,
            value      TEXT NOT NULL,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (entity_id, key),
                           FOREIGN KEY(entity_id) REFERENCES entities(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS users (
            entity_id     TEXT PRIMARY KEY,
            username      TEXT UNIQUE NOT NULL,
            password_hash TEXT NOT NULL,
            role          TEXT NOT NULL,
            last_login    DATETIME,
            FOREIGN KEY(entity_id) REFERENCES entities(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(entity_type);
        CREATE INDEX IF NOT EXISTS idx_relations_to ON relations(to_id);
        CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
        ")?;
        Ok(())
    }

    // --- GESTION DES ENTITÉS ---

    pub fn create_entity(
        &self,
        id: &str,
        entity_type: &str,
        template_id: Option<&str>,
        label: Option<&str>,
        description: Option<&str>,
        config: &Value,
        is_system: bool
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO entities (id, entity_type, template_id, label, description, configuration, is_system)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                     params![
                         id,
                     entity_type,
                     template_id,
                     label,
                     description,
                     config.to_string(),
                     if is_system { 1 } else { 0 }
                     ],
        )?;
        Ok(())
    }

    pub fn get_entity_by_id(&self, id: &str) -> Result<Entity> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, entity_type, template_id, label, description, configuration, is_enabled, is_system FROM entities WHERE id = ?",
            [id],
            |row| self.map_row_to_entity(row)
        )
    }

    pub fn get_entities(&self, entity_type: Option<&str>) -> Result<Vec<Entity>> {
        let conn = self.conn.lock().unwrap();
        let mut query = String::from("SELECT id, entity_type, template_id, label, description, configuration, is_enabled, is_system FROM entities");
        let mut params_list: Vec<String> = Vec::new();

        if let Some(t) = entity_type {
            query.push_str(" WHERE entity_type = ?");
            params_list.push(t.to_string());
        }

        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params_list), |row| {
            self.map_row_to_entity(row)
        })?;
        rows.collect()
    }

    pub fn update_entity(&self, id: &str, label: Option<&str>, config: &Value, is_enabled: bool) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE entities SET label = ?1, configuration = ?2, is_enabled = ?3, updated_at = CURRENT_TIMESTAMP WHERE id = ?4",
            params![label, config.to_string(), is_enabled, id],
        )?;
        Ok(())
    }

    pub fn delete_entity(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM entities WHERE id = ?1", [id])?;
        if affected == 0 { return Err(rusqlite::Error::QueryReturnedNoRows); }
        Ok(())
    }

    // --- GESTION DES SETTINGS ---

    pub fn set_setting(&self, entity_id: &str, key: &str, value: &Value) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO settings (entity_id, key, value, updated_at) VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP)",
                     params![entity_id, key, value.to_string()],
        )?;
        Ok(())
    }

    pub fn get_setting(&self, entity_id: &str, key: &str) -> Result<Value> {
        let conn = self.conn.lock().unwrap();
        let val_str: String = conn.query_row(
            "SELECT value FROM settings WHERE entity_id = ?1 AND key = ?2",
            params![entity_id, key],
            |row| row.get(0),
        )?;
        Ok(serde_json::from_str(&val_str).unwrap_or(Value::Null))
    }

    // --- GESTION DES UTILISATEURS ---

    pub fn create_user(&self, username: &str, password_hash: &str, role: &str, description: Option<&str>, is_system: bool) -> Result<String> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        let entity_id = format!("user_{}", username);

        // Insertion de l'entité avec la description fournie
        tx.execute(
            "INSERT INTO entities (id, entity_type, label, description, is_system) VALUES (?1, 'user', ?2, ?3, ?4)",
                   params![entity_id, username, description, if is_system { 1 } else { 0 }]
        )?;

        tx.execute(
            "INSERT INTO users (entity_id, username, password_hash, role) VALUES (?1, ?2, ?3, ?4)",
                   params![entity_id, username, password_hash, role],
        )?;

        tx.commit()?;
        Ok(entity_id)
    }

    pub fn delete_user(&self, user_entity_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute(
            "DELETE FROM entities WHERE id = ?1 AND entity_type = 'user'",
            [user_entity_id],
        )?;
        if affected == 0 { return Err(rusqlite::Error::QueryReturnedNoRows); }
        Ok(())
    }

    // --- GESTION DES RELATIONS ---

    pub fn create_relation(&self, from_id: &str, predicate: &str, to_id: &str, metadata: &Value) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO relations (from_id, predicate, to_id, metadata) VALUES (?1, ?2, ?3, ?4)",
                     params![from_id, predicate, to_id, metadata.to_string()],
        )?;
        Ok(())
    }

    pub fn get_relations(&self, from_id: Option<&str>, predicate: Option<&str>, to_id: Option<&str>) -> Result<Vec<RelationRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut query = String::from("SELECT from_id, predicate, to_id, metadata FROM relations WHERE 1=1");
        let mut params_list: Vec<rusqlite::types::Value> = Vec::new();

        if let Some(f) = from_id {
            query.push_str(" AND from_id = ?");
            params_list.push(rusqlite::types::Value::Text(f.to_string()));
        }
        if let Some(p) = predicate {
            query.push_str(" AND predicate = ?");
            params_list.push(rusqlite::types::Value::Text(p.to_string()));
        }
        if let Some(t) = to_id {
            query.push_str(" AND to_id = ?");
            params_list.push(rusqlite::types::Value::Text(t.to_string()));
        }

        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params_list), |row| {
            let meta_str: String = row.get(3)?;
            Ok(RelationRecord {
                from_id: row.get(0)?,
               predicate: row.get(1)?,
               to_id: row.get(2)?,
               metadata: serde_json::from_str(&meta_str).unwrap_or(Value::Null),
            })
        })?;
        rows.collect()
    }

    pub fn delete_relation(&self, from_id: &str, predicate: &str, to_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute(
            "DELETE FROM relations WHERE from_id = ?1 AND predicate = ?2 AND to_id = ?3",
            params![from_id, predicate, to_id],
        )?;
        if affected == 0 { return Err(rusqlite::Error::QueryReturnedNoRows); }
        Ok(())
    }

    fn map_row_to_entity(&self, row: &rusqlite::Row) -> Result<Entity> {
        let config_str: String = row.get(5)?;
        Ok(Entity {
            id: row.get(0)?,
           entity_type: row.get(1)?,
           template_id: row.get(2)?,
           label: row.get(3)?,
           description: row.get(4)?,
           configuration: serde_json::from_str(&config_str).unwrap_or(Value::Null),
           is_enabled: row.get(6).unwrap_or(1) == 1,
           is_system: row.get(7).unwrap_or(0) == 1,
        })
    }
}
