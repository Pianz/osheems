// crates/common/examples/test_interface_manager.rs

use osheems_common::managers::interface::InterfaceManager;
use osheems_common::managers::database::DatabaseManager;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialisation des logs
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("--- OSHEEMS : TEST DÉTECTION HARDWARE ---");

    // 2. Création du DatabaseManager (selon ta signature : pas d'argument, pas d'await)
    // Note : Comme ton new() ne prend pas de chemin, il utilise probablement
    // le fichier par défaut 'oshems_main.db' défini dans ton projet.
    let db_inner = DatabaseManager::new()?;
    let db_manager = Arc::new(db_inner);

    // 3. Création de l'InterfaceManager
    let manager = InterfaceManager::new(db_manager.clone());

    // 4. Lancement du cycle : scan_hardware() + load_enabled_interfaces()
    // C'est ici que le code va détecter tes ports série et les écrire en DB
    println!("🔍 Scan du matériel en cours...");
    if let Err(e) = manager.start().await {
        eprintln!("❌ Erreur lors de l'exécution : {}", e);
    }

    // 5. Vérification du résultat
    // On récupère tout ce qui a été classé comme "interface" dans la table entities
    println!("\n📊 Résultats de la détection automatique :");
    match db_manager.main.get_all_entities(Some("interface")) {
        Ok(entities) => {
            if entities.is_empty() {
                println!("ℹ️ Aucune interface détectée (vérifiez vos droits d'accès au matériel).");
            } else {
                for entity in entities {
                    println!("📍 ID: {} | Driver: {} | Activé: {}",
                             entity.id,
                             entity.configuration.get("driver").and_then(|v| v.as_str()).unwrap_or("inconnu"),
                             entity.is_enabled
                    );
                }
            }
        },
        Err(e) => eprintln!("❌ Impossible de lire la table entities : {}", e),
    }

    println!("\n--- Fin du test ---");
    Ok(())
}
