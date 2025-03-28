//! Apple File Connection (AFC) service implementation
//! 
//! This module provides functionality to interact with the iOS device's filesystem
//! through the AFC protocol.

use crate::{IdeviceError, IdeviceService, ServiceProviderType};
use std::path::Path;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::collections::HashMap;

const AFC_SERVICE_NAME: &str = "com.apple.afc";

/// AFC operation codes
#[repr(u64)]
enum AfcOperations {
    Status = 0x00000001,
    Data = 0x00000002,
    ReadDir = 0x00000003,
    ReadFile = 0x00000004,
    WriteFile = 0x00000005,
    WritePart = 0x00000006,
    TruncFile = 0x00000007,
    RemovePath = 0x00000008,
    MakeDir = 0x00000009,
    GetFileInfo = 0x0000000a,
    GetDeviceInfo = 0x0000000b,
    WriteFileAtomic = 0x0000000c,
    FileRefOpen = 0x0000000d,
    FileRefRead = 0x0000000e,
    FileRefWrite = 0x0000000f,
    FileRefSeek = 0x00000010,
    FileRefTell = 0x00000011,
    FileRefClose = 0x00000012,
    FileRefSetSize = 0x00000013,
    GetConnectionInfo = 0x00000014,
    SetConnectionOptions = 0x00000015,
    RenamePath = 0x00000016,
    SetFSBlockSize = 0x00000017,
    SetSocketBlockSize = 0x00000018,
    FileRefLock = 0x00000019,
    MakeLink = 0x0000001a,
    GetFileHash = 0x0000001b,
    SetModTime = 0x0000001c,
    GetFileHashWithRange = 0x0000001d,
    FileRefSetImmutableHint = 0x0000001e,
    GetSizeOfPathContents = 0x0000001f,
    RemovePathAndContents = 0x00000020,
    DirectoryEnumeratorRefOpen = 0x00000021,
    DirectoryEnumeratorRefOpenDirectory = 0x00000022,
    DirectoryEnumeratorRefRead = 0x00000023,
    DirectoryEnumeratorRefClose = 0x00000024,
}

/// AFC packet header
#[derive(Debug)]
struct AfcPacketHeader {
    entire_length: u64,
    this_length: u64,
    packet_num: u64,
    operation: u64,
}

impl AfcPacketHeader {
    fn new(operation: AfcOperations, data_length: u64) -> Self {
        Self {
            entire_length: 40 + data_length, // header (40 bytes) + data length
            this_length: 40 + data_length,
            packet_num: 0,
            operation: operation as u64,
        }
    }

    async fn serialize(&self, writer: &mut tokio::net::TcpStream) -> Result<(), IdeviceError> {
        writer.write_u64(self.entire_length).await?;
        writer.write_u64(self.this_length).await?;
        writer.write_u64(self.packet_num).await?;
        writer.write_u64(self.operation).await?;
        writer.write_u64(0).await?; // Reserved
        Ok(())
    }

    async fn deserialize(reader: &mut tokio::net::TcpStream) -> Result<Self, IdeviceError> {
        let entire_length = reader.read_u64().await?;
        let this_length = reader.read_u64().await?;
        let packet_num = reader.read_u64().await?;
        let operation = reader.read_u64().await?;
        let _reserved = reader.read_u64().await?;

        Ok(Self {
            entire_length,
            this_length,
            packet_num,
            operation,
        })
    }
}

/// AFC client for interacting with the iOS device's filesystem
pub struct AfcClient {
    socket: tokio::net::TcpStream,
    packet_num: u64,
}

impl AfcClient {
    /// Connect to the AFC service
    pub async fn connect(provider: &dyn ServiceProviderType) -> Result<Self, IdeviceError> {
        let service = provider.start_service(AFC_SERVICE_NAME).await?;
        
        Ok(Self {
            socket: service.socket,
            packet_num: 0,
        })
    }

