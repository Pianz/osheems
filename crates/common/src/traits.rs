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
            // --- Electric (Mesures instantanées) ---
            TraitDefinition::new("voltage", TraitCategory::Electric, "V", "RMS Voltage"),
            TraitDefinition::new("current", TraitCategory::Electric, "A", "RMS Current"),
            TraitDefinition::new("power_active", TraitCategory::Electric, "W", "Active power (real)"),
            TraitDefinition::new("power_apparent", TraitCategory::Electric, "VA", "Apparent power"),
            TraitDefinition::new("power_reactive", TraitCategory::Electric, "VAR", "Reactive power"),
            TraitDefinition::new("power_factor", TraitCategory::Electric, "", "Power factor"),

            // --- Electric (Compteurs d'énergie cumulés) ---
            TraitDefinition::new("energy_active", TraitCategory::Electric, "Wh", "Cumulative active energy (import)"),
            TraitDefinition::new("energy_active_returned", TraitCategory::Electric, "Wh", "Cumulative active energy returned (export)"),
            TraitDefinition::new("energy_active_import_kwh", TraitCategory::Electric, "kWh", "Cumulative energy imported (standard unit)"),

            // --- Thermal ---
            TraitDefinition::new("temperature", TraitCategory::Thermal, "°C", "Ambient or sensor temperature"),
            TraitDefinition::new("thermostat_setpoint", TraitCategory::Thermal, "°C", "Temperature Setpoint"),

            // --- Control ---
            TraitDefinition::new("switch_state", TraitCategory::Control, "bool", "State of a relay/switch"),
            TraitDefinition::new("switch_cmd", TraitCategory::Control, "bool", "Control command for a switch"),

            // --- Storage ---
            TraitDefinition::new("battery_soc", TraitCategory::Storage, "%", "State of Charge"),
            TraitDefinition::new("battery_voltage", TraitCategory::Storage, "V", "Battery bank voltage"),

            // --- System ---
            TraitDefinition::new("rssi", TraitCategory::System, "dBm", "Signal strength"),
            TraitDefinition::new("uptime", TraitCategory::System, "s", "System uptime"),
            TraitDefinition::new("status", TraitCategory::System, "string", "Device status message"),
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
    pub fn new(id: &str, category: TraitCategory, unit: &str, desc: &str) -> Self {
        Self {
            id: id.to_string(),
            category,
            default_unit: unit.to_string(),
                description: desc.to_string(),
        }
    }
}
