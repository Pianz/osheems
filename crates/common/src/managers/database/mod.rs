#[cfg(feature = "native")]
pub mod manager;

#[cfg(feature = "native")]
pub use manager::DatabaseManager;
