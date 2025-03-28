// Jackson Coxson

use std::{
    net::{AddrParseError, IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    str::FromStr,
};

#[cfg(not(unix))]
use std::net::SocketAddrV4;

use log::{debug, warn};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{
    pairing_file::PairingFile, provider::UsbmuxdProvider, Idevice, IdeviceError, ReadWrite,
};

mod des;
mod raw_packet;

#[derive(Debug, Clone)]
pub enum Connection {
    Usb,
    Network(IpAddr),
    Unknown(String),
}

#[derive(Debug, Clone)]
pub struct UsbmuxdDevice {
    pub connection_type: Connection,
    pub udid: String,
    pub device_id: u32,
}

pub struct UsbmuxdConnection {
    socket: Box<dyn ReadWrite>,
    tag: u32,
}

#[derive(Clone, Debug)]
pub enum UsbmuxdAddr {
    #[cfg(unix)]
    UnixSocket(String),
    TcpSocket(SocketAddr),
}

impl UsbmuxdAddr {
    pub const DEFAULT_PORT: u16 = 27015;
    pub const SOCKET_FILE: &'static str = "/var/run/usbmuxd";

    pub async fn to_socket(&self) -> Result<Box<dyn ReadWrite>, IdeviceError> {
        Ok(match self {
            #[cfg(unix)]
            Self::UnixSocket(addr) => Box::new(tokio::net::UnixStream::connect(addr).await?),
            Self::TcpSocket(addr) => Box::new(tokio::net::TcpStream::connect(addr).await?),
        })
    }

    pub async fn connect(&self, tag: u32) -> Result<UsbmuxdConnection, IdeviceError> {
        let socket = self.to_socket().await?;
        Ok(UsbmuxdConnection::new(socket, tag))
    }

    pub fn from_env_var() -> Result<Self, AddrParseError> {
        Ok(match std::env::var("USBMUXD_SOCKET_ADDRESS") {
            Ok(var) => {
                #[cfg(unix)]
                if var.contains(':') {
                    Self::TcpSocket(SocketAddr::from_str(&var)?)
                } else {
                    Self::UnixSocket(var)
                }
                #[cfg(not(unix))]
                Self::TcpSocket(SocketAddr::from_str(&var)?)
            }
            Err(_) => Self::default(),
        })
    }
}

impl Default for UsbmuxdAddr {
    fn default() -> Self {
        #[cfg(not(unix))]
        {
            Self::TcpSocket(SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::new(127, 0, 0, 1),
                Self::DEFAULT_PORT,
            )))
        }
        #[cfg(unix)]
        Self::UnixSocket(Self::SOCKET_FILE.to_string())
    }
}

impl UsbmuxdConnection {
    pub const BINARY_PLIST_VERSION: u32 = 0;
    pub const XML_PLIST_VERSION: u32 = 1;

    pub const RESULT_MESSAGE_TYPE: u32 = 1;
    pub const PLIST_MESSAGE_TYPE: u32 = 8;

    pub async fn default() -> Result<Self, IdeviceError> {
        let socket = UsbmuxdAddr::default().to_socket().await?;

        Ok(Self {
            socket: Box::new(socket),
            tag: 0,
        })
    }

    pub fn new(socket: Box<dyn ReadWrite>, tag: u32) -> Self {
        Self { socket, tag }
    }

    pub async fn get_devices(&mut self) -> Result<Vec<UsbmuxdDevice>, IdeviceError> {
        let mut req = plist::Dictionary::new();
        req.insert("MessageType".into(), "ListDevices".into());
        req.insert("ClientVersionString".into(), "idevice-rs".into());
        req.insert("kLibUSBMuxVersion".into(), 3.into());
        self.write_plist(req).await?;
        let res = self.read_plist().await?;
        let res = plist::to_value(&res)?;
        let res = plist::from_value::<des::ListDevicesResponse>(&res)?;

        let mut devs = Vec::new();
        for dev in res.device_list {
            let connection_type = match dev.properties.connection_type.as_str() {
                "Network" => {
                    if let Some(addr) = dev.properties.network_address {
                        let addr = &Into::<Vec<u8>>::into(addr);
                        if addr.len() < 8 {
                            warn!("Device address bytes len < 8");
                            return Err(IdeviceError::UnexpectedResponse);
                        }

                        match addr[0] {
                            0x02 => {
                                // ipv4
                                Connection::Network(IpAddr::V4(Ipv4Addr::new(
                                    addr[4], addr[5], addr[6], addr[7],
                                )))
                            }
                            0x1E => {
                                // ipv6
                                if addr.len() < 24 {
                                    warn!("IPv6 address is less than 24 bytes");
                                    return Err(IdeviceError::UnexpectedResponse);
                                }

                                Connection::Network(IpAddr::V6(Ipv6Addr::new(
                                    u16::from_be_bytes([addr[8], addr[9]]),
                                    u16::from_be_bytes([addr[10], addr[11]]),
                                    u16::from_be_bytes([addr[12], addr[13]]),
                                    u16::from_be_bytes([addr[14], addr[15]]),
                                    u16::from_be_bytes([addr[16], addr[17]]),
                                    u16::from_be_bytes([addr[18], addr[19]]),
                                    u16::from_be_bytes([addr[20], addr[21]]),
                                    u16::from_be_bytes([addr[22], addr[23]]),
                                )))
                            }
                            _ => {
                                warn!("Unknown IP address protocol: {:02X}", addr[0]);
                                Connection::Unknown(format!("Network {:02X}", addr[0]))
                            }
                        }
                    } else {
                        warn!("Device is network attached, but has no network info");
                        return Err(IdeviceError::UnexpectedResponse);
                    }
                }
                "USB" => Connection::Usb,
                _ => Connection::Unknown(dev.properties.connection_type),
            };
            debug!("Connection type: {connection_type:?}");
            devs.push(UsbmuxdDevice {
                connection_type,
                udid: dev.properties.serial_number,
                device_id: dev.device_id,
            })
        }

        Ok(devs)
    }

