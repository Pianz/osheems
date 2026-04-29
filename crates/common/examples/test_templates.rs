use osheems_common::managers::template::TemplateManager; // Import mis à jour
use std::path::PathBuf;

#[tokio::main] // Nécessaire car get_template est async
async fn main() {
    // 1. Initialise les logs pour voir les détails du parsing
    // Assure-je que env_logger est dans ton Cargo.toml
    env_logger::init();

    println!("--- VALIDATION DES TEMPLATES OSHEEMS ---");

    // 2. Chemin vers le dossier racine
    let templates_path = PathBuf::from("templates");

    if !templates_path.exists() {
        eprintln!("❌ Erreur : Le dossier 'templates/' est introuvable à la racine.");
        return;
    }

    // 3. Initialisation du TemplateManager
    // Note: reload_sync est appelé dans le new()
    println!("🔍 Analyse du dossier : {:?}", templates_path.canonicalize().unwrap_or(templates_path.clone()));
    let manager = TemplateManager::new(templates_path);

    // 4. Test spécifique du Shelly Pro 3EM
    let target_id = "shelly_pro_3em";
    println!("\n--- Vérification du template : {} ---", target_id);

    // Ajout du .await car l'accès au RwLock est asynchrone
    if let Some(t) = manager.get_template(target_id).await {
        println!("✅ Template trouvé !");
        println!("   Identité : {} - {}", t.identity.brand, t.identity.model);
        println!("   Type d'entité : {}", t.entity_type);

        // Vérification des points (states/actions)
        let states_count = t.points.states.len();
        let actions_count = t.points.actions.len();
        println!("   Points définis : {} états, {} actions", states_count, actions_count);

        // Vérification des mappings protocolaires
        if !t.mappings.is_empty() {
            let protocols: Vec<_> = t.mappings.keys().collect();
            println!("✅ Mappings détectés : {:?}", protocols);

            for proto in protocols {
                match proto.as_str() {
                    "modbus" => println!("     -> Configuration Modbus valide."),
                    "mqtt" => println!("     -> Configuration MQTT valide."),
                    _ => println!("     -> Protocole '{}' détecté.", proto),
                }
            }
        } else {
            println!("⚠️ Attention : Aucun mapping (Modbus/MQTT) trouvé pour ce template.");
        }
    } else {
        println!("❌ Erreur : Le template '{}' n'a pas été chargé.", target_id);

        // Aide au debug : Lister ce qui a été trouvé
        let all_templates = manager.list_templates().await;
        if all_templates.is_empty() {
            println!("   (Aucun template n'est chargé dans le manager)");
        } else {
            println!("   Templates disponibles : {:?}",
                     all_templates.iter().map(|t| &t.template_id).collect::<Vec<_>>()
            );
        }
    }

    println!("\n--- Fin de validation ---");
}
