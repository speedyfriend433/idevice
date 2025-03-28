//! Diagnostics service implementation
//! 
//! This module provides functionality to retrieve diagnostic information from iOS devices.

use crate::{IdeviceError, IdeviceService, ServiceProviderType};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::collections::HashMap;

const DIAGNOSTICS_SERVICE_NAME: &str = "com.apple.mobile.diagnostics_relay";

/// Diagnostics action types
#[derive(Debug, Clone, Copy)]
pub enum DiagnosticsAction {
    /// Request all diagnostics
    All,
    /// Request diagnostics for a specific domain
    Domain(DiagnosticsDomain),
    /// Request I/O Registry entry
    IORegistry,
    /// Request network interfaces
    NetworkInterfaces,
    /// Restart the device
    Restart,
    /// Shutdown the device
    Shutdown,
    /// Sleep the device
    Sleep,
}

/// Diagnostics domains
#[derive(Debug, Clone, Copy)]
pub enum DiagnosticsDomain {
    /// WiFi diagnostics
    WiFi,
    /// GasGauge (battery) diagnostics
    GasGauge,
    /// NAND (storage) diagnostics
    NAND,
    /// HDMI diagnostics
    HDMI,
}

impl DiagnosticsDomain {
    fn as_str(&self) -> &'static str {
        match self {
            DiagnosticsDomain::WiFi => "com.apple.mobile.wifi",
            DiagnosticsDomain::GasGauge => "com.apple.mobile.gas_gauge",
            DiagnosticsDomain::NAND => "com.apple.mobile.NAND",
            DiagnosticsDomain::HDMI => "com.apple.mobile.HDMI",
        }
    }
}

/// Diagnostics client for retrieving diagnostic information from iOS devices
pub struct DiagnosticsClient {
    socket: tokio::net::TcpStream,
}

impl DiagnosticsClient {
    /// Connect to the Diagnostics service
    pub async fn connect(provider: &dyn ServiceProviderType) -> Result<Self, IdeviceError> {
        let service = provider.start_service(DIAGNOSTICS_SERVICE_NAME).await?;
        
        Ok(Self {
            socket: service.socket,
        })
    }

    /// Request diagnostics information
    pub async fn request_diagnostics(&mut self, action: DiagnosticsAction) -> Result<plist::Value, IdeviceError> {
        let mut dict = plist::Dictionary::new();
        
        match action {
            DiagnosticsAction::All => {
                dict.insert("Request".into(), "All".into());
            }
            DiagnosticsAction::Domain(domain) => {
                dict.insert("Request".into(), "Diagnostics".into());
                dict.insert("Domain".into(), domain.as_str().into());
            }
            DiagnosticsAction::IORegistry => {
                dict.insert("Request".into(), "IORegistry".into());
            }
            DiagnosticsAction::NetworkInterfaces => {
                dict.insert("Request".into(), "NetworkInterfaces".into());
            }
            DiagnosticsAction::Restart => {
                dict.insert("Request".into(), "Restart".into());
            }
            DiagnosticsAction::Shutdown => {
                dict.insert("Request".into(), "Shutdown".into());
            }
            DiagnosticsAction::Sleep => {
                dict.insert("Request".into(), "Sleep".into());
            }
        }
        
        self.send_plist(&dict).await?;
        let response = self.read_plist().await?;
        
        // Check for errors
        if let Some(status) = response.get("Status") {
            if let Some(status) = status.as_string() {
                if status != "Success" {
                    let error_msg = response.get("Error")
                        .and_then(|e| e.as_string())
                        .unwrap_or("Unknown error");
                    return Err(IdeviceError::DiagnosticsError(error_msg.to_string()));
                }
            }
        }
        
        // Return the diagnostics data
        if let Some(diagnostics) = response.get("Diagnostics") {
            return Ok(diagnostics.clone());
        }
        
        // If no diagnostics data, return the whole response
        Ok(response)
    }

    /// Get device information
    pub async fn get_device_info(&mut self) -> Result<HashMap<String, String>, IdeviceError> {
        let response = self.request_diagnostics(DiagnosticsAction::All).await?;
        
        let mut info = HashMap::new();
        if let Some(dict) = response.as_dictionary() {
            for (key, value) in dict {
                if let Some(value_str) = value.as_string() {
                    info.insert(key.clone(), value_str.to_string());
                } else {
                    info.insert(key.clone(), format!("{:?}", value));
                }
            }
        }
        
        Ok(info)
    }

    /// Get I/O Registry information
    pub async fn get_io_registry(&mut self) -> Result<plist::Value, IdeviceError> {
        self.request_diagnostics(DiagnosticsAction::IORegistry).await
    }

    /// Get network interfaces information
    pub async fn get_network_interfaces(&mut self) -> Result<plist::Value, IdeviceError> {
        self.request_diagnostics(DiagnosticsAction::NetworkInterfaces).await
    }

    /// Restart the device
    pub async fn restart(&mut self) -> Result<(), IdeviceError> {
        self.request_diagnostics(DiagnosticsAction::Restart).await?;
        Ok(())
    }

    /// Shutdown the device
    pub async fn shutdown(&mut self) -> Result<(), IdeviceError> {
        self.request_diagnostics(DiagnosticsAction::Shutdown).await?;
        Ok(())
    }

    /// Sleep the device
    pub async fn sleep(&mut self) -> Result<(), IdeviceError> {
        self.request_diagnostics(DiagnosticsAction::Sleep).await?;
        Ok(())
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