pub mod manager;

pub use manager::GatewayManager;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Représentation en mémoire d'une Gateway active.
/// Contrairement à l'entité brute en DB, celle-ci peut contenir
/// des états de connexion en temps réel.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GatewayInstance {
    pub id: String,
    pub label: String,
    pub interface_id: String,
    pub configuration: Value,
    pub is_connected: bool,
}

/// Énumération des types de drivers supportés par les gateways.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GatewayDriver {
    Mqtt,
    ModbusIp,
    ModbusRtu,
    Zigbee,
    Virtual,
}
