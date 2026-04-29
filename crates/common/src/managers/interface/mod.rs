use async_trait::async_trait;
use std::io;

#[cfg(feature = "native")]
pub mod manager;
pub mod uart;
pub mod network;
pub mod i2c;

// Note : Les modules i2c et spi seront ajoutés ici
// au fur et à mesure de leur implémentation.

/// Trait fondamental pour toutes les interfaces de communication d'OSHEEMS.
/// L'utilisation de 'async_trait' permet une gestion non-bloquante via Tokio.
#[async_trait]
pub trait AsyncInterface: Send + Sync {
    /// Initialise et ouvre l'accès physique à l'interface.
    async fn open(&mut self) -> io::Result<()>;

    /// Ferme proprement l'interface et libère les ressources.
    async fn close(&mut self) -> io::Result<()>;

    /// Envoie des octets bruts sur l'interface.
    async fn write(&self, data: &[u8]) -> io::Result<usize>;

    /// Lit des octets depuis l'interface et les place dans le buffer.
    async fn read(&self, buffer: &mut [u8]) -> io::Result<usize>;

    /// Vérifie si l'interface est toujours fonctionnelle (ex: socket toujours ouverte).
    fn is_alive(&self) -> bool;
}

#[cfg(feature = "native")]
pub use manager::InterfaceManager;
pub use uart::UartInterface;
pub use network::NetworkInterface;
pub use i2c::I2cInterface;
