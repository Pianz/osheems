use osheems_common::templates::TemplateManager;
use std::path::PathBuf;
use std::fs;

fn main() {
    // 1. Initialise les logs
    env_logger::init();

    // 2. Crée une arborescence de test
    let test_path = PathBuf::from("./test_templates");
    let shelly_dir = test_path.join("devices/shelly/pro3em");
    let mapping_dir = shelly_dir.join("mapping");

    if test_path.exists() {
        fs::remove_dir_all(&test_path).unwrap();
    }
    fs::create_dir_all(&mapping_dir).unwrap();

    // 3. Le JSON du Template
    let template_json = r#"{
    "template_id": "shelly_pro_3em",
    "version": "1.0.0",
    "entity_type": "device",
    "identity": {
    "brand": "SHELLY",
    "model": "Pro 3EM",
    "traits": ["three_phase_meter"],
    "protocols": ["modbus_tcp", "mqtt"]
},
"config": {
"unit_id": { "type": "number", "default": 1, "description": "Modbus Unit ID" }
},
"points": {
"states": [
{ "id": "power_a", "trait": "power_active", "unit": "W" }
],
"actions": [
{ "id": "relay_switch", "trait": "switch_cmd", "type": "bool" }
]
}
}"#;

// 4. Les Mappings
let modbus_mapping_json = r#"{
"transport_config": { "byte_order": "ABDC" },
"points": {
"power_a": { "address": 1007, "quantity": 2, "data_type": "float32" }
}
}"#;

let mqtt_mapping_json = r#"{
"transport_config": { },
"points": {
"power_a": { "point_topic": "status/em:0", "json_path": "a_act_power" }
}
}"#;

// --- LA CORRECTION EST ICI ---
fs::write(shelly_dir.join("template.json"), template_json).unwrap();
fs::write(mapping_dir.join("modbus.json"), modbus_mapping_json).unwrap();
fs::write(mapping_dir.join("mqtt.json"), mqtt_mapping_json).unwrap(); // <--- Cette ligne manquait !
// -----------------------------

// 5. Test du TemplateManager
println!("\n--- Initialisation du TemplateManager ---");
let manager = TemplateManager::new(test_path);

// 6. Vérification
if let Some(t) = manager.get_template("shelly_pro_3em") {
    println!("✅ Succès ! Template '{}' chargé.", t.template_id);
    println!("Brand: {}", t.identity.brand);

    if !t.mappings.is_empty() {
        // Ici tu devrais voir ["modbus", "mqtt"]
        println!("Detected mappings: {:?}", t.mappings.keys());

        if t.mappings.contains_key("modbus") && t.mappings.contains_key("mqtt") {
            println!("  -> All mappings (Modbus & MQTT) correctly parsed!");
        }
    } else {
        println!("⚠️ Aucun mapping trouvé.");
    }
} else {
    println!("❌ Échec : Le template n'a pas été trouvé.");
}
}
