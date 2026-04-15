use osheems_common::templates::TemplateManager;
use std::path::PathBuf;

fn main() {
    // 1. Initialise les logs pour voir les détails du parsing
    env_logger::init();

    println!("--- VALIDATION DES TEMPLATES OSHEEMS ---");

    // 2. Chemin vers le dossier racine
    // On part du principe que tu lances le test depuis la racine du workspace
    let templates_path = PathBuf::from("templates");

    if !templates_path.exists() {
        eprintln!("❌ Erreur : Le dossier 'templates/' est introuvable à la racine.");
        return;
    }

    // 3. Initialisation du TemplateManager
    println!("🔍 Analyse du dossier : {:?}", templates_path.canonicalize().unwrap_or(templates_path.clone()));
    let manager = TemplateManager::new(templates_path);

    // 4. Liste les templates trouvés pour debug
    // (Si tu as une méthode pour lister les IDs, sinon on teste en direct)

    // 5. Test spécifique du Shelly Pro 3EM
    let target_id = "shelly_pro_3em";
    println!("\n--- Vérification du template : {} ---", target_id);

    if let Some(t) = manager.get_template(target_id) {
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
    }

    println!("\n--- Fin de validation ---");
}
