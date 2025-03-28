//! Notification Proxy service implementation
//! 
//! This module provides functionality to send and receive notifications to/from iOS devices.

use crate::{IdeviceError, IdeviceService, ServiceProviderType};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use std::collections::HashSet;

const NOTIFICATION_PROXY_SERVICE_NAME: &str = "com.apple.mobile.notification_proxy";

/// Predefined notification types that can be observed or posted
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NotificationType {
    /// Notification sent when a sync is about to start
    SyncWillStart,
    /// Notification sent when a sync has finished
    SyncDidFinish,
    /// Notification sent when a backup is about to start
    BackupWillStart,
    /// Notification sent when a backup has finished
    BackupDidFinish,
    /// Notification sent when a restore is about to start
    RestoreWillStart,
    /// Notification sent when a restore has finished
    RestoreDidFinish,
    /// Notification sent when an app has been installed
    AppInstalled,
    /// Notification sent when a pairing has succeeded
    PairingSucceeded,
    /// Notification sent when iTunes is starting a sync
    ITunesSyncWillStart,
    /// Notification sent when iTunes has finished a sync
    ITunesSyncDidFinish,
    /// Notification sent when a download is about to start
    DownloadWillStart,
    /// Notification sent when a download has finished
    DownloadDidFinish,
    /// Custom notification type
    Custom(String),
}

impl NotificationType {
    fn as_str(&self) -> &str {
        match self {
            NotificationType::SyncWillStart => "com.apple.itunes-client.syncWillStart",
            NotificationType::SyncDidFinish => "com.apple.itunes-client.syncDidFinish",
            NotificationType::BackupWillStart => "com.apple.itunes-client.backupWillStart",
            NotificationType::BackupDidFinish => "com.apple.itunes-client.backupDidFinish",
            NotificationType::RestoreWillStart => "com.apple.itunes-client.restoreWillStart",
            NotificationType::RestoreDidFinish => "com.apple.itunes-client.restoreDidFinish",
            NotificationType::AppInstalled => "com.apple.mobile.application_installed",
            NotificationType::PairingSucceeded => "com.apple.mobile.paired",
            NotificationType::ITunesSyncWillStart => "com.apple.itunes-mobdev.syncWillStart",
            NotificationType::ITunesSyncDidFinish => "com.apple.itunes-mobdev.syncDidFinish",
            NotificationType::DownloadWillStart => "com.apple.mobile.data_sync.willStart",
            NotificationType::DownloadDidFinish => "com.apple.mobile.data_sync.didFinish",
            NotificationType::Custom(s) => s,
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "com.apple.itunes-client.syncWillStart" => NotificationType::SyncWillStart,
            "com.apple.itunes-client.syncDidFinish" => NotificationType::SyncDidFinish,
            "com.apple.itunes-client.backupWillStart" => NotificationType::BackupWillStart,
            "com.apple.itunes-client.backupDidFinish" => NotificationType::BackupDidFinish,
            "com.apple.itunes-client.restoreWillStart" => NotificationType::RestoreWillStart,
            "com.apple.itunes-client.restoreDidFinish" => NotificationType::RestoreDidFinish,
            "com.apple.mobile.application_installed" => NotificationType::AppInstalled,
            "com.apple.mobile.paired" => NotificationType::PairingSucceeded,
            "com.apple.itunes-mobdev.syncWillStart" => NotificationType::ITunesSyncWillStart,
            "com.apple.itunes-mobdev.syncDidFinish" => NotificationType::ITunesSyncDidFinish,
            "com.apple.mobile.data_sync.willStart" => NotificationType::DownloadWillStart,
            "com.apple.mobile.data_sync.didFinish" => NotificationType::DownloadDidFinish,
            _ => NotificationType::Custom(s.to_string()),
        }
    }
}

/// Notification Proxy client for sending and receiving notifications
pub struct NotificationProxyClient {
    socket: tokio::net::TcpStream,
    notification_rx: Option<mpsc::Receiver<NotificationType>>,
    notification_tx: Option<mpsc::Sender<NotificationType>>,
}

