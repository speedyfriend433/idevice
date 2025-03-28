//! Web Inspector service implementation

use crate::{IdeviceError, IdeviceService, ServiceProviderType};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const WEB_INSPECTOR_SERVICE_NAME: &str = "com.apple.webinspector";

/// Web Inspector client for debugging web content
pub struct WebInspectorClient {
    socket: tokio::net::TcpStream,
}

impl WebInspectorClient {
    /// Connect to the Web Inspector service
    pub async fn connect(provider: &dyn ServiceProviderType) -> Result<Self, IdeviceError> {
        let service = provider.start_service(WEB_INSPECTOR_SERVICE_NAME).await?;
        Ok(Self {
            socket: service.socket,
        })
    }

    /// Get list of inspectable applications
    pub async fn get_applications(&mut self) -> Result<Vec<String>, IdeviceError> {
        let mut command = [0u8; 4];
        command[0] = b'L';
        self.socket.write_all(&command).await?;
        
        let mut len_buf = [0u8; 4];
        self.socket.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        
        let mut data = vec![0u8; len];
        self.socket.read_exact(&mut data).await?;
        
        let plist: plist::Value = plist::from_bytes(&data)
            .map_err(|e| IdeviceError::WebInspectorError(e.to_string()))?;

        // Parse applications from plist structure
        let applications = plist.as_dictionary()
            .and_then(|dict| dict.get("Applications"))
            .and_then(|apps| apps.as_array())
            .ok_or_else(|| IdeviceError::WebInspectorError("Invalid application list structure".into()))?;

        let mut app_list = Vec::new();
        for app in applications {
            if let Some(name) = app.as_dictionary()
                .and_then(|app_dict| app_dict.get("Name"))
                .and_then(|name_val| name_val.as_string()) 
            {
                app_list.push(name.to_string());
            }
        }

        Ok(app_list)
    }

    /// Connect to a specific web view
    pub async fn connect_to_webview(&mut self, app_id: &str) -> Result<tokio_tungstenite::WebSocketStream<
        tokio::net::TcpStream
    >, IdeviceError> {
        // Send connect command
        let mut command = [0u8; 4];
        command[0] = b'C';
        self.socket.write_all(&command).await?;
        
        // Send application ID
        let app_id_bytes = app_id.as_bytes();
        let len = (app_id_bytes.len() as u32).to_be_bytes();
        self.socket.write_all(&len).await?;
        self.socket.write_all(app_id_bytes).await?;

        // Read WebSocket connection details
        let mut len_buf = [0u8; 4];
        self.socket.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        
        let mut data = vec![0u8; len];
        self.socket.read_exact(&mut data).await?;
        
        let plist: plist::Value = plist::from_bytes(&data)
            .map_err(|e| IdeviceError::WebInspectorError(e.to_string()))?;

        // Parse WebSocket connection details
        let ws_url = plist.as_dictionary()
            .and_then(|dict| dict.get("WebSocketURL"))
            .and_then(|url| url.as_string())
            .ok_or_else(|| IdeviceError::WebInspectorError("Missing WebSocket URL".into()))?;

        // Establish WebSocket connection
        let (ws_stream, _) = tokio_tungstenite::connect_async(ws_url)
            .await
            .map_err(|e| IdeviceError::WebInspectorError(e.to_string()))?;

        Ok(ws_stream)
    }

    /// Forward developer tools protocol messages
    pub async fn forward_messages(
        ws_stream: &mut tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    ) -> Result<(), IdeviceError> {
        loop {
            match ws_stream.next().await {
                Some(Ok(msg)) => {
                    // Handle incoming WebSocket messages
                    if let tungstenite::Message::Text(text) = msg {
                        println!("Received DevTools message: {}", text);
                    }
                }
                Some(Err(e)) => return Err(IdeviceError::WebInspectorError(e.to_string())),
                None => break,
            }
        }
        Ok(())
    }
}