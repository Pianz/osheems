use serde::{Deserialize, Serialize};
use std::fmt;
use std::convert::TryFrom;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CorePayload {
    pub value: serde_json::Value,
    pub unit: String,
    pub label: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MqttSuffix {
    Get,
    Set,
    Evt,
    Conf,
    Logs,
}

impl fmt::Display for MqttSuffix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            MqttSuffix::Get => "get",
            MqttSuffix::Set => "set",
            MqttSuffix::Evt => "evt",
            MqttSuffix::Conf => "conf",
            MqttSuffix::Logs => "logs",
        };
        write!(f, "{}", s)
    }
}

// Ajout indispensable pour la compilation du Manager
impl TryFrom<&str> for MqttSuffix {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "get" => Ok(MqttSuffix::Get),
            "set" => Ok(MqttSuffix::Set),
            "evt" => Ok(MqttSuffix::Evt),
            "conf" => Ok(MqttSuffix::Conf),
            "logs" | "log" => Ok(MqttSuffix::Logs),
            _ => Err(format!("Unknown suffix: {}", value)),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OsheemsRole {
    Core,
    Edge,
}

impl fmt::Display for OsheemsRole {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OsheemsRole::Core => write!(f, "core"),
            OsheemsRole::Edge => write!(f, "edge"),
        }
    }
}
