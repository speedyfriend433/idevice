// Jackson Coxson
// idevice Rust implementation of File Relay functionality

use clap::{Arg, Command};
use idevice::{file_relay::{FileRelayClient, FileRelaySource}, IdeviceService};
use std::fs::File;
use std::io::Write;

mod common;

#[tokio::main]
async fn main() {
    env_logger::init();

    let matches = Command::new("file_relay_tool")
        .about("Retrieve files and logs from iOS devices")
        .arg(
            Arg::new("host")
                .long("host")
                .value_name("HOST")
                .help("IP address of the device"),
        )
        .arg(
            Arg::new("pairing_file")
                .long("pairing-file")
                .value_name("PATH")
                .help("Path to the pairing file"),
        )
        .arg(
            Arg::new("udid")
                .value_name("UDID")
                .help("UDID of the device (overrides host/pairing file)")
                .index(1),
        )
        .arg(
            Arg::new("about")
                .long("about")
                .help("Show about information")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("source")
                .long("source")
                .short('s')
                .value_name("SOURCE")
                .help("Source to request (can be specified multiple times)")
                .action(clap::ArgAction::Append)
                .value_parser([
                    "AppleSupport", "Network", "VPN", "Wifi", "UserDatabases",
                    "CrashReporter", "Tmp", "SystemConfiguration", "Keyboard",
                    "Logs", "Lockdown", "MobileInstallation", "CrashReporter-Clearable",
                    "Diagnostics", "All"
                ]),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .value_name("FILE")
                .help("Output file path (default: relay.zip)")
                .default_value("relay.zip"),
        )
        .get_matches();

    if matches.get_flag("about") {
        println!("file_relay_tool - retrieve files and logs from iOS devices. Reimplementation of libimobiledevice's functionality.");
        println!("Copyright (c) 2025 Jackson Coxson");
        return;
    }

    let udid = matches.get_one::<String>("udid");
    let host = matches.get_one::<String>("host");
    let pairing_file = matches.get_one::<String>("pairing_file");
    let output_path = matches.get_one::<String>("output").unwrap();

    let provider =
        match common::get_provider(udid, host, pairing_file, "file-relay-tool-jkcoxson").await {
            Ok(p) => p,
            Err(e) => {
                eprintln!("{e}");
                return;
            }
        };

    let mut file_relay_client = match FileRelayClient::connect(&*provider).await {
        Ok(client) => client,
        Err(e) => {
            eprintln!("Failed to connect to File Relay service: {e:?}");
            return;
        }
    };

    // Parse sources
    let sources = match matches.get_many::<String>("source") {
        Some(sources) => {
            let mut result = Vec::new();
            for source in sources {
                match source.as_str() {
                    "AppleSupport" => result.push(FileRelaySource::AppleSupport),
                    "Network" => result.push(FileRelaySource::Network),
                    "VPN" => result.push(FileRelaySource::VPN),
                    "Wifi" => result.push(FileRelaySource::Wifi),
                    "UserDatabases" => result.push(FileRelaySource::UserDatabases),
                    "CrashReporter" => result.push(FileRelaySource::CrashReporter),
                    "Tmp" => result.push(FileRelaySource::Tmp),
                    "SystemConfiguration" => result.push(FileRelaySource::SystemConfiguration),
                    "Keyboard" => result.push(FileRelaySource::Keyboard),
                    "Logs" => result.push(FileRelaySource::Logs),
                    "Lockdown" => result.push(FileRelaySource::Lockdown),
                    "MobileInstallation" => result.push(FileRelaySource::MobileInstallation),
                    "CrashReporter-Clearable" => result.push(FileRelaySource::CrashReporterClearable),
                    "Diagnostics" => result.push(FileRelaySource::Diagnostics),
                    "All" => result.push(FileRelaySource::All),
                    _ => {
                        eprintln!("Unknown source: {}", source);
                        return;
                    }
                }
            }
            result
        }
        None => {
            // Default to All if no sources specified
            vec![FileRelaySource::All]
        }
    };

    println!("Requesting files from sources: {:?}", sources.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    match file_relay_client.request_files(&sources).await {
        Ok(data) => {
            println!("Received {} bytes of data", data.len());
            
            // Save the data to a file
            match File::create(output_path) {
                Ok(mut file) => {
                    match file.write_all(&data) {
                        Ok(_) => {
                            println!("Data saved to: {}", output_path);
                        }
                        Err(e) => {
                            eprintln!("Failed to write data to file: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to create output file: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to request files: {e:?}");
        }
    }
}