impl NotificationProxyClient {
    /// Connect to the Notification Proxy service
    pub async fn connect(provider: &dyn ServiceProviderType) -> Result<Self, IdeviceError> {
        let service = provider.start_service(NOTIFICATION_PROXY_SERVICE_NAME).await?;
        
        Ok(Self {
            socket: service.socket,
            notification_rx: None,
            notification_tx: None,
        })
    }

    /// Observe a notification type
    pub async fn observe_notification(&mut self, notification: NotificationType) -> Result<(), IdeviceError> {
        let mut command = vec![0u8; 2];
        command[0] = 'O' as u8;
        command[1] = 'N' as u8;
        
        let notification_str = notification.as_str();
        let notification_bytes = notification_str.as_bytes();
        
        // Send the command
        self.socket.write_all(&command).await?;
        
        // Send the notification length as a 32-bit big-endian integer
        let len = (notification_bytes.len() as u32).to_be_bytes();
        self.socket.write_all(&len).await?;
        
        // Send the notification string
        self.socket.write_all(notification_bytes).await?;
        
        Ok(())
    }

    /// Post a notification
    pub async fn post_notification(&mut self, notification: NotificationType) -> Result<(), IdeviceError> {
        let mut command = vec![0u8; 2];
        command[0] = 'P' as u8;
        command[1] = 'N' as u8;
        
        let notification_str = notification.as_str();
        let notification_bytes = notification_str.as_bytes();
        
        // Send the command
        self.socket.write_all(&command).await?;
        
        // Send the notification length as a 32-bit big-endian integer
        let len = (notification_bytes.len() as u32).to_be_bytes();
        self.socket.write_all(&len).await?;
        
        // Send the notification string
        self.socket.write_all(notification_bytes).await?;
        
        Ok(())
    }

    /// Start listening for notifications
    pub async fn start_listening(&mut self) -> Result<mpsc::Receiver<NotificationType>, IdeviceError> {
        if self.notification_rx.is_some() {
            return Err(IdeviceError::NotificationProxyError("Already listening for notifications".to_string()));
        }
        
        let (tx, rx) = mpsc::channel(100);
        self.notification_tx = Some(tx.clone());
        self.notification_rx = Some(rx.clone());
        
        let mut socket = self.socket.try_clone().map_err(|e| {
            IdeviceError::NotificationProxyError(format!("Failed to clone socket: {}", e))
        })?;
        
        // Spawn a task to listen for notifications
        tokio::spawn(async move {
            loop {
                // Read the command
                let mut command = [0u8; 2];
                if let Err(_) = socket.read_exact(&mut command).await {
                    break;
                }
                
                // Check if it's a notification
                if command[0] == 'N' as u8 && command[1] == 'P' as u8 {
                    // Read the notification length
                    let mut len_buf = [0u8; 4];
                    if let Err(_) = socket.read_exact(&mut len_buf).await {
                        break;
                    }
                    let len = u32::from_be_bytes(len_buf) as usize;
                    
                    // Read the notification string
                    let mut notification_bytes = vec![0u8; len];
                    if let Err(_) = socket.read_exact(&mut notification_bytes).await {
                        break;
                    }
                    
                    // Convert to string
                    if let Ok(notification_str) = String::from_utf8(notification_bytes) {
                        let notification = NotificationType::from_str(&notification_str);
                        
                        // Send the notification to the channel
                        if let Err(_) = tx.send(notification).await {
                            break;
                        }
                    }
                }
            }
        });
        
        Ok(rx)
    }

    /// Stop listening for notifications
    pub fn stop_listening(&mut self) {
        self.notification_rx = None;
        self.notification_tx = None;
    }

    /// Observe multiple notification types
    pub async fn observe_notifications(&mut self, notifications: &[NotificationType]) -> Result<(), IdeviceError> {
        for notification in notifications {
            self.observe_notification(notification.clone()).await?;
        }
        
        Ok(())
    }
}