    /// Get device info
    pub async fn get_device_info(&mut self) -> Result<HashMap<String, String>, IdeviceError> {
        self.send_packet(AfcOperations::GetDeviceInfo, &[]).await?;
        let response = self.receive_response().await?;
        
        let mut info = HashMap::new();
        let mut key = None;
        
        for (i, item) in response.split(|&b| b == 0).enumerate() {
            if item.is_empty() {
                continue;
            }
            
            let s = String::from_utf8_lossy(item).to_string();
            
            if i % 2 == 0 {
                key = Some(s);
            } else if let Some(k) = key.take() {
                info.insert(k, s);
            }
        }
        
        Ok(info)
    }

    /// Read directory contents
    pub async fn read_directory(&mut self, path: &str) -> Result<Vec<String>, IdeviceError> {
        let path_bytes = path.as_bytes();
        let mut data = vec![0; path_bytes.len() + 1]; // +1 for null terminator
        data[..path_bytes.len()].copy_from_slice(path_bytes);
        
        self.send_packet(AfcOperations::ReadDir, &data).await?;
        let response = self.receive_response().await?;
        
        let mut entries = Vec::new();
        for item in response.split(|&b| b == 0) {
            if !item.is_empty() {
                entries.push(String::from_utf8_lossy(item).to_string());
            }
        }
        
        // Remove the last empty entry if it exists
        if entries.last().map_or(false, |s| s.is_empty()) {
            entries.pop();
        }
        
        Ok(entries)
    }

    /// Get file info
    pub async fn get_file_info(&mut self, path: &str) -> Result<HashMap<String, String>, IdeviceError> {
        let path_bytes = path.as_bytes();
        let mut data = vec![0; path_bytes.len() + 1]; // +1 for null terminator
        data[..path_bytes.len()].copy_from_slice(path_bytes);
        
        self.send_packet(AfcOperations::GetFileInfo, &data).await?;
        let response = self.receive_response().await?;
        
        let mut info = HashMap::new();
        let mut key = None;
        
        for (i, item) in response.split(|&b| b == 0).enumerate() {
            if item.is_empty() {
                continue;
            }
            
            let s = String::from_utf8_lossy(item).to_string();
            
            if i % 2 == 0 {
                key = Some(s);
            } else if let Some(k) = key.take() {
                info.insert(k, s);
            }
        }
        
        Ok(info)
    }

    /// Create directory
    pub async fn make_directory(&mut self, path: &str) -> Result<(), IdeviceError> {
        let path_bytes = path.as_bytes();
        let mut data = vec![0; path_bytes.len() + 1]; // +1 for null terminator
        data[..path_bytes.len()].copy_from_slice(path_bytes);
        
        self.send_packet(AfcOperations::MakeDir, &data).await?;
        let _ = self.receive_response().await?;
        
        Ok(())
    }

    /// Remove path (file or empty directory)
    pub async fn remove_path(&mut self, path: &str) -> Result<(), IdeviceError> {
        let path_bytes = path.as_bytes();
        let mut data = vec![0; path_bytes.len() + 1]; // +1 for null terminator
        data[..path_bytes.len()].copy_from_slice(path_bytes);
        
        self.send_packet(AfcOperations::RemovePath, &data).await?;
        let _ = self.receive_response().await?;
        
        Ok(())
    }

    /// Rename path
    pub async fn rename_path(&mut self, from_path: &str, to_path: &str) -> Result<(), IdeviceError> {
        let from_bytes = from_path.as_bytes();
        let to_bytes = to_path.as_bytes();
        
        let mut data = vec![0; from_bytes.len() + 1 + to_bytes.len() + 1];
        data[..from_bytes.len()].copy_from_slice(from_bytes);
        data[from_bytes.len() + 1..from_bytes.len() + 1 + to_bytes.len()].copy_from_slice(to_bytes);
        
        self.send_packet(AfcOperations::RenamePath, &data).await?;
        let _ = self.receive_response().await?;
        
        Ok(())
    }

