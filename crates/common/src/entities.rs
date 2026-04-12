use serde::{Serialize, Deserialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
// Import des traits Rusqlite
use rusqlite::types::{FromSql, FromSqlResult, FromSqlError, ToSql, ToSqlOutput, ValueRef};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    // --- System & Users ---
    System,      // OSHEEMS Core
    User,        // User identity and access rights

    // --- Organization & Topology ---
    Asset,       // Actif: Building, Fleet, Site, Industrial plant
    Area,        // Zone: Floor, Room, Thermal zone, Section

    // --- Connectivity & Hardware ---
    Interface,   // Physical port/Link: /dev/ttyUSB0, eth0, wlan0, CAN bus
    Gateway,     // Connectivity hub: Zigbee Bridge, Modbus-IP Gateway
    Device,      // Physical hardware: Shelly, Heat pump, Inverter, Meter

    // --- Logic & Control ---
    Controller,  // Decision logic: Surplus optimizer, Load shedder
    Regulator,   // Control loop: PID, Thermostat, Dimmer
    Virtual,     // Calculation: Virtual entity (Power sums, Ratios)

    // --- Software & Integration ---
    Integration, // External: Weather API, Energy Prices, Cloud Exports
    Service,     // Internal Service: Dashboard UI, API Server, MQTT Broker
}

impl fmt::Display for EntityType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            EntityType::System => "system",
            EntityType::User => "user",
            EntityType::Asset => "asset",
            EntityType::Area => "area",
            EntityType::Interface => "interface",
            EntityType::Gateway => "gateway",
            EntityType::Device => "device",
            EntityType::Controller => "controller",
            EntityType::Regulator => "regulator",
            EntityType::Virtual => "virtual",
            EntityType::Integration => "integration",
            EntityType::Service => "service",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for EntityType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "system"      => Ok(EntityType::System),
            "user"        => Ok(EntityType::User),
            "asset"       => Ok(EntityType::Asset),
            "area"        => Ok(EntityType::Area),
            "interface"   => Ok(EntityType::Interface),
            "gateway"     => Ok(EntityType::Gateway),
            "device"      => Ok(EntityType::Device),
            "controller"  => Ok(EntityType::Controller),
            "regulator"   => Ok(EntityType::Regulator),
            "virtual"     => Ok(EntityType::Virtual),
            "integration" => Ok(EntityType::Integration),
            "service"     => Ok(EntityType::Service),
            _             => Err(format!("Unknown EntityType: {}", s)),
        }
    }
}

// --- RUSQLITE TRAITS ---

impl ToSql for EntityType {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.to_string()))
    }
}

impl FromSql for EntityType {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let s = value.as_str()?;
        s.parse::<EntityType>().map_err(|_| FromSqlError::InvalidType)
    }
}

// --- DATA STRUCTURES ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// Unique identifier (Slug/Path). Replaces both old 'id' (i64) and 'name'.
    /// Example: "edge1/kitchen/shelly_pro3em"
    pub id: String,
    pub entity_type: EntityType,
    pub template_id: Option<String>,

    /// Human-friendly display name. Example: "Main Meter"
    pub label: Option<String>,

    pub description: Option<String>,
    pub configuration: Value,
    pub is_enabled: bool,
    pub is_system: bool,
}

impl Default for Entity {
    fn default() -> Self {
        Self {
            id: "new_entity".to_string(),
            entity_type: EntityType::Device,
            template_id: None,
            label: None,
            description: None,
            configuration: Value::Object(Default::default()),
            is_enabled: true,
            is_system: false,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ActiveEntity {
    pub base: Entity,
    pub points_state: HashMap<String, f64>,
    pub status_state: HashMap<String, bool>,
}
