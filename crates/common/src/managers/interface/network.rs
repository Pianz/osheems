use crate::managers::interface::AsyncInterface;
use async_trait::async_trait;
use tokio::net::{TcpStream, UdpSocket};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::io;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;
use serde_json::Value;

pub enum NetworkProtocol {
    Tcp(TcpStream),
    Udp(UdpSocket),
}

pub struct NetworkInterface {
    address: String,
    // On stocke le socket dans l'énumération
    stream: Arc<TokioMutex<Option<NetworkProtocol>>>,
    is_udp: bool,
}

impl NetworkInterface {
    pub fn from_config(config: &Value) -> Option<Self> {
        let host = config.get("host")?.as_str()?;
        let port = config.get("port")?.as_u64()?;
        let proto = config.get("protocol").and_then(|v| v.as_str()).unwrap_or("tcp");

        Some(Self {
            address: format!("{}:{}", host, port),
             stream: Arc::new(TokioMutex::new(None)),
             is_udp: proto.to_lowercase() == "udp",
        })
    }
}

#[async_trait]
impl AsyncInterface for NetworkInterface {
    async fn open(&mut self) -> io::Result<()> {
        let mut lock = self.stream.lock().await;

        if self.is_udp {
            // En UDP, on "bind" sur une adresse locale (0.0.0.0:0 pour laisser l'OS choisir le port de sortie)
            // puis on "connect" pour fixer la destination par défaut des appels read/write.
            let socket = UdpSocket::bind("0.0.0.0:0").await?;
            socket.connect(&self.address).await?;
            *lock = Some(NetworkProtocol::Udp(socket));
            println!("[UDP] Socket lié et prêt pour {}", self.address);
        } else {
            let stream = TcpStream::connect(&self.address).await?;
            stream.set_nodelay(true)?;
            *lock = Some(NetworkProtocol::Tcp(stream));
            println!("[TCP] Connecté à {}", self.address);
        }
        Ok(())
    }

    async fn write(&self, data: &[u8]) -> io::Result<usize> {
        let mut lock = self.stream.lock().await;
        match lock.as_mut() {
            Some(NetworkProtocol::Tcp(s)) => {
                s.write_all(data).await?;
                s.flush().await?;
                Ok(data.len())
            }
            Some(NetworkProtocol::Udp(s)) => s.send(data).await,
            None => Err(io::Error::new(io::ErrorKind::NotConnected, "Interface réseau non ouverte")),
        }
    }

    async fn read(&self, buffer: &mut [u8]) -> io::Result<usize> {
        let mut lock = self.stream.lock().await;
        match lock.as_mut() {
            Some(NetworkProtocol::Tcp(s)) => s.read(buffer).await,
            Some(NetworkProtocol::Udp(s)) => s.recv(buffer).await,
            None => Err(io::Error::new(io::ErrorKind::NotConnected, "Interface réseau non ouverte")),
        }
    }

    async fn close(&mut self) -> io::Result<()> {
        let mut lock = self.stream.lock().await;
        if let Some(NetworkProtocol::Tcp(mut s)) = lock.take() {
            s.shutdown().await?;
        }
        // En UDP, le "close" se fait juste en droppant le socket (ce que lock.take() fait).
        Ok(())
    }

    fn is_alive(&self) -> bool {
        self.stream.try_lock().map(|l| l.is_some()).unwrap_or(true)
    }
}
