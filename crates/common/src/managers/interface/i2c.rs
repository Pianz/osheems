use crate::managers::interface::AsyncInterface;
use async_trait::async_trait;
use std::io;
use std::sync::{Arc, Mutex};
use serde_json::Value;
use i2cdev::core::I2CDevice;
use i2cdev::linux::LinuxI2CDevice;

pub struct I2cInterface {
    path: String,
    address: u16,
    // L'I2C sous Linux n'est pas nativement asynchrone,
    // on utilise un Mutex synchrone pour protéger l'accès au descripteur de fichier.
    device: Arc<Mutex<Option<LinuxI2CDevice>>>,
}

impl I2cInterface {
    /// Config attendue : {"protocol": "i2c", "path": "/dev/i2c-1", "address": 118}
    pub fn from_config(config: &Value) -> Option<Self> {
        let path = config.get("path")?.as_str()?.to_string();
        let address = config.get("address")?.as_u64()? as u16;

        Some(Self {
            path,
            address,
            device: Arc::new(Mutex::new(None)),
        })
    }
}

#[async_trait]
impl AsyncInterface for I2cInterface {
    async fn open(&mut self) -> io::Result<()> {
        let path = self.path.clone();
        let address = self.address;
        let device_arc = self.device.clone();

        // On effectue l'ouverture dans un thread bloquant pour ne pas figer l'exécuteur Tokio
        tokio::task::spawn_blocking(move || {
            let mut lock = device_arc.lock().unwrap();
            let dev = LinuxI2CDevice::new(&path, address)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            *lock = Some(dev);
            Ok(())
        }).await.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
    }

    async fn write(&self, data: &[u8]) -> io::Result<usize> {
        let device_arc = self.device.clone();
        let buf = data.to_vec();

        tokio::task::spawn_blocking(move || {
            let mut lock = device_arc.lock().unwrap();
            if let Some(dev) = lock.as_mut() {
                dev.write(&buf)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                Ok(buf.len())
            } else {
                Err(io::Error::new(io::ErrorKind::NotConnected, "I2C non ouvert"))
            }
        }).await.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
    }

    async fn read(&self, buffer: &mut [u8]) -> io::Result<usize> {
        let device_arc = self.device.clone();
        // On doit cloner la taille pour le thread
        let len = buffer.len();

        let result = tokio::task::spawn_blocking(move || {
            let mut lock = device_arc.lock().unwrap();
            if let Some(dev) = lock.as_mut() {
                let mut tmp_buf = vec![0u8; len];
                dev.read(&mut tmp_buf)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                Ok(tmp_buf)
            } else {
                Err(io::Error::new(io::ErrorKind::NotConnected, "I2C non ouvert"))
            }
        }).await.map_err(|e| io::Error::new(io::ErrorKind::Other, e))??;

        // On recopie les données lues dans le buffer d'origine
        buffer.copy_from_slice(&result);
        Ok(result.len())
    }

    async fn close(&mut self) -> io::Result<()> {
        let mut lock = self.device.lock().unwrap();
        *lock = None; // Le drop de LinuxI2CDevice ferme le fichier
        Ok(())
    }

    fn is_alive(&self) -> bool {
        self.device.lock().map(|l| l.is_some()).unwrap_or(false)
    }
}
