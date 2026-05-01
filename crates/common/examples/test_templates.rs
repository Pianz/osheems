use osheems_common::managers::template::TemplateManager;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    env_logger::init();

    println!("--- VALIDATION DES TEMPLATES OSHEEMS ---");

    let templates_path = PathBuf::from("templates");

    if !templates_path.exists() {
        eprintln!("❌ Erreur : Le dossier 'templates/' est introuvable.");
        return;
    }

    let manager = TemplateManager::new(templates_path);

    let target_id = "devices/shelly/pro3em";
    println!("\n--- Vérification du template : {} ---", target_id);

    if let Some(t) = manager.get_template(target_id).await {
        println!("✅ Template trouvé !");
        // ACCÈS VIA .definition
        println!("   Identité : {} - {}", t.definition.identity.brand, t.definition.identity.model);
        println!("   Type d'entité : {}", t.definition.entity_type);

        let states_count = t.definition.points.states.len();
        let actions_count = t.definition.points.actions.len();
        println!("   Points définis : {} états, {} actions", states_count, actions_count);

        // Les mappings et scripts sont à la racine de l'objet EntityTemplate
        println!("   Scripts chargés : {}", t.scripts.len());

        if !t.mappings.is_empty() {
            let protocols: Vec<_> = t.mappings.keys().collect();
            println!("✅ Mappings détectés : {:?}", protocols);
        } else {
            println!("⚠️ Attention : Aucun mapping trouvé.");
        }
    } else {
        println!("❌ Erreur : Le template '{}' n'a pas été chargé.", target_id);
    }

    println!("\n--- Fin de validation ---");
}
