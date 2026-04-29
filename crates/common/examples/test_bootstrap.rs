use osheems_common::managers::database::DatabaseManager;
use std::fs;
use std::path::PathBuf;

fn main() {
    env_logger::init();
    println!("--- TEST DU BOOTSTRAP OSHEEMS ---");

    let db_path = PathBuf::from("database/main.db");
    if db_path.exists() {
        println!("🗑️ Suppression de l'ancienne base pour test...");
        let _ = fs::remove_file(&db_path);
    }

    println!("\n🚀 Lancement du DatabaseManager...");
    let db_manager = match DatabaseManager::new() {
        Ok(manager) => {
            println!("✅ DatabaseManager initialisé (Bootstrap automatique inclus).");
            manager
        },
        Err(e) => {
            eprintln!("❌ Échec critique : {:?}", e);
            return;
        }
    };

    println!("\n--- Vérification des données injectées ---");

    // Utilisation de .get_entity (nom mis à jour)
    match db_manager.main.get_entity("system") {
        Ok(ent) => println!("✅ Entité Système trouvée : {} ({:?})", ent.id, ent.label),
        Err(e) => println!("❌ Entité Système manquante ! {:?}", e),
    }

    // Utilisation de .get_setting
    match db_manager.main.get_setting("system", "system.version") {
        Ok(v) => println!("✅ Version du système : {:?}", v),
        Err(e) => println!("❌ Réglage system.version manquant ! {:?}", e),
    }

    // Utilisation de .get_user
    match db_manager.main.get_user("osheems") {
        Ok(user) => {
            println!("✅ Utilisateur '{}' trouvé !", user.username);
            println!("   Rôle: {:?}", user.role);
        },
        Err(e) => println!("❌ Utilisateur 'osheems' manquant : {:?}", e),
    }

    println!("\n--- Fin du test de Bootstrap ---");
}
