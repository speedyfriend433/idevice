// Jackson Coxson
// idevice Rust implementation of AFC file operations

use clap::{Arg, Command};
use idevice::{afc::AfcClient, IdeviceService};

mod common;

#[tokio::main]
async fn main() {
    env_logger::init();

    let matches = Command::new("afc_tool")
        .about("Interact with iOS device filesystem")
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
            Arg::new("list")
                .long("list")
                .short('l')
                .value_name("PATH")
                .help("List directory contents"),
        )
        .arg(
            Arg::new("info")
                .long("info")
                .short('i')
                .value_name("PATH")
                .help("Get file/directory info"),
        )
        .arg(
            Arg::new("mkdir")
                .long("mkdir")
                .value_name("PATH")
                .help("Create directory"),
        )
        .arg(
            Arg::new("remove")
                .long("remove")
                .short('r')
                .value_name("PATH")
                .help("Remove file or directory"),
        )
        .arg(
            Arg::new("device-info")
                .long("device-info")
                .short('d')
                .help("Get device info")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    if matches.get_flag("about") {
        println!("afc_tool - interact with iOS device filesystem. Reimplementation of libimobiledevice's functionality.");
        println!("Copyright (c) 2025 Jackson Coxson");
        return;
    }

    let udid = matches.get_one::<String>("udid");
    let host = matches.get_one::<String>("host");
    let pairing_file = matches.get_one::<String>("pairing_file");

    let provider =
        match common::get_provider(udid, host, pairing_file, "afc-tool-jkcoxson").await {
            Ok(p) => p,
            Err(e) => {
                eprintln!("{e}");
                return;
            }
        };

    let mut afc_client = match AfcClient::connect(&*provider).await {
        Ok(client) => client,
        Err(e) => {
            eprintln!("Failed to connect to AFC service: {e:?}");
            return;
        }
    };

    if matches.get_flag("device-info") {
        match afc_client.get_device_info().await {
            Ok(info) => {
                println!("Device Info:");
                for (key, value) in info {
                    println!("  {}: {}", key, value);
                }
            }
            Err(e) => {
                eprintln!("Failed to get device info: {e:?}");
            }
        }
    }

    if let Some(path) = matches.get_one::<String>("list") {
        match afc_client.read_directory(path).await {
            Ok(entries) => {
                println!("Directory contents of '{}':", path);
                for entry in entries {
                    println!("  {}", entry);
                }
            }
            Err(e) => {
                eprintln!("Failed to list directory: {e:?}");
            }
        }
    }

    if let Some(path) = matches.get_one::<String>("info") {
        match afc_client.get_file_info(path).await {
            Ok(info) => {
                println!("Info for '{}':", path);
                for (key, value) in info {
                    println!("  {}: {}", key, value);
                }
            }
            Err(e) => {
                eprintln!("Failed to get file info: {e:?}");
            }
        }
    }

    if let Some(path) = matches.get_one::<String>("mkdir") {
        match afc_client.make_directory(path).await {
            Ok(_) => {
                println!("Directory '{}' created successfully", path);
            }
            Err(e) => {
                eprintln!("Failed to create directory: {e:?}");
            }
        }
    }

    if let Some(path) = matches.get_one::<String>("remove") {
        match afc_client.remove_path(path).await {
            Ok(_) => {
                println!("Path '{}' removed successfully", path);
            }
            Err(e) => {
                eprintln!("Failed to remove path: {e:?}");
            }
        }
    }
}