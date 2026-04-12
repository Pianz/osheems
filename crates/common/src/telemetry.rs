use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    pub timestamp: i64,
    pub entity_id: i64,
    pub key: String,
    pub value: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HistoryQuery {
    pub entity_id: i64,
    pub key: String,
    pub start_ts: i64,
    pub end_ts: i64,
}
