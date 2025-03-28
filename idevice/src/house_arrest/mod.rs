//! House Arrest service implementation
//! 
//! This module provides functionality to access app containers on iOS devices.

use crate::{IdeviceError, IdeviceService, ServiceProviderType};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::collections::HashMap;

const HOUSE_ARREST_SERVICE_NAME: &str = "com.apple.mobile.house_arrest";

/// House Arrest client for accessing app containers
pub struct HouseArrestClient {
    socket: tokio::net::TcpStream,
}

impl HouseArrestClient {
    /// Connect to the House Arrest service
    pub async fn connect(provider: &dyn ServiceProviderType) -> Result<Self, IdeviceError> {
        let service = provider.start_service(HOUSE_ARREST_SERVICE_NAME).await?;
        
        Ok(Self {
            socket: service.socket,
        })
    }

    /// Get an AFC client for accessing the app's Documents directory
    pub async fn documents(&mut self, bundle_id: &str) -> Result<crate::afc::AfcClient, IdeviceError> {
        self.send_command("VendDocuments", bundle_id).await?;
        self.check_result().await?;
        
        // The service has now switched to AFC protocol
        Ok(crate::afc::AfcClient {
            socket: std::mem::replace(&mut self.socket, tokio::net::TcpStream::connect("0.0.0.0:0").await.unwrap()),
            packet_num: 0,
        })
    }

    /// Get an AFC client for accessing the app's Container directory
    pub async fn container(&mut self, bundle_id: &str) -> Result<crate::afc::AfcClient, IdeviceError> {
        self.send_command("VendContainer", bundle_id).await?;
        self.check_result().await?;
        
        // The service has now switched to AFC protocol
        Ok(crate::afc::AfcClient {
            socket: std::mem::replace(&mut self.socket, tokio::net::TcpStream::connect("0.0.0.0:0").await.unwrap()),
            packet_num: 0,
        })
    }

    /// List installed applications
    pub async fn list_installed_applications(&mut self) -> Result<Vec<String>, IdeviceError> {
        self.send_command("ListApplications", "").await?;
        let result = self.read_plist().await?;
        
        if let Some(error) = result.get("Error") {
            let error_str = error.as_string().unwrap_or("Unknown error");
            return Err(IdeviceError::HouseArrestError(error_str.to_string()));
        }
        
        if let Some(apps) = result.get("ApplicationList") {
            if let Some(apps_dict) = apps.as_dictionary() {
                return Ok(apps_dict.keys().map(|k| k.to_string()).collect());
            }
        }
        
        Err(IdeviceError::HouseArrestError("Failed to get application list".to_string()))
    }

    /// Get application information
    pub async fn get_application_info(&mut self, bundle_id: &str) -> Result<HashMap<String, plist::Value>, IdeviceError> {
        self.send_command("Lookup", bundle_id).await?;
        let result = self.read_plist().await?;
        
        if let Some(error) = result.get("Error") {
            let error_str = error.as_string().unwrap_or("Unknown error");
            return Err(IdeviceError::HouseArrestError(error_str.to_string()));
        }
        
        if let Some(info) = result.get("LookupResult") {
            if let Some(info_dict) = info.as_dictionary() {
                return Ok(info_dict.clone());
            }
        }
        
        Err(IdeviceError::HouseArrestError("Failed to get application info".to_string()))
    }

    // Helper methods
    async fn send_command(&mut self, command: &str, bundle_id: &str) -> Result<(), IdeviceError> {
        let mut dict = plist::Dictionary::new();
        dict.insert("Command".into(), command.into());
        dict.insert("Identifier".into(), bundle_id.into());
        
        let xml = plist::to_format_xml(&dict)?;
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

    async fn check_result(&mut self) -> Result<(), IdeviceError> {
        let result = self.read_plist().await?;
        
        if let Some(error) = result.get("Error") {
            let error_str = error.as_string().unwrap_or("Unknown error");
            return Err(IdeviceError::HouseArrestError(error_str.to_string()));
        }
        
        if let Some(status) = result.get("Status") {
            let status_str = status.as_string().unwrap_or("");
            if status_str != "Complete" {
                return Err(IdeviceError::HouseArrestError(format!("Unexpected status: {}", status_str)));
            }
        } else {
            return Err(IdeviceError::HouseArrestError("No status in response".to_string()));
        }
        
        Ok(())
    }
}