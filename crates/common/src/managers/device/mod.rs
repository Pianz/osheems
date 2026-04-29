pub mod manager;
pub use manager::DeviceManager;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceInstance {
    pub id: String,
    pub template_id: String,
    pub gateway_id: String,
    pub is_online: bool,
}
