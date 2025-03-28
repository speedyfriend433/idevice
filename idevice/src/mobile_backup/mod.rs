//! Mobile Backup service implementation
//! 
//! This module provides functionality for device backup and restore operations.

use crate::{IdeviceError, IdeviceService, ServiceProviderType};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::path::Path;

const MOBILE_BACKUP_SERVICE_NAME: &str = "com.apple.mobile.backup";

/// Backup types supported by the service
#[derive(Debug, Clone, Copy)]
pub enum BackupType {
    /// Full device backup
    Full,
    /// Incremental backup
    Incremental,
}

/// Mobile Backup client for iOS device backup/restore operations
pub struct MobileBackupClient {
    socket: tokio::net::TcpStream,
}

impl MobileBackupClient {
    /// Connect to the Mobile Backup service
    pub async fn connect(provider: &dyn ServiceProviderType) -> Result<Self, IdeviceError> {
        let service = provider.start_service(MOBILE_BACKUP_SERVICE_NAME).await?;
        Ok(Self {
            socket: service.socket,
        })
    }

    /// Start a backup operation
    pub async fn start_backup(
        &mut self,
        backup_type: BackupType,
        target_dir: &Path,
        encryption_key: Option<&str>,
    ) -> Result<(), IdeviceError> {
        let mut dict = plist::Dictionary::new();
        dict.insert("MessageName".into(), "InitiateBackup".into());
        dict.insert("BackupType".into(), match backup_type {
            BackupType::Full => "Full",
            BackupType::Incremental => "Incremental",
        }.into());
        dict.insert("TargetDirectory".into(), target_dir.to_str().unwrap().into());
        
        if let Some(key) = encryption_key {
            dict.insert("EncryptionKey".into(), key.into());
        }

        self.send_plist(&dict).await?;
        self.read_confirmation().await
    }

    /// Start a restore operation
    pub async fn start_restore(
        &mut self,
        backup_dir: &Path,
        encryption_key: Option<&str>,
    ) -> Result<(), IdeviceError> {
        let mut dict = plist::Dictionary::new();
        dict.insert("MessageName".into(), "InitiateRestore".into());
        dict.insert("BackupDirectory".into(), backup_dir.to_str().unwrap().into());
        
        if let Some(key) = encryption_key {
            dict.insert("EncryptionKey".into(), key.into());
        }

        self.send_plist(&dict).await?;
        self.read_confirmation().await
    }

    /// Get backup information
    pub async fn get_backup_info(&mut self) -> Result<plist::Value, IdeviceError> {
        let dict = plist::Dictionary::from_iter(vec![
            ("MessageName".into(), "GetBackupInfo".into())
        ]);
        
        self.send_plist(&dict).await?;
        self.read_plist().await
    }

    // Helper methods
    async fn send_plist(&mut self, dict: &plist::Dictionary) -> Result<(), IdeviceError> {
        let xml = plist::to_format_xml(dict)?;
        let xml_bytes = xml.into_bytes();
        
        let len = (xml_bytes.len() as u32).to_be_bytes();
        self.socket.write_all(&len).await?;
        self.socket.write_all(&xml_bytes).await?;
        Ok(())
    }

    async fn read_plist(&mut self) -> Result<plist::Value, IdeviceError> {
        let mut len_buf = [0u8; 4];
        self.socket.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        
        let mut data = vec![0u8; len];
        self.socket.read_exact(&mut data).await?;
        plist::from_bytes(&data).map_err(Into::into)
    }

    async fn read_confirmation(&mut self) -> Result<(), IdeviceError> {
        let response = self.read_plist().await?;
        if let Some(status) = response.as_dictionary().and_then(|d| d.get("Status")) {
            if status.as_string() != Some("Success") {
                return Err(IdeviceError::MobileBackupError(
                    response.as_dictionary()
                        .and_then(|d| d.get("Error"))
                        .and_then(|e| e.as_string())
                        .unwrap_or("Unknown error")
                        .to_string()
                ));
            }
        }
        Ok(())
    }
}