//! AMFI (Apple Mobile File Integrity) service implementation

use crate::{IdeviceError, IdeviceService, ServiceProviderType};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const AMFI_SERVICE_NAME: &str = "com.apple.amfi";

/// AMFI client for interacting with Apple Mobile File Integrity service
pub struct AmfiClient {
    socket: tokio::net::TcpStream,
}

impl AmfiClient {
    /// Connect to the AMFI service
    pub async fn connect(provider: &dyn ServiceProviderType) -> Result<Self, IdeviceError> {
        let service = provider.start_service(AMFI_SERVICE_NAME).await?;
        Ok(Self {
            socket: service.socket,
        })
    }

    /// Get developer mode status
    pub async fn get_developer_mode_status(&mut self) -> Result<bool, IdeviceError> {
        let mut command = [0u8; 4];
        self.socket.write_all(b"Q").await?;
        self.socket.read_exact(&mut command).await?;
        Ok(command[0] != 0)
    }
}