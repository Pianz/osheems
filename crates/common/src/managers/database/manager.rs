use crate::db::{MainDatabase, TelemetryDatabase};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::env;
use rusqlite::Result;

pub struct DatabaseManager {
    /// Accès direct à la base principale
    pub main: MainDatabase,
    /// Accès à la télémétrie protégé par Mutex pour la rotation des fichiers
    pub telemetry: Arc<Mutex<TelemetryDatabase>>,
}

impl DatabaseManager {
    /// Initialise le gestionnaire de base de données
    pub fn new() -> Result<Self> {
        // Détermination du dossier de stockage (Snap ou local)
        let db_dir = match env::var("SNAP_DATA") {
            Ok(val) => PathBuf::from(val),
            Err(_) => PathBuf::from("database"),
        };

        // Création du dossier si inexistant (important pour le premier lancement)
        if !db_dir.exists() {
            std::fs::create_dir_all(&db_dir).ok();
        }

        // Initialisation des bases de bas niveau
        // Note: J'utilise "main.db" pour correspondre à ton fichier de test de hier
        let main_path = db_dir.join("main.db");
        let main = MainDatabase::open(&main_path)?;
        let telemetry = TelemetryDatabase::new(&db_dir);

        let manager = Self {
            main,
            telemetry: Arc::new(Mutex::new(telemetry)),
        };

        // Exécution du bootstrap via la couche DB
        // IMPORTANT: Vérifie que dans ton implémentation de bootstrap(),
        // les appels à create_relation et create_setting passent bien le flag 'true'
        // pour les composants vitaux du système.
        manager.main.bootstrap()?;

        Ok(manager)
    }

    /// Helper pour effectuer une maintenance manuelle sur les fichiers de télémétrie
    pub fn cleanup_telemetry(&self) {
        let tel = self.telemetry.lock().unwrap();
        tel.cleanup_old_files();
    }
}
