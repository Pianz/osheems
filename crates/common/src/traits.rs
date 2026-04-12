use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TraitCategory {
    Electric,
    Thermal,
    Fluid,
    Environmental,
    Control,
    Storage,
    Solar,
    System,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct TraitDefinition {
    pub id: String,
    pub category: TraitCategory,
    pub default_unit: String,
    pub description: String,
}

pub struct TraitRegistry {
    traits: HashMap<String, TraitDefinition>,
}

impl TraitRegistry {
    /// Initialise le registre avec les traits officiels d'OSHEEMS
    pub fn build() -> Self {
        let mut traits = HashMap::new();

        let official_list = vec![
            // --- Electric ---
            TraitDefinition::new("power_active", TraitCategory::Electric, "W", "Active power (real)"),
            TraitDefinition::new("energy_active_import", TraitCategory::Electric, "kWh", "Cumulative energy imported"),
            TraitDefinition::new("voltage", TraitCategory::Electric, "V", "RMS Voltage"),
            TraitDefinition::new("current", TraitCategory::Electric, "A", "RMS Current"),
            TraitDefinition::new("power_factor", TraitCategory::Electric, "", "Power factor"),

            // --- Control ---
            TraitDefinition::new("switch_state", TraitCategory::Control, "bool", "State of a relay/switch"),
            TraitDefinition::new("switch_cmd", TraitCategory::Control, "bool", "Control command for a switch"),

            // --- Storage ---
            TraitDefinition::new("battery_soc", TraitCategory::Storage, "%", "State of Charge"),

            // --- System ---
            TraitDefinition::new("rssi", TraitCategory::System, "dBm", "Signal strength"),
            TraitDefinition::new("uptime", TraitCategory::System, "s", "System uptime"),
        ];

        for t in official_list {
            traits.insert(t.id.clone(), t);
        }

        Self { traits }
    }

    /// Vérifie si un trait existe dans le registre
    pub fn exists(&self, trait_id: &str) -> bool {
        self.traits.contains_key(trait_id)
    }

    /// Récupère la définition d'un trait
    pub fn get(&self, trait_id: &str) -> Option<&TraitDefinition> {
        self.traits.get(trait_id)
    }
}

impl TraitDefinition {
    fn new(id: &str, category: TraitCategory, unit: &str, desc: &str) -> Self {
        Self {
            id: id.to_string(),
            category,
            default_unit: unit.to_string(),
                description: desc.to_string(),
        }
    }
}