    pub async fn get_device(&mut self, udid: &str) -> Result<UsbmuxdDevice, IdeviceError> {
        let devices = self.get_devices().await?;
        match devices.into_iter().find(|x| x.udid == udid) {
            Some(d) => Ok(d),
            None => Err(IdeviceError::DeviceNotFound),
        }
    }

    pub async fn get_pair_record(&mut self, udid: &str) -> Result<PairingFile, IdeviceError> {
        debug!("Getting pair record for {udid}");
        let mut req = plist::Dictionary::new();
        req.insert("MessageType".into(), "ReadPairRecord".into());
        req.insert("PairRecordID".into(), udid.into());
        self.write_plist(req).await?;
        let res = self.read_plist().await?;

        match res.get("PairRecordData") {
            Some(plist::Value::Data(d)) => PairingFile::from_bytes(d),
            _ => Err(IdeviceError::UnexpectedResponse),
        }
    }

    pub async fn get_buid(&mut self) -> Result<String, IdeviceError> {
        let mut req = plist::Dictionary::new();
        req.insert("MessageType".into(), "ReadBUID".into());
        self.write_plist(req).await?;
        let mut res = self.read_plist().await?;

        match res.remove("BUID") {
            Some(plist::Value::String(s)) => Ok(s),
            _ => Err(IdeviceError::UnexpectedResponse),
        }
    }

    pub async fn connect_to_device(
        mut self,
        device_id: u32,
        port: u16,
        label: impl Into<String>,
    ) -> Result<Idevice, IdeviceError> {
        debug!("Connecting to device {device_id} on port {port}");
        let port = port.to_be();

        let mut req = plist::Dictionary::new();
        req.insert("MessageType".into(), "Connect".into());
        req.insert("DeviceID".into(), device_id.into());
        req.insert("PortNumber".into(), port.into());
        self.write_plist(req).await?;
        match self.read_plist().await?.get("Number") {
            Some(plist::Value::Integer(i)) => match i.as_unsigned() {
                Some(0) => Ok(Idevice::new(self.socket, label)),
                Some(1) => Err(IdeviceError::UsbBadCommand),
                Some(2) => Err(IdeviceError::UsbBadDevice),
                Some(3) => Err(IdeviceError::UsbConnectionRefused),
                Some(6) => Err(IdeviceError::UsbBadVersion),
                _ => Err(IdeviceError::UnexpectedResponse),
            },
            _ => Err(IdeviceError::UnexpectedResponse),
        }
    }

    async fn write_plist(&mut self, req: plist::Dictionary) -> Result<(), IdeviceError> {
        let raw = raw_packet::RawPacket::new(
            req,
            Self::XML_PLIST_VERSION,
            Self::PLIST_MESSAGE_TYPE,
            self.tag,
        );

        let raw: Vec<u8> = raw.into();
        self.socket.write_all(&raw).await?;

        Ok(())
    }

    async fn read_plist(&mut self) -> Result<plist::Dictionary, IdeviceError> {
        let mut header_buffer = [0; 16];
        self.socket.read_exact(&mut header_buffer).await?;

        // We are safe to unwrap as it only panics if the buffer isn't 4
        let packet_size = u32::from_le_bytes(header_buffer[..4].try_into().unwrap()) - 16;
        debug!("Reading {packet_size} bytes from muxer");

        let mut body_buffer = vec![0; packet_size as usize];
        self.socket.read_exact(&mut body_buffer).await?;

        let res = plist::from_bytes(&body_buffer)?;
        debug!("Read from muxer: {}", crate::pretty_print_dictionary(&res));

        Ok(res)
    }
}

impl UsbmuxdDevice {
    pub fn to_provider(
        &self,
        addr: UsbmuxdAddr,
        tag: u32,
        label: impl Into<String>,
    ) -> UsbmuxdProvider {
        let label = label.into();

        UsbmuxdProvider {
            addr,
            tag,
            udid: self.udid.clone(),
            device_id: self.device_id,
            label,
        }
    }
}
