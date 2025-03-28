//! Screenshot service implementation
//!
//! This module provides functionality to capture screenshots from iOS devices.

use crate::{IdeviceError, IdeviceService, ServiceProviderType};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const SCREENSHOTR_SERVICE_NAME: &str = "com.apple.screenshotr";

/// Screenshot client for capturing device screens
pub struct ScreenshotClient {
    socket: tokio::net::TcpStream,
}

impl ScreenshotClient {
    /// Connect to the screenshot service
    pub async fn connect(provider: &dyn ServiceProviderType) -> Result<Self, IdeviceError> {
        let service = provider.start_service(SCREENSHOTR_SERVICE_NAME).await?;
        
        Ok(Self {
            socket: service.socket,
        })
    }

    /// Take a screenshot from the device
    pub async fn take_screenshot(&mut self) -> Result<Vec<u8>, IdeviceError> {
        // Send the screenshot request
        let request = plist::Dictionary::new();
        self.send_plist(&request).await?;
        
        // Receive the response
        let response = self.read_plist().await?;
        
        // Check for errors
        if let Some(status) = response.get("Status") {
            if let Some(status) = status.as_string() {
                if status != "Success" {
                    let error_msg = response.get("Error")
                        .and_then(|e| e.as_string())
                        .unwrap_or("Unknown error");
                    return Err(IdeviceError::ScreenshotError(error_msg.to_string()));
                }
            }
        }
        
        // Extract the image data
        if let Some(data) = response.get("ImageData") {
            if let Some(data) = data.as_data() {
                return Ok(data.to_vec());
            }
        }
        
        Err(IdeviceError::ScreenshotError("No image data received".to_string()))
    }

    /// Save a screenshot to a file
    #[cfg(feature = "image")]
    pub async fn save_screenshot(&mut self, path: &str) -> Result<(), IdeviceError> {
        let data = self.take_screenshot().await?;
        
        // Convert the data to an image
        let img = image::load_from_memory(&data)
            .map_err(|e| IdeviceError::ScreenshotError(format!("Failed to parse image: {}", e)))?;
        
        // Save the image
        img.save(path)
            .map_err(|e| IdeviceError::ScreenshotError(format!("Failed to save image: {}", e)))?;
        
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