    /// Read file
    pub async fn read_file(&mut self, path: &str) -> Result<Vec<u8>, IdeviceError> {
        // Open file
        let path_bytes = path.as_bytes();
        let mut data = vec![0; path_bytes.len() + 1]; // +1 for null terminator
        data[..path_bytes.len()].copy_from_slice(path_bytes);
        
        self.send_packet(AfcOperations::FileRefOpen, &data).await?;
        let response = self.receive_response().await?;
        
        if response.len() < 8 {
            return Err(IdeviceError::AfcError("Failed to open file".to_string()));
        }
        
        let file_handle = u64::from_le_bytes([
            response[0], response[1], response[2], response[3],
            response[4], response[5], response[6], response[7],
        ]);
        
        // Read file content
        let mut file_content = Vec::new();
        let chunk_size = 65536; // 64KB chunks
        
        loop {
            let mut read_data = vec![0; 8 + 8];
            read_data[..8].copy_from_slice(&file_handle.to_le_bytes());
            read_data[8..].copy_from_slice(&chunk_size.to_le_bytes());
            
            self.send_packet(AfcOperations::FileRefRead, &read_data).await?;
            let chunk = self.receive_response().await?;
            
            if chunk.is_empty() {
                break;
            }
            
            file_content.extend_from_slice(&chunk);
            
            if chunk.len() < chunk_size as usize {
                break;
            }
        }
        
        // Close file
        let close_data = file_handle.to_le_bytes().to_vec();
        self.send_packet(AfcOperations::FileRefClose, &close_data).await?;
        let _ = self.receive_response().await?;
        
        Ok(file_content)
    }

    /// Write file
    pub async fn write_file(&mut self, path: &str, data: &[u8]) -> Result<(), IdeviceError> {
        // Open file with write mode (3)
        let path_bytes = path.as_bytes();
        let mut open_data = vec![0; path_bytes.len() + 1 + 8]; // path + null + mode
        open_data[..path_bytes.len()].copy_from_slice(path_bytes);
        // Write mode (3) at the end
        open_data[path_bytes.len() + 1..].copy_from_slice(&3u64.to_le_bytes());
        
        self.send_packet(AfcOperations::FileRefOpen, &open_data).await?;
        let response = self.receive_response().await?;
        
        if response.len() < 8 {
            return Err(IdeviceError::AfcError("Failed to open file for writing".to_string()));
        }
        
        let file_handle = u64::from_le_bytes([
            response[0], response[1], response[2], response[3],
            response[4], response[5], response[6], response[7],
        ]);
        
        // Write data in chunks
        let chunk_size = 65536; // 64KB chunks
        
        for chunk in data.chunks(chunk_size) {
            let mut write_data = vec![0; 8];
            write_data[..8].copy_from_slice(&file_handle.to_le_bytes());
            write_data.extend_from_slice(chunk);
            
            self.send_packet(AfcOperations::FileRefWrite, &write_data).await?;
            let _ = self.receive_response().await?;
        }
        
        // Close file
        let close_data = file_handle.to_le_bytes().to_vec();
        self.send_packet(AfcOperations::FileRefClose, &close_data).await?;
        let _ = self.receive_response().await?;
        
        Ok(())
    }

    // Helper methods
    async fn send_packet(&mut self, operation: AfcOperations, data: &[u8]) -> Result<(), IdeviceError> {
        let header = AfcPacketHeader::new(operation, data.len() as u64);
        header.serialize(&mut self.socket).await?;
        
        if !data.is_empty() {
            self.socket.write_all(data).await?;
        }
        
        self.packet_num += 1;
        Ok(())
    }

    async fn receive_response(&mut self) -> Result<Vec<u8>, IdeviceError> {
        let header = AfcPacketHeader::deserialize(&mut self.socket).await?;
        
        let data_length = (header.entire_length - 40) as usize;
        if data_length > 0 {
            let mut data = vec![0; data_length];
            self.socket.read_exact(&mut data).await?;
            Ok(data)
        } else {
            Ok(Vec::new())
        }
    }
}