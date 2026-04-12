use osheems_common::db::DatabaseManager;
use std::fs;
use std::path::PathBuf;

fn main() {
    // 1. Initialise les logs pour voir les erreurs SQL potentielles
    env_logger::init();

    println!("--- TEST DU BOOTSTRAP OSHEEMS ---");

    // 2. Nettoyage pour le test
    let db_path = PathBuf::from("database/main.db");
    if db_path.exists() {
        println!("🗑️ Suppression de l'ancienne base pour test...");
        let _ = fs::remove_file(&db_path);
    }

    // 3. Initialisation du DatabaseManager
    // L'appel à .new() déclenche automatiquement .bootstrap()
    println!("\n🚀 Lancement du DatabaseManager...");
    let db = match DatabaseManager::new() {
        Ok(manager) => {
            println!("✅ DatabaseManager initialisé avec succès.");
            manager
        },
        Err(e) => {
            eprintln!("❌ Échec critique du DatabaseManager : {:?}", e);
            return;
        }
    };

    // 4. Vérifications post-bootstrap
    println!("\n--- Vérification des données injectées ---");

    // Vérifier l'entité système
    match db.get_entity_by_id("system") {
        Ok(ent) => println!("✅ Entité Système trouvée : {} ({})", ent.id, ent.label.unwrap_or_default()),
        Err(e) => println!("❌ Entité Système manquante ! {:?}", e),
    }

    // Vérifier les réglages
    match db.get_setting("system", "system.version") {
        Ok(v) => println!("✅ Version du système : {}", v),
        Err(e) => println!("❌ Réglage system.version manquant ! {:?}", e),
    }

    // --- CORRECTION ICI ---
    // On appelle simplement la méthode existante du manager
    match db.get_user_by_username("osheems") {
        Ok(user) => {
            println!("✅ Utilisateur '{}' trouvé !", user.username);
            println!("   ID: {}", user.entity_id);
            println!("   Rôle: {:?}", user.role);
        },
        Err(e) => {
            println!("❌ Utilisateur 'osheems' manquant ou erreur de mapping !");
            println!("   Détail de l'erreur : {:?}", e);
        }
    }

    println!("\n--- Fin du test de Bootstrap ---");
}
