pub mod main_db;
pub mod telemetry_db;
pub mod bootstrap;

// On importe directement depuis les modules sources
pub use crate::entities::Entity;
pub use crate::relations::RelationRecord;

// On garde l'export des structures de bases de données
pub use main_db::MainDatabase;
pub use telemetry_db::{TelemetryDatabase, TelemetryRecord, TelemetryStats};
