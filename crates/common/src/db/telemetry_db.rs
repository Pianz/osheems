#[cfg(feature = "native")]
use rusqlite::{params, Connection, Result};
use std::path::{Path, PathBuf};
use chrono::{Local, NaiveDate, Duration};
use std::fs;

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

    /// Fournit un accès temporaire à la connexion du mois en cours
    pub fn with_conn<F, T>(&mut self, f: F) -> Result<T>
    where
    F: FnOnce(&Connection) -> T,
    {
        let conn = self.ensure_connection()?;
        Ok(f(conn))
    }

    fn ensure_connection(&mut self) -> Result<&Connection> {
        let now = Local::now();
        let month_str = now.format("%Y_%m").to_string();

        if self.conn.is_some() && self.current_month == month_str {
            return Ok(self.conn.as_ref().unwrap());
        }

        if self.current_month != month_str {
            self.cleanup_old_files();
        }

        let db_name = format!("telemetry_{}.db", month_str);
        let db_path = self.base_path.join(db_name);

        let conn = Connection::open(db_path)?;

        // --- OPTIMISATION WAL ---
        conn.execute_batch("
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        ")?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS telemetry (
                timestamp   INTEGER NOT NULL,
                entity_id   INTEGER NOT NULL,
                key         TEXT NOT NULL,
                value       REAL NOT NULL
        )",
        [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_telemetry_query ON telemetry (entity_id, key, timestamp)",
                     []
        )?;

        self.conn = Some(conn);
        self.current_month = month_str;

        Ok(self.conn.as_ref().unwrap())
    }

    pub fn insert_data(&mut self, entity_id: i64, key: &str, value: f64) -> Result<()> {
        let timestamp = Local::now().timestamp();
        let conn = self.ensure_connection()?;

        conn.execute(
            "INSERT INTO telemetry (timestamp, entity_id, key, value) VALUES (?1, ?2, ?3, ?4)",
                     params![timestamp, entity_id, key, value],
        )?;
        Ok(())
    }

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
                                    println!("[MAINTENANCE] Deleting obsolete data: {}", file_name);
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
