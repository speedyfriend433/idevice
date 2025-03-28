// Add this to your existing IdeviceError enum
#[derive(Debug, thiserror::Error)]
pub enum IdeviceError {
    #[error("AFC error: {0}")]
    AfcError(String),
    
    #[error("Amfi error: {0}")]
    AmfiError(String),
    
    #[error("Companion Proxy error: {0}")]
    CompanionProxyError(String),
    
    #[error("Core Device Proxy error: {0}")]
    CoreDeviceProxyError(String),
    
    #[error("Debug Proxy error: {0}")]
    DebugProxyError(String),
    
    #[error("Diagnostics error: {0}")]
    DiagnosticsError(String),
    
    #[error("DVT error: {0}")]
    DvtError(String),
    
    #[error("File Relay error: {0}")]
    FileRelayError(String),
    
    #[error("Heartbeat error: {0}")]
    HeartbeatError(String),
    
    #[error("House Arrest error: {0}")]
    HouseArrestError(String),
    
    #[error("Installation Proxy error: {0}")]
    InstallationProxyError(String),
    
    #[error("Instproxy error: {0}")]
    InstproxyError(String),
    
    #[error("Misagent error: {0}")]
    MisagentError(String),
    
    #[error("Mobile Backup error: {0}")]
    MobileBackupError(String),
    
    #[error("Mounter error: {0}")]
    MounterError(String),
    
    #[error("Notification Proxy error: {0}")]
    NotificationProxyError(String),
    
    #[error("Screenshot error: {0}")]
    ScreenshotError(String),
    
    #[error("Simulate Location error: {0}")]
    SimulateLocationError(String),
    
    #[error("TCP Tunnel error: {0}")]
    TcpTunnelError(String),
    
    #[error("USBMuxd error: {0}")]
    UsbmuxdError(String),
    
    #[error("Web Inspector error: {0}")]
    WebInspectorError(String),
    
    #[error("XPC error: {0}")]
    XpcError(String),
}