#[cfg(feature = "native")]
use rusqlite::{params, Connection, Result};
use std::path::{Path, PathBuf};
use chrono::{Local, NaiveDate, Duration};
use std::fs;

#[cfg(feature = "native")]
pub struct TelemetryRecord {
    pub timestamp: i64,
    pub entity_id: String,
    pub key: String,
    pub value: f64,
}

#[cfg(feature = "native")]
pub struct TelemetryStats {
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub count: i64,
    pub sum: f64,
}

#[cfg(feature = "native")]
pub struct TelemetryDatabase {
    base_path: PathBuf,
    conn: Option<Connection>,
    current_month: String,
}

#[cfg(feature = "native")]
impl TelemetryDatabase {
    pub fn new(base_path: &Path) -> Self {
        let _ = fs::create_dir_all(base_path);
        Self {
            base_path: base_path.to_path_buf(),
            conn: None,
            current_month: String::new(),
        }
    }

    fn ensure_connection(&mut self) -> Result<&Connection> {
        let now = Local::now();
        let month_str = now.format("%Y_%m").to_string();

        if self.conn.is_some() && self.current_month == month_str {
            return Ok(self.conn.as_ref().unwrap());
        }

        if !self.current_month.is_empty() && self.current_month != month_str {
            self.cleanup_old_files();
        }

        let db_name = format!("telemetry_{}.db", month_str);
        let db_path = self.base_path.join(db_name);

        let conn = Connection::open(db_path)?;
        conn.execute_batch("
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        CREATE TABLE IF NOT EXISTS telemetry (
            timestamp   INTEGER NOT NULL,
            entity_id   TEXT NOT NULL,
            key         TEXT NOT NULL,
            value       REAL NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_telemetry_query ON telemetry (entity_id, key, timestamp);
        ")?;

        self.conn = Some(conn);
        self.current_month = month_str;
        Ok(self.conn.as_ref().unwrap())
    }

    // --- TELEMETRY CRUD ---

    pub fn create_telemetry_entry(&mut self, entity_id: &str, key: &str, value: f64) -> Result<()> {
        let timestamp = Local::now().timestamp();
        let conn = self.ensure_connection()?;
        conn.execute(
            "INSERT INTO telemetry (timestamp, entity_id, key, value) VALUES (?1, ?2, ?3, ?4)",
                     params![timestamp, entity_id, key, value],
        )?;
        Ok(())
    }

    pub fn get_last_telemetry_entry(&mut self, entity_id: &str, key: &str) -> Result<TelemetryRecord> {
        let conn = self.ensure_connection()?;
        conn.query_row(
            "SELECT timestamp, entity_id, key, value FROM telemetry
            WHERE entity_id = ?1 AND key = ?2 ORDER BY timestamp DESC LIMIT 1",
            params![entity_id, key],
            |row| Ok(TelemetryRecord {
                timestamp: row.get(0)?,
                     entity_id: row.get(1)?,
                     key: row.get(2)?,
                     value: row.get(3)?,
            })
        )
    }

    pub fn get_all_telemetry_by_range(&mut self, entity_id: &str, key: &str, start_ts: i64, end_ts: i64) -> Result<Vec<TelemetryRecord>> {
        let conn = self.ensure_connection()?;
        let mut stmt = conn.prepare(
            "SELECT timestamp, entity_id, key, value FROM telemetry
            WHERE entity_id = ?1 AND key = ?2 AND timestamp BETWEEN ?3 AND ?4
            ORDER BY timestamp ASC"
        )?;

        let rows = stmt.query_map(params![entity_id, key, start_ts, end_ts], |row| {
            Ok(TelemetryRecord {
                timestamp: row.get(0)?,
               entity_id: row.get(1)?,
               key: row.get(2)?,
               value: row.get(3)?,
            })
        })?;

        rows.collect()
    }

    pub fn get_telemetry_stats_by_range(&mut self, entity_id: &str, key: &str, start_ts: i64, end_ts: i64) -> Result<TelemetryStats> {
        let conn = self.ensure_connection()?;
        conn.query_row(
            "SELECT MIN(value), MAX(value), AVG(value), COUNT(value), SUM(value) FROM telemetry
            WHERE entity_id = ?1 AND key = ?2 AND timestamp BETWEEN ?3 AND ?4",
            params![entity_id, key, start_ts, end_ts],
            |row| Ok(TelemetryStats {
                min: row.get(0).unwrap_or(0.0),
                     max: row.get(1).unwrap_or(0.0),
                     avg: row.get(2).unwrap_or(0.0),
                     count: row.get(3).unwrap_or(0),
                     sum: row.get(4).unwrap_or(0.0),
            })
        )
    }

    pub fn delete_telemetry_for_entity(&mut self, entity_id: &str) -> Result<()> {
        let conn = self.ensure_connection()?;
        conn.execute("DELETE FROM telemetry WHERE entity_id = ?", [entity_id])?;
        Ok(())
    }

    // --- MAINTENANCE ---

    pub fn cleanup_old_files(&self) {
        if let Ok(entries) = fs::read_dir(&self.base_path) {
            let limit_date = Local::now().naive_local().date() - Duration::days(365);
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name.starts_with("telemetry_") && file_name.ends_with(".db") {
                        if let Some(date_part) = file_name.strip_prefix("telemetry_").and_then(|s| s.strip_suffix(".db")) {
                            if let Ok(file_date) = NaiveDate::parse_from_str(&format!("{}_01", date_part), "%Y_%m_%d") {
                                if file_date < limit_date {
                                    let _ = fs::remove_file(path);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
