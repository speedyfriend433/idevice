// Jackson Coxson
// idevice Rust implementation of Diagnostics functionality

use clap::{Arg, Command};
use idevice::{diagnostics::{DiagnosticsClient, DiagnosticsAction, DiagnosticsDomain}, IdeviceService};
use std::fs::File;
use std::io::Write;

mod common;

#[tokio::main]
async fn main() {
    env_logger::init();

    let matches = Command::new("diagnostics_tool")
        .about("Retrieve diagnostic information from iOS devices")
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
            Arg::new("all")
                .long("all")
                .short('a')
                .help("Request all diagnostics")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("wifi")
                .long("wifi")
                .short('w')
                .help("Request WiFi diagnostics")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("battery")
                .long("battery")
                .short('b')
                .help("Request battery (GasGauge) diagnostics")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("storage")
                .long("storage")
                .short('s')
                .help("Request storage (NAND) diagnostics")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("ioreg")
                .long("ioreg")
                .short('i')
                .help("Request I/O Registry")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("network")
                .long("network")
                .short('n')
                .help("Request network interfaces")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("restart")
                .long("restart")
                .short('r')
                .help("Restart the device")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("shutdown")
                .long("shutdown")
                .help("Shutdown the device")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("sleep")
                .long("sleep")
                .help("Sleep the device")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .value_name("FILE")
                .help("Output file path (default: output to console)"),
        )
        .get_matches();

    if matches.get_flag("about") {
        println!("diagnostics_tool - retrieve diagnostic information from iOS devices. Reimplementation of libimobiledevice's functionality.");
        println!("Copyright (c) 2025 Jackson Coxson");
        return;
    }

    let udid = matches.get_one::<String>("udid");
    let host = matches.get_one::<String>("host");
    let pairing_file = matches.get_one::<String>("pairing_file");
    let output_path = matches.get_one::<String>("output");

    let provider =
        match common::get_provider(udid, host, pairing_file, "diagnostics-tool-jkcoxson").await {
            Ok(p) => p,
            Err(e) => {
                eprintln!("{e}");
                return;
            }
        };

    let mut diagnostics_client = match DiagnosticsClient::connect(&*provider).await {
        Ok(client) => client,
        Err(e) => {
            eprintln!("Failed to connect to Diagnostics service: {e:?}");
            return;
        }
    };

    // Determine the action to perform
    let action = if matches.get_flag("all") {
        DiagnosticsAction::All
    } else if matches.get_flag("wifi") {
        DiagnosticsAction::Domain(DiagnosticsDomain::WiFi)
    } else if matches.get_flag("battery") {
        DiagnosticsAction::Domain(DiagnosticsDomain::GasGauge)
    } else if matches.get_flag("storage") {
        DiagnosticsAction::Domain(DiagnosticsDomain::NAND)
    } else if matches.get_flag("ioreg") {
        DiagnosticsAction::IORegistry
    } else if matches.get_flag("network") {
        DiagnosticsAction::NetworkInterfaces
    } else if matches.get_flag("restart") {
        DiagnosticsAction::Restart
    } else if matches.get_flag("shutdown") {
        DiagnosticsAction::Shutdown
    } else if matches.get_flag("sleep") {
        DiagnosticsAction::Sleep
    } else {
        // Default to All if no action specified
        DiagnosticsAction::All
    };

    // Perform the action
    match action {
        DiagnosticsAction::Restart => {
            println!("Restarting device...");
            match diagnostics_client.restart().await {
                Ok(_) => println!("Device restart initiated"),
                Err(e) => eprintln!("Failed to restart device: {e:?}"),
            }
        }
        DiagnosticsAction::Shutdown => {
            println!("Shutting down device...");
            match diagnostics_client.shutdown().await {
                Ok(_) => println!("Device shutdown initiated"),
                Err(e) => eprintln!("Failed to shutdown device: {e:?}"),
            }
        }
        DiagnosticsAction::Sleep => {
            println!("Putting device to sleep...");
            match diagnostics_client.sleep().await {
                Ok(_) => println!("Device sleep initiated"),
                Err(e) => eprintln!("Failed to put device to sleep: {e:?}"),
            }
        }
        _ => {
            // Request diagnostics
            println!("Requesting diagnostics...");
            match diagnostics_client.request_diagnostics(action).await {
                Ok(data) => {
                    // Convert to pretty XML
                    let xml = plist::to_format_xml(&data).unwrap_or_else(|_| "Failed to format XML".to_string());
                    
                    // Output the data
                    if let Some(path) = output_path {
                        // Save to file
                        match File::create(path) {
                            Ok(mut file) => {
                                match file.write_all(xml.as_bytes()) {
                                    Ok(_) => println!("Diagnostics saved to: {}", path),
                                    Err(e) => eprintln!("Failed to write data to file: {}", e),
                                }
                            }
                            Err(e) => eprintln!("Failed to create output file: {}", e),
                        }
                    } else {
                        // Print to console
                        println!("{}", xml);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to request diagnostics: {e:?}");
                }
            }
        }
    }
}