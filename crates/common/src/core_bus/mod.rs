pub mod manager;
pub mod types;
pub mod runner;
pub mod dispatcher;

// Re-export pour faciliter l'usage : core_bus::CoreBusManager
pub use manager::CoreBusManager;
