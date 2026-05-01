// crates/common/src/managers/database/manager.rs

use crate::db::{MainDatabase, TelemetryDatabase};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::env;
use rusqlite::Result;
use log::{info, error};

pub struct DatabaseManager {
    /// Direct access to the main configuration database
    pub main: MainDatabase,
    /// Thread-safe access to telemetry for file rotation handling
    pub telemetry: Arc<Mutex<TelemetryDatabase>>,
}

impl DatabaseManager {
    /// Initializes the database management layer for OSHEEMS
    pub fn new() -> Result<Self> {
        // Determine storage directory (Snap environment or local fallback)
        let db_dir = match env::var("SNAP_DATA") {
            Ok(val) => PathBuf::from(val),
            Err(_) => PathBuf::from("database"),
        };

        // Create directory if it does not exist (crucial for first run)
        if !db_dir.exists() {
            if let Err(e) = std::fs::create_dir_all(&db_dir) {
                error!("[DB_MGR] Failed to create database directory: {}", e);
            }
        }

        // Initialize low-level database handlers
        // Using "main.db" as per your previous test files
        let main_path = db_dir.join("main.db");
        let main = MainDatabase::open(&main_path)?;
        let telemetry = TelemetryDatabase::new(&db_dir);

        let manager = Self {
            main,
            telemetry: Arc::new(Mutex::new(telemetry)),
        };

        // Execute bootstrap via the DB layer
        // Ensure that in bootstrap(), vital system components
        // use 'true' for the system-protected flag.
        info!("[DB_MGR] Running database bootstrap...");
        manager.main.bootstrap()?;

        info!("[DB_MGR] Database systems initialized at: {:?}", db_dir);
        Ok(manager)
    }

    /// Helper to perform manual maintenance on telemetry files
    pub fn cleanup_telemetry(&self) {
        match self.telemetry.lock() {
            Ok(tel) => {
                info!("[DB_MGR] Starting telemetry maintenance...");
                tel.cleanup_old_files();
            },
            Err(e) => error!("[DB_MGR] Failed to lock telemetry for cleanup: {}", e),
        }
    }
}
