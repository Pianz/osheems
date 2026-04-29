use crate::managers::interface::AsyncInterface;
use async_trait::async_trait;
use tokio_serial::{SerialPortBuilderExt, SerialStream, SerialPort};
use std::io;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;
use serde_json::Value;

pub struct UartInterface {
    path: String,
    baud_rate: u32,
    /// We protect the stream with a Tokio Mutex to allow Send + Sync access
    stream: Arc<TokioMutex<Option<SerialStream>>>,
}

impl UartInterface {
    /// Instantiates the interface from a JSON block (resolved by InterfaceManager)
    pub fn from_config(config: &Value) -> Option<Self> {
        // The manager ensures "path" is the current valid /dev/ttyUSBx
        let path = config.get("path")?.as_str()?.to_string();
        let baud_rate = config.get("baud_rate")?.as_u64()? as u32;

        Some(Self {
            path,
            baud_rate,
            stream: Arc::new(TokioMutex::new(None)),
        })
    }
}

#[async_trait]
impl AsyncInterface for UartInterface {
    async fn open(&mut self) -> io::Result<()> {
        let mut lock = self.stream.lock().await;

        // Port configuration via tokio-serial
        let port = tokio_serial::new(&self.path, self.baud_rate)
        .data_bits(tokio_serial::DataBits::Eight)
        .parity(tokio_serial::Parity::None)
        .stop_bits(tokio_serial::StopBits::One)
        .open_native_async()?;

        *lock = Some(port);
        println!("[UART] Port {} opened at {} baud.", self.path, self.baud_rate);
        Ok(())
    }

    async fn write(&self, data: &[u8]) -> io::Result<usize> {
        let mut lock = self.stream.lock().await;
        if let Some(port) = lock.as_mut() {
            use tokio::io::AsyncWriteExt;
            port.write_all(data).await?;
            port.flush().await?;
            Ok(data.len())
        } else {
            Err(io::Error::new(io::ErrorKind::NotConnected, "UART port not open"))
        }
    }

    async fn read(&self, buffer: &mut [u8]) -> io::Result<usize> {
        let mut lock = self.stream.lock().await;
        if let Some(port) = lock.as_mut() {
            use tokio::io::AsyncReadExt;
            port.read(buffer).await
        } else {
            Err(io::Error::new(io::ErrorKind::NotConnected, "UART port not open"))
        }
    }

    async fn close(&mut self) -> io::Result<()> {
        let mut lock = self.stream.lock().await;
        *lock = None;
        Ok(())
    }

    fn is_alive(&self) -> bool {
        // Simple check of stream presence
        true
    }
}
