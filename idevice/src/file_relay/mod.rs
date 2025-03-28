//! File Relay service implementation
//! 
//! This module provides functionality to retrieve various files and logs from iOS devices.

use crate::{IdeviceError, IdeviceService, ServiceProviderType};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::collections::HashSet;

const FILE_RELAY_SERVICE_NAME: &str = "com.apple.mobile.file_relay";

/// File Relay sources that can be requested
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileRelaySource {
    AppleSupport,
    Network,
    VPN,
    Wifi,
    UserDatabases,
    CrashReporter,
    Tmp,
    SystemConfiguration,
    Keyboard,
    Logs,
    Lockdown,
    MobileInstallation,
    CrashReporterClearable,
    Diagnostics,
    All,
}

impl FileRelaySource {
    fn as_str(&self) -> &'static str {
        match self {
            FileRelaySource::AppleSupport => "AppleSupport",
            FileRelaySource::Network => "Network",
            FileRelaySource::VPN => "VPN",
            FileRelaySource::Wifi => "Wifi",
            FileRelaySource::UserDatabases => "UserDatabases",
            FileRelaySource::CrashReporter => "CrashReporter",
            FileRelaySource::Tmp => "Tmp",
            FileRelaySource::SystemConfiguration => "SystemConfiguration",
            FileRelaySource::Keyboard => "Keyboard",
            FileRelaySource::Logs => "Logs",
            FileRelaySource::Lockdown => "Lockdown",
            FileRelaySource::MobileInstallation => "MobileInstallation",
            FileRelaySource::CrashReporterClearable => "CrashReporter-Clearable",
            FileRelaySource::Diagnostics => "Diagnostics",
            FileRelaySource::All => "All",
        }
    }
}

/// File Relay client for retrieving files and logs from iOS devices
pub struct FileRelayClient {
    socket: tokio::net::TcpStream,
}

impl FileRelayClient {
    /// Connect to the File Relay service
    pub async fn connect(provider: &dyn ServiceProviderType) -> Result<Self, IdeviceError> {
        let service = provider.start_service(FILE_RELAY_SERVICE_NAME).await?;
        
        Ok(Self {
            socket: service.socket,
        })
    }

    /// Request files from the device
    pub async fn request_files(&mut self, sources: &[FileRelaySource]) -> Result<Vec<u8>, IdeviceError> {
        // Create a set of unique sources
        let sources_set: HashSet<_> = sources.iter().collect();
        
        // Create the request dictionary
        let mut dict = plist::Dictionary::new();
        let sources_array: Vec<plist::Value> = sources_set.iter()
            .map(|s| s.as_str().into())
            .collect();
        dict.insert("Sources".into(), sources_array.into());
        
        // Send the request
        self.send_plist(&dict).await?;
        
        // Read the response
        let response = self.read_plist().await?;
        
        // Check for errors
        if let Some(error) = response.get("Error") {
            let error_str = error.as_string().unwrap_or("Unknown error");
            return Err(IdeviceError::FileRelayError(error_str.to_string()));
        }
        
        // Check if we have a status
        if let Some(status) = response.get("Status") {
            let status_str = status.as_string().unwrap_or("");
            if status_str != "Complete" {
                return Err(IdeviceError::FileRelayError(format!("Unexpected status: {}", status_str)));
            }
        }
        
        // Read the file data
        let mut length_buf = [0u8; 4];
        self.socket.read_exact(&mut length_buf).await?;
        let length = u32::from_be_bytes(length_buf) as usize;
        
        let mut data = vec![0u8; length];
        self.socket.read_exact(&mut data).await?;
        
        Ok(data)
    }

    // Helper methods
    async fn send_plist(&mut self, dict: &plist::Dictionary) -> Result<(), IdeviceError> {
        let xml = plist::to_format_xml(dict)?;
        let xml_bytes = xml.into_bytes();
        
        // Send the length as a 32-bit big-endian integer
        let len = (xml_bytes.len() as u32).to_be_bytes();
        self.socket.write_all(&len).await?;
        
        // Send the XML data
        self.socket.write_all(&xml_bytes).await?;
        
        Ok(())
    }

    async fn read_plist(&mut self) -> Result<plist::Dictionary, IdeviceError> {
        // Read the length as a 32-bit big-endian integer
        let mut len_buf = [0u8; 4];
        self.socket.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        
        // Read the XML data
        let mut data = vec![0u8; len];
        self.socket.read_exact(&mut data).await?;
        
        // Parse the XML data
        let dict = plist::from_bytes(&data)?;
        
        Ok(dict)
    }
}