//! Companion Proxy service implementation

use crate::{IdeviceError, IdeviceService, ServiceProviderType};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const COMPANION_PROXY_SERVICE_NAME: &str = "com.apple.companion_proxy";

/// Companion Proxy client for device pairing
pub struct CompanionProxyClient {
    socket: tokio::net::TcpStream,
}

impl CompanionProxyClient {
    /// Connect to the Companion Proxy service
    pub async fn connect(provider: &dyn ServiceProviderType) -> Result<Self, IdeviceError> {
        let service = provider.start_service(COMPANION_PROXY_SERVICE_NAME).await?;
        Ok(Self {
            socket: service.socket,
        })
    }
}